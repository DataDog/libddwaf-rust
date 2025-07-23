use std::ptr::null_mut;

use crate::object::{AsRawMutObject, WafArray, WafMap, WafOwned};
use crate::{Config, Handle};

/// A builder for [Handle]s.
///
/// This is used to maintain a live view over mutable configuration, and is best
/// suited for cases where the Waf's configuration evolves regularly, such as
/// through remote configuration.
#[repr(transparent)]
pub struct Builder {
    raw: libddwaf_sys::ddwaf_builder,
}
impl Builder {
    /// Creates a new [Builder] instance using the provided [Config]. Returns [None] if the
    /// builder's initialization fails.
    #[must_use]
    pub fn new(config: &Config) -> Option<Self> {
        let builder = Builder {
            raw: unsafe { libddwaf_sys::ddwaf_builder_init(&raw const config.raw) },
        };
        if builder.raw.is_null() {
            return None;
        }
        Some(builder)
    }

    /// Adds or updates the configuration for the given path.
    ///
    /// Returns true if the ruleset was successfully added or updated. Any warning/error information
    /// is conveyed through the provided diagnostics object.
    ///
    /// # Panics
    /// Panics if the provided `path` is longer than [`u32::MAX`] bytes.
    #[must_use]
    pub fn add_or_update_config(
        &mut self,
        path: &str,
        ruleset: &impl AsRef<libddwaf_sys::ddwaf_object>,
        diagnostics: Option<&mut WafOwned<WafMap>>,
    ) -> bool {
        debug_assert!(
            !path.is_empty(),
            concat!(
                "path cannot be empty (",
                stringify!(bindings::ddwaf_builder_add_or_update_config),
                " would always fail)"
            )
        );
        let path_len = u32::try_from(path.len()).expect("path is too long");
        unsafe {
            libddwaf_sys::ddwaf_builder_add_or_update_config(
                self.raw,
                path.as_ptr().cast(),
                path_len,
                ruleset.as_ref(),
                diagnostics.map_or(null_mut(), |o| std::ptr::from_mut(o.as_raw_mut()).cast()),
            )
        }
    }

    /// Removes the configuration for the given path if some exists.
    ///
    /// Returns true if some configuration was indeed removed.
    ///
    /// # Panics
    /// Panics if the provided `path` is longer than [`u32::MAX`] bytes.
    pub fn remove_config(&mut self, path: &str) -> bool {
        let path_len = u32::try_from(path.len()).expect("path is too long");
        unsafe {
            libddwaf_sys::ddwaf_builder_remove_config(self.raw, path.as_ptr().cast(), path_len)
        }
    }

    /// Returns the number of configuration paths currently loaded in this [Builder], optionally
    /// filtered by a regular expression.
    ///
    /// # Panics
    /// Panics if the provided `filter` regular expression is longer than [`u32::MAX`] bytes.
    #[must_use]
    pub fn config_paths_count(&mut self, filter: Option<&'_ str>) -> u32 {
        let filter = filter.unwrap_or("");
        let filter_len = u32::try_from(filter.len()).expect("filter is too long");
        unsafe {
            libddwaf_sys::ddwaf_builder_get_config_paths(
                self.raw,
                null_mut(),
                filter.as_ptr().cast(),
                filter_len,
            )
        }
    }

    /// Returns the configuration paths currently loaded in this [Builder], optionally filtered by
    /// a regular expression.
    ///
    /// # Panics
    /// Panics if the provided `filter` regular expression is longer than [`u32::MAX`] bytes.
    #[must_use]
    pub fn config_paths(&mut self, filter: Option<&'_ str>) -> WafOwned<WafArray> {
        let mut res = WafOwned::<WafArray>::default();
        let filter = filter.unwrap_or("");
        let filter_len = u32::try_from(filter.len()).expect("filter is too long");
        let _ = unsafe {
            libddwaf_sys::ddwaf_builder_get_config_paths(
                self.raw,
                res.as_raw_mut(),
                filter.as_ptr().cast(),
                filter_len,
            )
        };
        res
    }

    /// Builds a new [Handle] from the current configuration in this [Builder].
    ///
    /// Returns [None] if the builder fails to create a new [Handle], meaning the current
    /// configuration contains no active instructions (no rules nor processors are available).
    #[must_use]
    pub fn build(&mut self) -> Option<Handle> {
        let raw = unsafe { libddwaf_sys::ddwaf_builder_build_instance(self.raw) };
        if raw.is_null() {
            return None;
        }
        Some(Handle { raw })
    }
}
impl Drop for Builder {
    fn drop(&mut self) {
        unsafe { libddwaf_sys::ddwaf_builder_destroy(self.raw) }
    }
}

// SAFETY: no thread-local data and no data can be changed under us if we have an owning handle
unsafe impl Send for Builder {}
// SAFETY: changes are only made through exclusive references
unsafe impl Sync for Builder {}
