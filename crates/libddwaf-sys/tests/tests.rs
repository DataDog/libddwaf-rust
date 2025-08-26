use std::ffi::{CStr, CString};

use libddwaf_sys::*;

#[test]
fn test_version() {
    let version = unsafe { ddwaf_get_version() };
    assert!(!version.is_null());
    assert_eq!(
        CString::new(env!("CARGO_PKG_VERSION"))
            .expect("failed to create CString")
            .as_c_str(),
        unsafe { CStr::from_ptr(version) }
    );
}

#[test]
fn test_eq_invalid() {
    let left = ddwaf_object::default();
    let right = ddwaf_object::default();
    assert_eq!(left, right); // We always consider invalid objects to be equal.
}

#[test]
fn test_eq_null() {
    let mut left = ddwaf_object::default();
    unsafe { ddwaf_object_null(&mut left) };

    let right = ddwaf_object {
        type_: DDWAF_OBJ_NULL,
        ..ddwaf_object::default()
    };
    assert_eq!(left, right);
    assert_ne!(left, ddwaf_object::default());
}

#[test]
fn test_eq_signed() {
    let mut left = ddwaf_object::default();
    unsafe { ddwaf_object_signed(&mut left, -42) };
    assert_eq!(
        left,
        ddwaf_object {
            type_: DDWAF_OBJ_SIGNED,
            __bindgen_anon_1: _ddwaf_object__bindgen_ty_1 { intValue: -42 },
            ..ddwaf_object::default()
        }
    );

    assert_ne!(
        left,
        ddwaf_object {
            type_: DDWAF_OBJ_SIGNED,
            __bindgen_anon_1: _ddwaf_object__bindgen_ty_1 { intValue: 42 }, // Sign mismatch
            ..ddwaf_object::default()
        }
    );
    assert_ne!(left, ddwaf_object::default());
}

#[test]
fn test_eq_unsigned() {
    let mut left = ddwaf_object::default();
    unsafe { ddwaf_object_unsigned(&mut left, 1337) };

    assert_eq!(
        left,
        ddwaf_object {
            type_: DDWAF_OBJ_UNSIGNED,
            __bindgen_anon_1: _ddwaf_object__bindgen_ty_1 { uintValue: 1337 },
            ..ddwaf_object::default()
        }
    );

    assert_ne!(
        left,
        ddwaf_object {
            type_: DDWAF_OBJ_UNSIGNED,
            __bindgen_anon_1: _ddwaf_object__bindgen_ty_1 { uintValue: 42 }, // Value mismatch
            ..ddwaf_object::default()
        }
    );
    assert_ne!(left, ddwaf_object::default());
}

#[test]
fn test_eq_bool() {
    let mut left = ddwaf_object::default();
    unsafe { ddwaf_object_bool(&mut left, true) };

    assert_eq!(
        left,
        ddwaf_object {
            type_: DDWAF_OBJ_BOOL,
            __bindgen_anon_1: _ddwaf_object__bindgen_ty_1 { boolean: true },
            ..ddwaf_object::default()
        }
    );

    assert_ne!(
        left,
        ddwaf_object {
            type_: DDWAF_OBJ_BOOL,
            __bindgen_anon_1: _ddwaf_object__bindgen_ty_1 { boolean: false }, // Value mismatch
            ..ddwaf_object::default()
        }
    );
    assert_ne!(left, ddwaf_object::default());
}

#[test]
fn test_eq_float() {
    let mut left = ddwaf_object::default();
    unsafe { ddwaf_object_float(&mut left, 1337.42) };

    assert_eq!(
        left,
        ddwaf_object {
            type_: DDWAF_OBJ_FLOAT,
            __bindgen_anon_1: _ddwaf_object__bindgen_ty_1 { f64_: 1337.42 },
            ..ddwaf_object::default()
        }
    );

    assert_ne!(
        left,
        ddwaf_object {
            type_: DDWAF_OBJ_FLOAT,
            __bindgen_anon_1: _ddwaf_object__bindgen_ty_1 { f64_: 1337.0 }, // Value mismatch
            ..ddwaf_object::default()
        }
    );
    assert_ne!(left, ddwaf_object::default());
}

