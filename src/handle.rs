use std::ffi::CStr;

use crate::Context;

/// A fully configured WAF instance.
///
/// This is obtained by [`Builder::build`][crate::Builder::build] and provides facility to create new [`Context`]
/// that use the underlying instance's configuration.
#[repr(transparent)]
pub struct Handle {
    pub(crate) raw: crate::bindings::ddwaf_handle,
}
impl Handle {
    /// Creates a new [Context] from this instance.
    #[must_use]
    pub fn new_context(&self) -> Context {
        Context {
            raw: unsafe { crate::bindings::ddwaf_context_init(self.raw) },
            keepalive: Vec::new(),
        }
    }

    /// Returns the list of actions that may be produced by this instance's ruleset.
    pub fn known_actions(&self) -> Vec<&CStr> {
        self.call_cstr_array_fn(crate::bindings::ddwaf_known_actions)
    }

    /// Returns the list of addresses that are used by this instance's ruleset.
    ///
    /// Sending data for addresses not in this list to [`Context::run`] should be avoided as this
    /// data will never result in any side-effects.
    pub fn known_addresses(&self) -> Vec<&CStr> {
        self.call_cstr_array_fn(crate::bindings::ddwaf_known_addresses)
    }

    fn call_cstr_array_fn(
        &self,
        f: unsafe extern "C" fn(
            crate::bindings::ddwaf_handle,
            *mut u32,
        ) -> *const *const std::os::raw::c_char,
    ) -> Vec<&CStr> {
        let mut size = std::mem::MaybeUninit::<u32>::uninit();
        let ptr = unsafe { f(self.raw, size.as_mut_ptr()) };
        if ptr.is_null() {
            return vec![];
        }
        let size = unsafe { size.assume_init() as usize };
        let arr = unsafe { std::slice::from_raw_parts(ptr, size) };
        arr.iter().map(|&x| unsafe { CStr::from_ptr(x) }).collect()
    }
}
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { crate::bindings::ddwaf_destroy(self.raw) }
    }
}
// SAFETY: ddwaf instances are effectively immutable
unsafe impl Send for Handle {}
// SAFETY: ddwaf instances are effectively immutable
unsafe impl Sync for Handle {}
