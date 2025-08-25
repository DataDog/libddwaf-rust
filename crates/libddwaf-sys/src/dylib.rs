#![allow(clippy::missing_safety_doc)]

use std::io::Write;

use lazy_static::lazy_static;

#[cfg(target_os = "macos")]
const LIBDDWAF_SHARED_OBJECT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/download/lib/libddwaf.dylib"));
#[cfg(target_os = "linux")]
const LIBDDWAF_SHARED_OBJECT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/download/lib/libddwaf.so"));
#[cfg(target_os = "windows")]
const LIBDDWAF_SHARED_OBJECT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/download/lib/libddwaf.dll"));

lazy_static! {
    static ref LIBRARY: Option<crate::ddwaf> = init();
}

/// Initialize the global shared library instance.
///
/// Dumps the shared object blob to a temporary file, then proceeds to load it
/// with the [crate::ddwaf::new].
fn init() -> Option<crate::ddwaf> {
    tracing::debug!("dumping embedded libddwaf shared object to a temporary file...");
    let mut tmp = match tempfile::NamedTempFile::new() {
        Ok(tmp) => tmp,
        Err(e) => {
            tracing::error!("failed to create temporary file: {e}");
            return None;
        }
    };
    if let Err(e) = tmp.write_all(LIBDDWAF_SHARED_OBJECT) {
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
    match unsafe { crate::ddwaf::new(tmp.path()) } {
        Ok(lib) => Some(lib),
        Err(e) => {
            tracing::error!("failed to load libddwaf shared object: {e}");
            None
        }
    }
}

// Below are re-exports over the lazy static [LIBRARY] above, so we are
// API-compatible with the static version of the library. If the library failed
// loading, these behave as no-ops, or return error values when possible.

pub unsafe fn ddwaf_get_version() -> *const std::os::raw::c_char {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_get_version() }
    } else {
        std::ptr::null()
    }
}

#[cold]
pub unsafe fn ddwaf_set_log_cb(cb: crate::ddwaf_log_cb, min_level: crate::DDWAF_LOG_LEVEL) -> bool {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_set_log_cb(cb, min_level) }
    } else {
        false
    }
}

pub unsafe fn ddwaf_object_from_json(
    output: *mut crate::ddwaf_object,
    json_str: *const std::os::raw::c_char,
    length: u32,
) -> bool {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_object_from_json(output, json_str, length) }
    } else {
        false
    }
}

pub unsafe fn ddwaf_object_free(object: *mut crate::ddwaf_object) {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_object_free(object) }
    }
}

pub unsafe fn ddwaf_builder_init(config: *const crate::ddwaf_config) -> crate::ddwaf_builder {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_builder_init(config) }
    } else {
        std::ptr::null_mut()
    }
}

pub unsafe fn ddwaf_builder_add_or_update_config(
    builder: crate::ddwaf_builder,
    path: *const std::os::raw::c_char,
    path_len: u32,
    config: *const crate::ddwaf_object,
    diagnostics: *mut crate::ddwaf_object,
) -> bool {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe {
            library.ddwaf_builder_add_or_update_config(builder, path, path_len, config, diagnostics)
        }
    } else {
        false
    }
}

pub unsafe fn ddwaf_builder_remove_config(
    builder: crate::ddwaf_builder,
    path: *const std::os::raw::c_char,
    path_len: u32,
) -> bool {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_builder_remove_config(builder, path, path_len) }
    } else {
        false
    }
}

pub unsafe fn ddwaf_builder_get_config_paths(
    builder: crate::ddwaf_builder,
    paths: *mut crate::ddwaf_object,
    filter: *const ::std::os::raw::c_char,
    filter_len: u32,
) -> u32 {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_builder_get_config_paths(builder, paths, filter, filter_len) }
    } else {
        if !paths.is_null() {
            (*paths).type_ = crate::DDWAF_OBJ_ARRAY;
            (*paths).nbEntries = 0;
        }
        0
    }
}