#[test]
fn test_eq_string() {
    let mut left = ddwaf_object::default();
    let blank = CString::new("").expect("Failed to create blank CString");
    unsafe { ddwaf_object_stringl(&mut left, blank.as_ref().as_ptr().cast(), 0) };
    assert_eq!(
        left,
        ddwaf_object {
            type_: DDWAF_OBJ_STRING,
            ..ddwaf_object::default()
        }
    );

    let mut left = ddwaf_object::default();
    unsafe { ddwaf_object_stringl(&mut left, b"Hello, world!".as_ptr().cast(), 13) };

    let str = String::from("Hello, world!");
    assert_eq!(
        left,
        ddwaf_object {
            type_: DDWAF_OBJ_STRING,
            __bindgen_anon_1: _ddwaf_object__bindgen_ty_1 {
                stringValue: str.as_bytes().as_ptr().cast(),
            },
            nbEntries: str.len() as _,
            ..ddwaf_object::default()
        }
    );

    let str = String::from("Hello, world");
    assert_ne!(
        left,
        ddwaf_object {
            type_: DDWAF_OBJ_STRING,
            __bindgen_anon_1: _ddwaf_object__bindgen_ty_1 {
                stringValue: str.as_bytes().as_ptr().cast(),
            },
            nbEntries: str.len() as _, // Length mismatch
            ..ddwaf_object::default()
        }
    );
    assert_ne!(left, ddwaf_object::default());
}

#[test]
fn test_eq_array_and_map() {
    // NB -- Map is a superset of array, so we don't test arrays separately.

    assert_eq!(
        ddwaf_object {
            type_: DDWAF_OBJ_ARRAY,
            ..ddwaf_object::default()
        },
        ddwaf_object {
            type_: DDWAF_OBJ_ARRAY,
            ..ddwaf_object::default()
        }
    );

    let mut items = [ddwaf_object::default()];
    unsafe { ddwaf_object_unsigned(&mut items[0], 42) };
    items[0].parameterName = b"key".as_ptr().cast();
    items[0].parameterNameLength = 3;

    let left = ddwaf_object {
        type_: DDWAF_OBJ_MAP,
        nbEntries: 1,
        __bindgen_anon_1: _ddwaf_object__bindgen_ty_1 {
            array: items.as_mut_ptr().cast(),
        },
        ..ddwaf_object::default()
    };

    assert_eq!(
        left,
        ddwaf_object {
            type_: DDWAF_OBJ_MAP,
            nbEntries: 1,
            __bindgen_anon_1: _ddwaf_object__bindgen_ty_1 {
                array: items.as_mut_ptr().cast(),
            },
            ..ddwaf_object::default()
        }
    );

    let mut items = [ddwaf_object::default()];
    unsafe { ddwaf_object_unsigned(&mut items[0], 42) };
    items[0].parameterName = b"yek".as_ptr().cast();
    items[0].parameterNameLength = 3;
    assert_ne!(
        left,
        ddwaf_object {
            type_: DDWAF_OBJ_MAP,
            nbEntries: 1,
            __bindgen_anon_1: _ddwaf_object__bindgen_ty_1 {
                array: items.as_mut_ptr().cast(), // Key mismatch
            },
            ..ddwaf_object::default()
        }
    );

    let mut items = [ddwaf_object::default()];
    unsafe { ddwaf_object_signed(&mut items[0], -1337) };
    items[0].parameterName = b"key".as_ptr().cast();
    items[0].parameterNameLength = 3;
    assert_ne!(
        left,
        ddwaf_object {
            type_: DDWAF_OBJ_MAP,
            nbEntries: 1,
            __bindgen_anon_1: _ddwaf_object__bindgen_ty_1 {
                array: items.as_mut_ptr().cast(), // Value mismatch
            },
            ..ddwaf_object::default()
        }
    );

    assert_ne!(
        left,
        ddwaf_object {
            type_: DDWAF_OBJ_MAP,
            nbEntries: 0, // Length mismatch
            __bindgen_anon_1: _ddwaf_object__bindgen_ty_1 {
                array: items.as_mut_ptr().cast()
            },
            ..ddwaf_object::default()
        }
    );
    assert_ne!(left, ddwaf_object::default());
}
