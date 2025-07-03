use std::ptr::null_mut;

use crate::object::{AsRawMutObject, WAFArray, WAFMap, WAFOwned};
use crate::{Config, Handle};

/// A builder for [Handle]s.
///
/// This is used to maintain a live view over mutable configuration, and is best
/// suited for cases where the WAF's configuration evolves regularly, such as
/// through remote configuration.
#[repr(transparent)]
pub struct Builder {
    raw: crate::bindings::ddwaf_builder,
}
impl Builder {
    /// Creates a new [Builder] instance using the provided [Config]. Returns [None] if the
    /// builder's initialization fails.
    #[must_use]
    pub fn new(config: &Config) -> Option<Self> {
        let builder = Builder {
            raw: unsafe { crate::bindings::ddwaf_builder_init(&raw const config.raw) },
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
        ruleset: &impl AsRef<crate::bindings::ddwaf_object>,
        diagnostics: Option<&mut WAFOwned<WAFMap>>,
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
            crate::bindings::ddwaf_builder_add_or_update_config(
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
            crate::bindings::ddwaf_builder_remove_config(self.raw, path.as_ptr().cast(), path_len)
        }
    }

    /// Returns the number of configuration paths currently loaded in this [Builder], optionally
    /// filtered by a regular expression.
    ///
    /// # Panics
    /// Panics if the provided `filter` regular expression is longer than [`u32::MAX`] bytes.
    #[must_use]
    pub fn config_paths_count(&self, filter: Option<&'_ str>) -> u32 {
        let filter = filter.unwrap_or("");
        let filter_len = u32::try_from(filter.len()).expect("filter is too long");
        unsafe {
            crate::bindings::ddwaf_builder_get_config_paths(
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
    pub fn config_paths(&self, filter: Option<&'_ str>) -> WAFOwned<WAFArray> {
        let mut res = WAFOwned::<WAFArray>::default();
        let filter = filter.unwrap_or("");
        let filter_len = u32::try_from(filter.len()).expect("filter is too long");
        let _ = unsafe {
            crate::bindings::ddwaf_builder_get_config_paths(
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
    pub fn build(&self) -> Option<Handle> {
        let raw = unsafe { crate::bindings::ddwaf_builder_build_instance(self.raw) };
        if raw.is_null() {
            return None;
        }
        Some(Handle { raw })
    }
}
impl Drop for Builder {
    fn drop(&mut self) {
        unsafe { crate::bindings::ddwaf_builder_destroy(self.raw) }
    }
}

// SAFETY: no thread-local data and no data can be changed under us if we have an owning handle
unsafe impl Send for Builder {}
// SAFETY: changes are only made through exclusive references
unsafe impl Sync for Builder {}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use crate::object::WAFMap;
    use crate::{waf_array, waf_map};

    use super::*;

    #[test]
    pub fn blank_config() {
        let builder = Builder::new(&Config::default()).expect("builder should be created");
        // Not adding any rules, so we can't get a handle...
        assert!(builder.build().is_none());
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        should_panic(
            expected = "path cannot be empty (bindings::ddwaf_builder_add_or_update_config would always fail)"
        )
    )]
    pub fn empty_path() {
        let mut builder = Builder::new(&Config::default()).expect("builder should be created");
        assert!(!builder.add_or_update_config("", &waf_map! {}, None)); // Panics when debug_assertions is enabled
    }

    fn log_cb(
        level: crate::log::LogLevel,
        file: &std::ffi::CStr,
        func: &std::ffi::CStr,
        line: u32,
        msg: &[u8],
    ) {
        let msg = String::from_utf8_lossy(msg);
        eprintln!("[{level:>5}] {file:?}:{line}({func:?}) {msg}");
    }

    #[test]
    pub fn add_update_remove_config() {
        unsafe { crate::log::set_log_cb(Some(log_cb), crate::log::LogLevel::Debug) };

        let mut builder = Builder::new(&Config::default()).expect("builder should be created");

        let rules_1 = waf_map! {
            ("version", "2.1"),
            ("metadata", waf_map!{
                ("rules_version", "1"),
            }),
            ("rules", waf_array![
                waf_map!{
                    ("id", "1"),
                    ("name", "rule 1"),
                    ("tags", waf_map!{ ("type", "flow1"), ("category", "test") }),
                    ("conditions", waf_array![
                        waf_map!{
                            ("operator", "match_regex"),
                            ("parameters", waf_map!{
                                ("inputs", waf_array![
                                    waf_map!{("address", "address.1")},
                                ]),
                                ("regex", ".*"),
                            }),
                        },
                    ]),
                    ("on_match", waf_array!["block"]),
                },
            ]),
        };
        let rules_2 = waf_map! {
            ("version", "2.1"),
            ("metadata", waf_map!{
                ("rules_version", "2"),
            }),
            ("rules", waf_array![
                waf_map!{
                    ("id", "1"),
                    ("name", "rule 1"),
                    ("tags", waf_map!{ ("type", "flow1"), ("category", "test") }),
                    ("conditions", waf_array![
                        waf_map!{
                            ("operator", "match_regex"),
                            ("parameters", waf_map!{
                                ("inputs", waf_array![
                                    waf_map!{("address", "address.2")},
                                ]),
                                ("regex", ".*"),
                            }),
                        },
                    ]),
                    ("on_match", waf_array!["block"]),
                },
            ]),
        };

        assert_eq!(builder.config_paths_count(None), 0);

        let mut diagnostics = WAFOwned::<WAFMap>::default();
        assert!(builder.add_or_update_config("test", &rules_1, Some(&mut diagnostics)));
        assert!(diagnostics.is_valid());
        assert_eq!(
            diagnostics.get(b"ruleset_version").and_then(|o| o.to_str()),
            Some("1"),
        );
        assert_eq!(builder.config_paths_count(None), 1);
        for path in builder.config_paths(None).iter() {
            assert_eq!(path.to_str(), Some("test"));
        }

        assert!(builder.add_or_update_config("test", &rules_2, Some(&mut diagnostics)));
        assert!(diagnostics.is_valid());
        assert_eq!(
            diagnostics.get(b"ruleset_version").and_then(|o| o.to_str()),
            Some("2"),
        );
        assert_eq!(builder.config_paths_count(None), 1);
        for path in builder.config_paths(None).iter() {
            assert_eq!(path.to_str(), Some("test"));
        }

        assert!(builder.remove_config("test"));
        assert_eq!(builder.config_paths_count(None), 0);
        assert!(builder.config_paths(None).is_empty());
    }
}