pub unsafe fn ddwaf_builder_build_instance(builder: crate::ddwaf_builder) -> crate::ddwaf_handle {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_builder_build_instance(builder) }
    } else {
        std::ptr::null_mut()
    }
}

pub unsafe fn ddwaf_builder_destroy(builder: crate::ddwaf_builder) {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_builder_destroy(builder) }
    }
}

pub unsafe fn ddwaf_context_init(handle: crate::ddwaf_handle) -> crate::ddwaf_context {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_context_init(handle) }
    } else {
        std::ptr::null_mut()
    }
}

pub unsafe fn ddwaf_run(
    context: crate::ddwaf_context,
    persistent_data: *mut crate::ddwaf_object,
    ephemeral_data: *mut crate::ddwaf_object,
    result: *mut crate::ddwaf_object,
    timeout: u64,
) -> crate::DDWAF_RET_CODE {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_run(context, persistent_data, ephemeral_data, result, timeout) }
    } else {
        crate::DDWAF_ERR_INTERNAL
    }
}

pub unsafe fn ddwaf_context_destroy(context: crate::ddwaf_context) {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_context_destroy(context) }
    }
}

pub unsafe extern "C" fn ddwaf_known_actions(
    handle: crate::ddwaf_handle,
    size: *mut u32,
) -> *const *const ::std::os::raw::c_char {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_known_actions(handle, size) }
    } else {
        std::ptr::null()
    }
}

pub unsafe extern "C" fn ddwaf_known_addresses(
    handle: crate::ddwaf_handle,
    size: *mut u32,
) -> *const *const ::std::os::raw::c_char {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_known_addresses(handle, size) }
    } else {
        std::ptr::null()
    }
}

pub unsafe fn ddwaf_destroy(handle: crate::ddwaf_handle) {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_destroy(handle) }
    }
}

pub unsafe fn ddwaf_object_null(object: *mut crate::ddwaf_object) -> *mut crate::ddwaf_object {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_object_null(object) }
    } else {
        (*object).type_ = crate::DDWAF_OBJ_NULL;
        object
    }
}

pub unsafe fn ddwaf_object_signed(
    object: *mut crate::ddwaf_object,
    value: i64,
) -> *mut crate::ddwaf_object {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_object_signed(object, value) }
    } else {
        (*object).type_ = crate::DDWAF_OBJ_SIGNED;
        (*object).__bindgen_anon_1.intValue = value;
        object
    }
}

pub unsafe fn ddwaf_object_unsigned(
    object: *mut crate::ddwaf_object,
    value: u64,
) -> *mut crate::ddwaf_object {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_object_unsigned(object, value) }
    } else {
        (*object).type_ = crate::DDWAF_OBJ_UNSIGNED;
        (*object).__bindgen_anon_1.uintValue = value;
        object
    }
}

pub unsafe fn ddwaf_object_bool(
    object: *mut crate::ddwaf_object,
    value: bool,
) -> *mut crate::ddwaf_object {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_object_bool(object, value) }
    } else {
        (*object).type_ = crate::DDWAF_OBJ_BOOL;
        (*object).__bindgen_anon_1.boolean = value;
        object
    }
}

pub unsafe fn ddwaf_object_float(
    object: *mut crate::ddwaf_object,
    value: f64,
) -> *mut crate::ddwaf_object {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_object_float(object, value) }
    } else {
        (*object).type_ = crate::DDWAF_OBJ_FLOAT;
        (*object).__bindgen_anon_1.f64_ = value;
        object
    }
}

pub unsafe fn ddwaf_object_stringl(
    object: *mut crate::ddwaf_object,
    string: *const std::os::raw::c_char,
    length: usize,
) -> *mut crate::ddwaf_object {
    if let Some(library) = LIBRARY.as_ref() {
        unsafe { library.ddwaf_object_stringl(object, string, length) }
    } else {
        (*object).type_ = crate::DDWAF_OBJ_STRING;
        (*object).__bindgen_anon_1.stringValue = string;
        (*object).nbEntries = u64::try_from(length).unwrap_or(u64::MAX);
        object
    }
}
