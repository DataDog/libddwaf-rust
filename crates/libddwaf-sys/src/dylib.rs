#![allow(clippy::missing_safety_doc)]

use std::io::{Write, copy};

use crate::*;

use lazy_static::lazy_static;

const LIBDDWAF_SHARED_OBJECT: &[u8] = include_bytes!(env!("LIBDDWAF_SHARED_OBJECT.zst"));

lazy_static! {
    static ref LIBRARY: ddwaf = init().unwrap_or_default();
}

/// Initialize the global shared library instance.
///
/// Dumps the shared object blob to a temporary file, then proceeds to load it
/// with the [ddwaf::new].
fn init() -> Option<ddwaf> {
    tracing::debug!("dumping embedded libddwaf shared object to a temporary file...");
    let mut tmp = match tempfile::NamedTempFile::new() {
        Ok(tmp) => tmp,
        Err(e) => {
            tracing::error!("failed to create temporary file: {e}");
            return None;
        }
    };

    let mut decoder = match zstd::Decoder::new(LIBDDWAF_SHARED_OBJECT) {
        Ok(dec) => dec,
        Err(e) => {
            tracing::error!("failed to create zstd decoder: {e}");
            return None;
        }
    };

    if let Err(e) = copy(&mut decoder, &mut tmp) {
        eprintln!("failed to write libddwaf shared object to temporay file: {e}");
        tracing::error!("failed to write libddwaf shared object to temporay file: {e}");
        return None;
    }
    if let Err(e) = tmp.flush() {
        tracing::error!("failed to flush libddwaf shared object to temporay file: {e}");
        return None;
    }

    tracing::debug!(
        "loading libddwaf shared object from temporary file {tmp}",
        tmp = tmp.path().display()
    );
    match unsafe { ddwaf::new(tmp.path()) } {
        Ok(lib) => Some(lib),
        Err(e) => {
            tracing::error!("failed to load libddwaf shared object: {e}");
            None
        }
    }
}

/// Re-exports a function from the static [`ddwaf``] instance, so that the API
/// remains consistent with the one when the `dynamic` feature is not enabled.
/// All exported functions will be tagged as `extern "C"`, and the body provided
/// in the macro corresponds to the default value to return when the shared
/// library could not be loaded.
macro_rules! reexport {
    (
        $($vis:vis unsafe fn $name:ident($($arg_name:ident: $arg_type: ty),*) $(-> $ret_type:ty)? { $($fallback:expr)? })*
    ) => {
        $(
            $vis unsafe extern "C" fn $name($($arg_name: $arg_type),*) $(-> $ret_type)? {
                unsafe { LIBRARY.$name($($arg_name),*) }
            }
        )*

        impl Default for ddwaf {
            #[cold]
            fn default() -> Self {
                $(
                    #[cold]
                    unsafe extern "C" fn $name($($arg_name: $arg_type),*) $(-> $ret_type)? {$($fallback)?}
                )*

                Self {
                    __library: unsafe { libloading::os::unix::Library::from_raw(std::ptr::null_mut()) }.into(),
                    $($name),*
                }
            }
        }
    };
}

// Please keep this list alphanumerically sorted for convenience.
reexport! {
    pub unsafe fn ddwaf_builder_add_or_update_config(builder: ddwaf_builder, path: *const std::os::raw::c_char, path_len: u32, config: *const ddwaf_object, diagnostics: *mut ddwaf_object) -> bool { false }
    pub unsafe fn ddwaf_builder_build_instance(builder: ddwaf_builder) -> ddwaf_handle { std::ptr::null_mut() }
    pub unsafe fn ddwaf_builder_destroy(builder: ddwaf_builder) {}
    pub unsafe fn ddwaf_builder_get_config_paths(builder: ddwaf_builder, paths: *mut ddwaf_object, filter: *const ::std::os::raw::c_char, filter_len: u32) -> u32 { 0 }
    pub unsafe fn ddwaf_builder_init(config: *const ddwaf_config) -> ddwaf_builder { std::ptr::null_mut() }
    pub unsafe fn ddwaf_builder_remove_config(builder: ddwaf_builder, path: *const std::os::raw::c_char, path_len: u32) -> bool { false }
    pub unsafe fn ddwaf_context_destroy(context: ddwaf_context) {}
    pub unsafe fn ddwaf_context_init(handle: ddwaf_handle) -> ddwaf_context { std::ptr::null_mut() }
    pub unsafe fn ddwaf_destroy(handle: ddwaf_handle) {}
    pub unsafe fn ddwaf_get_version() -> *const std::os::raw::c_char { std::ptr::null() }
    pub unsafe fn ddwaf_known_actions(handle: ddwaf_handle, size: *mut u32) -> *const *const ::std::os::raw::c_char { std::ptr::null() }
    pub unsafe fn ddwaf_known_addresses(handle: ddwaf_handle, size: *mut u32) -> *const *const ::std::os::raw::c_char { std::ptr::null() }
    pub unsafe fn ddwaf_object_array(object: *mut ddwaf_object) -> *mut ddwaf_object { std::ptr::null_mut() }
    pub unsafe fn ddwaf_object_bool(object: *mut ddwaf_object, value: bool) -> *mut ddwaf_object { std::ptr::null_mut() }
    pub unsafe fn ddwaf_object_float(object: *mut ddwaf_object, value: f64) -> *mut ddwaf_object { std::ptr::null_mut() }
    pub unsafe fn ddwaf_object_free(object: *mut ddwaf_object) {}
    pub unsafe fn ddwaf_object_from_json(output: *mut ddwaf_object, json_str: *const std::os::raw::c_char, length: u32) -> bool { false }
    pub unsafe fn ddwaf_object_invalid(object: *mut ddwaf_object) -> *mut ddwaf_object { std::ptr::null_mut() }
    pub unsafe fn ddwaf_object_map(object: *mut ddwaf_object) -> *mut ddwaf_object { std::ptr::null_mut() }
    pub unsafe fn ddwaf_object_null(object: *mut ddwaf_object) -> *mut ddwaf_object { std::ptr::null_mut() }
    pub unsafe fn ddwaf_object_signed(object: *mut ddwaf_object, value: i64) -> *mut ddwaf_object { std::ptr::null_mut() }
    pub unsafe fn ddwaf_object_stringl(object: *mut ddwaf_object, string: *const std::os::raw::c_char, length: usize) -> *mut ddwaf_object { std::ptr::null_mut() }
    pub unsafe fn ddwaf_object_unsigned(object: *mut ddwaf_object, value: u64) -> *mut ddwaf_object { std::ptr::null_mut() }
    pub unsafe fn ddwaf_run(context: ddwaf_context, persistent_data: *mut ddwaf_object, ephemeral_data: *mut ddwaf_object, result: *mut ddwaf_object, timeout: u64) -> DDWAF_RET_CODE { DDWAF_ERR_INTERNAL }
    pub unsafe fn ddwaf_set_log_cb(cb: ddwaf_log_cb, min_level: DDWAF_LOG_LEVEL) -> bool { false }
}
