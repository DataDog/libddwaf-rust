use std::error;
use std::fmt;
use std::ptr::null_mut;
use std::time::Duration;

use crate::object::{AsRawMutObject, Keyed, WafArray, WafMap, WafObject, WafOwned};

/// A WAF Context that can be used to evaluate the configured ruleset against address data.
///
/// This is obtained by calling [`Handle::new_context`][crate::Handle::new_context], and a given [`Context`] should only
/// be used to handle data for a single request.
pub struct Context {
    pub(crate) raw: libddwaf_sys::ddwaf_context,
    pub(crate) keepalive: Vec<WafMap>,
}
impl Context {
    /// Evaluates the configured ruleset against the provided address data, and returns the result
    /// of this evaluation.
    ///
    /// # Errors
    /// Returns an error if the WAF encountered an internal error, invalid object, or invalid argument while processing
    /// the request.
    pub fn run(
        &mut self,
        mut persistent_data: Option<WafMap>,
        ephemeral_data: Option<&WafMap>,
        timeout: Duration,
    ) -> Result<RunResult, RunError> {
        let mut res = std::mem::MaybeUninit::<RunOutput>::uninit();
        let persistent_ref = persistent_data
            .as_mut()
            // The bindings take non-const pointers to the data, but actually does not change it.
            .map_or(null_mut(), |f| unsafe {
                std::ptr::from_mut(f.as_raw_mut()).cast()
            });
        let ephemeral_ref = ephemeral_data
            .map(AsRef::<libddwaf_sys::ddwaf_object>::as_ref)
            // The bindings take non-const pointers to the data, but actually does not change it.
            .map_or(null_mut(), |r| std::ptr::from_ref(r).cast_mut());

        let status = unsafe {
            libddwaf_sys::ddwaf_run(
                self.raw,
                persistent_ref,
                ephemeral_ref,
                res.as_mut_ptr().cast(),
                timeout.as_micros().try_into().unwrap_or(u64::MAX),
            )
        };
        match status {
            libddwaf_sys::DDWAF_ERR_INTERNAL => {
                // It's unclear whether the persistent data needs to be kept alive or not, so we
                // keep it alive to be on the safe side.
                if let Some(obj) = persistent_data {
                    self.keepalive.push(obj);
                }
                Err(RunError::InternalError)
            }
            libddwaf_sys::DDWAF_ERR_INVALID_OBJECT => Err(RunError::InvalidObject),
            libddwaf_sys::DDWAF_ERR_INVALID_ARGUMENT => Err(RunError::InvalidArgument),
            libddwaf_sys::DDWAF_OK => {
                // We need to keep the persistent data alive as the WAF may hold references to it.
                if let Some(obj) = persistent_data {
                    self.keepalive.push(obj);
                }
                Ok(RunResult::NoMatch(unsafe { res.assume_init() }))
            }
            libddwaf_sys::DDWAF_MATCH => {
                // We need to keep the persistent data alive as the WAF may hold references to it.
                if let Some(obj) = persistent_data {
                    self.keepalive.push(obj);
                }
                Ok(RunResult::Match(unsafe { res.assume_init() }))
            }
            unknown => unreachable!(
                "Unexpected value returned by {}: 0x{:02X}",
                stringify!(libddwaf_sys::ddwaf_run),
                unknown
            ),
        }
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        unsafe { libddwaf_sys::ddwaf_context_destroy(self.raw) }
    }
}
// Safety: Operations that mutate the internal state are made safe by requiring a mutable borrow on
// the [Context] instance; and none of the internal state is exposed in any way.
unsafe impl Send for Context {}
// Safety: [Context] is trivially [Sync] because it contains no methods allowing shared references.
unsafe impl Sync for Context {}

/// The result of the [`Context::run`] operation.
#[derive(Debug)]
pub enum RunResult {
    /// The WAF successfully processed the request, and produced no match.
    NoMatch(RunOutput),
    /// The WAF successfully processed the request and some event rules matched
    /// some of the supplied address data.
    Match(RunOutput),
}

/// The error that can occur during a [`Context::run`] operation.
#[non_exhaustive]
#[derive(Debug)]
pub enum RunError {
    /// The WAF encountered an internal error while processing the request.
    InternalError,
    /// The WAF encountered an invalid object while processing the request.
    InvalidObject,
    /// The WAF encountered an invalid argument while processing the request.
    InvalidArgument,
}
impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::InternalError => write!(f, "The WAF encountered an internal error"),
            RunError::InvalidObject => write!(f, "The WAF encountered an invalid object"),
            RunError::InvalidArgument => write!(f, "The WAF encountered an invalid argument"),
        }
    }
}
impl error::Error for RunError {}

/// The data produced by a [`Context::run`] operation.
#[repr(transparent)]
pub struct RunOutput {
    data: WafOwned<WafMap>,
}
impl RunOutput {
    /// Returns true if the WAF did not have enough time to process all the address data that was
    /// being evaluated.
    #[must_use]
    pub fn timeout(&self) -> bool {
        debug_assert!(self.data.is_valid());
        self.data
            .get(b"timeout")
            .and_then(|o| o.to_bool())
            .unwrap_or_default()
    }

    /// Returns true if the WAF determined the trace for this request should have its priority
    /// overridden to ensure it is not dropped by the sampler.
    #[must_use]
    pub fn keep(&self) -> bool {
        debug_assert!(self.data.is_valid());
        self.data
            .get(b"keep")
            .and_then(|o| o.to_bool())
            .unwrap_or_default()
    }

    /// Returns the total time spent processing the request; excluding bindings overhead (which
    /// ought to be trivial).
    pub fn duration(&self) -> Duration {
        debug_assert!(self.data.is_valid());
        self.data
            .get(b"duration")
            .and_then(|o| o.to_u64())
            .map(Duration::from_nanos)
            .unwrap_or_default()
    }

    /// Returns the list of events that were produced by this WAF run.
    ///
    /// This is only expected to be populated when [`Context::run`] returns [`RunResult::Match`].
    pub fn events(&self) -> Option<&Keyed<WafArray>> {
        debug_assert!(self.data.is_valid());
        self.data
            .get(b"events")
            .and_then(Keyed::<WafObject>::as_type)
    }

    /// Returns the list of actions that were produced by this WAF run.
    ///
    /// This is only expected to be populated when [`Context::run`] returns [`RunResult::Match`].
    pub fn actions(&self) -> Option<&Keyed<WafMap>> {
        debug_assert!(self.data.is_valid());
        self.data
            .get(b"actions")
            .and_then(Keyed::<WafObject>::as_type)
    }

    /// Returns the list of attributes that were produced by this WAF run, and which should be
    /// attached to the surrounding trace.
    pub fn attributes(&self) -> Option<&Keyed<WafMap>> {
        debug_assert!(self.data.is_valid());
        self.data
            .get(b"attributes")
            .and_then(Keyed::<WafObject>::as_type)
    }
}
impl fmt::Debug for RunOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RunOutput")
            .field("timeout", &self.timeout())
            .field("keep", &self.keep())
            .field("duration", &self.duration())
            .field("events", &self.events())
            .field("actions", &self.actions())
            .field("attributes", &self.attributes())
            .finish()
    }
}
