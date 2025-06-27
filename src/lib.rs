#![warn(
    clippy::correctness,
    clippy::pedantic,
    clippy::perf,
    clippy::style,
    clippy::suspicious
)]
#![allow(clippy::used_underscore_binding, clippy::used_underscore_items)]

//! Rust bindings for the [`libddwaf` library](https://github.com/DataDog/libddwaf).
//!
//! # Basic Use
//!
//! The following high-level steps are typically used:
//! 1. Create a new [Builder]
//! 2. Add new configurations to it using [`Builder::add_or_update_config`]
//!     * Rulesets are often parsed from JSON documents using `serde_json`, via
//!       the `serde` feature.
//! 3. Call [`Builder::build`] to obtain a new [`Handle`]
//! 4. For any service request:
//!     1. Call [`Handle::new_context`] to obtain a new [`Context`]
//!     2. Call [`Context::run`] as appropriate with the necessary address data
//!
//! ```rust
//! use libddwaf::{
//!     object::*,
//!     waf_array,
//!     waf_map,
//!     Builder,
//!     RunResult,
//! };
//!
//! let mut builder = Builder::new(Default::default())
//!     .expect("Failed to build WAF instance");
//! let rule_set = waf_map!{
//!     /* Typically obtained by parsing a rules file using the serde feature */
//!     ("rules", waf_array!{ waf_map!{
//!         ("id", "1"),
//!         ("name", "rule 1"),
//!         ("tags", waf_map!{ ("type", "flow1"), ("category", "test") }),
//!         ("conditions", waf_array!{ waf_map!{
//!             ("operator", "match_regex"),
//!             ("parameters", waf_map!{
//!                 ("regex", ".*"),
//!                 ("inputs", waf_array!{ waf_map!{ ("address", "arg1" )} }),
//!             }),
//!         } }),
//!         ("on_match", waf_array!{ "block" })
//!     } }),
//! };
//! let mut diagnostics = WAFOwned::<WAFObject>::default();
//! if !builder.add_or_update_config("config/file/logical/path", &rule_set, Some(&mut diagnostics)) {
//!     panic!("Failed to add or update config!");
//! }
//! let waf = builder.build().expect("Failed to build WAF instance");
//!
//! // For each new request to be monitored...
//! let mut waf_ctx = waf.new_context();
//! let data = waf_map!{
//!     ("arg1", "value1"),
//! };
//! match waf_ctx.run(Some(data), None, std::time::Duration::from_millis(1)) {
//!     // Deal with the result as appropriate...
//!     Ok(RunResult::Match(res)) => {
//!         assert!(!res.timeout());
//!         assert!(res.keep());
//!         assert!(res.duration() >= std::time::Duration::default());
//!         assert_eq!(res.events().expect("Expected events").len(), 1);
//!         assert_eq!(res.actions().expect("Expected actions").len(), 1);
//!         assert_eq!(res.attributes().expect("Expected attributes").len(), 0);
//!     },
//!     Err(e) => panic!("Error while running the in-app WAF: {e}"),
//!     _ => panic!("Unexpected result"),
//! }
//! ```

use std::ffi::CStr;

#[cfg(feature = "serde")]
pub mod serde;

mod bindings;
pub mod log;
pub mod object;
mod private;

macro_rules! forward {
    ($($name:ident),*) => {
        $(
            mod $name;
            #[doc(inline = true)]
            pub use $name::*;
        )*
    };
}

forward!(builder, config, context, handle);

/// Returns the version of the underlying `libddwaf` library.
#[must_use]
pub fn get_version() -> &'static CStr {
    unsafe { CStr::from_ptr(bindings::ddwaf_get_version()) }
}

/// Helper macro to create [`object::WAFObject`]s.
#[macro_export]
macro_rules! waf_object {
    (null) => {
        $crate::object::WAFObject::from(())
    };
    ($l:expr) => {
        $crate::object::WAFObject::from($l)
    };
}

/// Helper macro to create [`object::WAFArray`]s.
#[macro_export]
macro_rules! waf_array {
    () => { $crate::object::WAFArray::new(0) };
    ($($e:expr),* $(,)?) => {
        {
            let size = [$($crate::__repl_expr_with_unit!($e)),*].len();
            let mut res = $crate::object::WAFArray::new(size as u64);
            let mut i = usize::MAX;
            $(
                i = i.wrapping_add(1);
                res[i] = $crate::waf_object!($e);
            )*
            res
        }
    };
}

/// Helper macro to create [`object::WAFMap`]s.
#[macro_export]
macro_rules! waf_map {
    () => { $crate::object::WAFMap::new(0) };
    ($(($k:literal, $v:expr)),* $(,)?) => {
        {
            let size = [$($crate::__repl_expr_with_unit!($v)),*].len();
            let mut res = $crate::object::WAFMap::new(size as u64);
            let mut i = usize::MAX;
            $(
                i = i.wrapping_add(1);
                let k: &str = $k.into();
                let obj = $crate::object::Keyed::<$crate::object::WAFObject>::from((k, $v));
                res[i] = obj.into();
            )*
            res
        }
    };
}

/// Helper macro to facilitate counting token trees within other macros.
///
/// Not intended for use outside of this crate, but must be exported as it is used by macros in this crate.
#[doc(hidden)]
#[macro_export]
macro_rules! __repl_expr_with_unit {
    ($e:expr) => {
        ()
    };
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::get_version;
    use crate::object::*;

    #[test]
    fn test_get_version() {
        assert_eq!(
            env!("CARGO_PKG_VERSION"),
            get_version()
                .to_str()
                .expect("Failed to convert version to str")
        );
    }

    #[test]
    fn sample_mixed_object() {
        let mut root = WAFArray::new(4);
        root[0] = 42_u64.into();
        root[1] = "Hello, world!".into();
        root[2] = WAFArray::new(1).into();
        root[2].as_type_mut::<WAFArray>().unwrap()[0] = 123_u64.into();

        let mut map = WAFMap::new(7);
        map[0] = ("key 1", "value 1").into();
        map[1] = ("key 2", -2_i64).into();
        map[2] = ("key 3", 2_u64).into();
        map[3] = ("key 4", 5.2).into();
        map[4] = ("key 5", ()).into();
        map[5] = ("key 6", true).into();
        root[3] = map.into();

        let res = format!("{root:?}");
        assert_eq!(
            res,
            "WAFArray[WAFUnsigned(42), WAFString(\"Hello, \
            world!\"), WAFArray[WAFUnsigned(123)], WAFMap{\
            \"key 1\"=WAFString(\"value 1\"), \"key 2\"=\
            WAFSigned(-2), \"key 3\"=WAFUnsigned(2), \
            \"key 4\"=WAFFloat(5.2), \"key 5\"=WAFNull, \
            \"key 6\"=WAFBool(true), \"\"=WAFInvalid}]"
        );
    }

    #[test]
    fn sample_mixed_object_macro() {
        let root = waf_array!(
            42_u64,
            "Hello, world!",
            waf_array!(123_u64),
            waf_map!(
                ("key 1", "value 1"),
                ("key 2", -2_i64),
                ("key 3", 2_u64),
                ("key 4", 5.2),
                ("key 5", waf_object!(null)),
                ("key 6", waf_array!()),
                ("key 7", waf_array!(true, false)),
            ),
            waf_array!(),
            waf_map!(),
        );

        assert_eq!(
            format!("{root:?}"),
            "WAFArray[WAFUnsigned(42), WAFString(\"Hello, \
            world!\"), WAFArray[WAFUnsigned(123)], WAFMap{\
            \"key 1\"=WAFString(\"value 1\"), \"key 2\"=\
            WAFSigned(-2), \"key 3\"=WAFUnsigned(2), \
            \"key 4\"=WAFFloat(5.2), \"key 5\"=WAFNull, \
            \"key 6\"=WAFArray[], \"key 7\"=WAFArray[WAFBool(true), \
            WAFBool(false)]}, WAFArray[], WAFMap{}]"
        );
    }

    #[test]
    #[allow(clippy::float_cmp)] // No operations are done on the values, they should be the same.
    fn ddwaf_obj_from_conversions() {
        let obj: WAFObject = 42u64.into();
        assert_eq!(obj.to_u64().unwrap(), 42u64);
        assert_eq!(obj.to_i64().unwrap(), 42i64);

        let obj: WAFObject = (-42i64).into();
        assert_eq!(obj.to_i64().unwrap(), -42i64);

        let obj: WAFObject = 3.0.into();
        assert_eq!(obj.to_f64().unwrap(), 3.0f64);

        let obj: WAFObject = true.into();
        assert!(obj.to_bool().unwrap());

        let obj: WAFObject = ().into();
        assert_eq!(obj.get_type(), WAFObjectType::Null);

        let obj: WAFObject = "Hello, world!".into();
        assert_eq!(obj.to_str().unwrap(), "Hello, world!");

        let obj: WAFObject = b"Hello, world!"[..].into();
        assert_eq!(obj.to_str().unwrap(), "Hello, world!");
    }

    #[test]
    fn ddwaf_obj_failed_conversions() {
        let mut obj: WAFObject = ().into();
        assert!(obj.as_type::<WAFBool>().is_none());
        assert!(obj.as_type_mut::<WAFBool>().is_none());

        assert!(obj.to_bool().is_none());
        assert!(obj.to_u64().is_none());
        assert!(obj.to_i64().is_none());
        assert!(obj.to_f64().is_none());
        assert!(obj.to_str().is_none());
    }

    #[test]
    fn invalid_utf8() {
        let non_utf8_str: &[u8] = &[0x80];
        let obj: Keyed<WAFString> = (non_utf8_str, non_utf8_str).into();
        assert_eq!(format!("{obj:?}"), r#""\x80"=WAFString("\x80")"#);

        assert!(obj.key_str().is_err());
        assert!(obj.as_str().is_err());
    }

    #[test]
    fn empty_key() {
        let map = waf_map!(("", 42_u64));
        let empty_slice: &[u8] = &[];
        assert_eq!(map[0].key(), empty_slice);
    }

    #[test]
    fn keyed_obj_methods() {
        let mut map = waf_map!(("key", 42_u64));
        let elem = &mut map[0];
        assert!(elem.as_type::<WAFBool>().is_none());
        let elem_cast = elem.as_type::<WAFUnsigned>().unwrap();
        assert_eq!(elem_cast.value(), 42u64);

        assert!(elem.as_type_mut::<WAFBool>().is_none());
        let elem_cast = elem.as_type_mut::<WAFUnsigned>().unwrap();
        elem_cast.set_key_str("key 2");
        assert_eq!(elem_cast.key_str().unwrap(), "key 2");
    }

    #[test]
    fn map_fetching_methods() {
        let mut map = waf_map!(("key1", 1u64), ("key2", 2u64),);

        // index
        assert_eq!(map[0].key(), b"key1");
        // index mut
        map[0].set_key(b"new key");
        assert_eq!(map[0].key(), b"new key");

        // get
        assert_eq!(map.get(b"key2").unwrap().to_u64().unwrap(), 2);
        assert!(map.get(b"bad key").is_none());
        // get_str
        assert_eq!(map.get_str("key2").unwrap().to_u64().unwrap(), 2);
        assert!(map.get_str("bad key").is_none());

        // get_mut
        map.get_mut(b"key2").unwrap().set_key_str("key3");
        let entry_k3 = map.get_str_mut("key3").unwrap();
        let new_entry: Keyed<WAFUnsigned> = ("key3", 3u64).into();
        let _ = std::mem::replace(entry_k3, new_entry.into());
        assert_eq!(map.get_str("key3").unwrap().to_u64().unwrap(), 3);

        assert!(map.get_mut(b"bad key").is_none());

        // get_str_mut
        map.get_str_mut("key3").unwrap().set_key(b"key4");
        assert_eq!(map.get_str("key4").unwrap().to_u64().unwrap(), 3);

        assert!(map.get_str_mut("bad key").is_none());
    }

    #[test]
    fn array_iteration() {
        let mut arr = waf_array!(1u64, "foo", waf_array!("xyz"), waf_object!(null));

        for (i, elem) in arr.iter().enumerate() {
            match i {
                0 => assert_eq!(elem.to_u64().unwrap(), 1),
                1 => assert_eq!(elem.to_str().unwrap(), "foo"),
                2 => assert_eq!(elem.as_type::<WAFArray>().unwrap().len(), 1),
                3 => assert_eq!(elem.get_type(), WAFObjectType::Null),
                _ => unreachable!(),
            }
        }

        for (i, elem) in arr.iter_mut().enumerate() {
            match i {
                0 => assert_eq!(elem.to_u64().unwrap(), 1),
                1 => {
                    assert_eq!(elem.to_str().unwrap(), "foo");
                    let new_str: WAFString = "bar".into();
                    let _ = std::mem::replace(elem, new_str.into());
                }
                2 => assert_eq!(elem.as_type::<WAFArray>().unwrap().len(), 1),
                3 => assert_eq!(elem.get_type(), WAFObjectType::Null),
                _ => unreachable!(),
            }
        }
        assert_eq!(arr[1].to_str().unwrap(), "bar");

        for (i, elem) in arr.into_iter().enumerate() {
            match i {
                0 => assert_eq!(elem.to_u64().unwrap(), 1),
                1 => assert_eq!(elem.to_str().unwrap(), "bar"),
                2 => assert_eq!(elem.as_type::<WAFArray>().unwrap().len(), 1),
                3 => assert_eq!(elem.get_type(), WAFObjectType::Null),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn map_iteration() {
        let mut map = waf_map!(
            ("key1", 1u64),
            ("key2", "foo"),
            ("key3", waf_array!("xyz")),
            ("key4", waf_object!(null))
        );

        for (i, elem) in map.iter().enumerate() {
            match i {
                0 => {
                    assert_eq!(elem.key_str().unwrap(), "key1");
                    assert_eq!(elem.to_u64().unwrap(), 1);
                }
                1 => {
                    assert_eq!(elem.key_str().unwrap(), "key2");
                    assert_eq!(elem.to_str().unwrap(), "foo");
                }
                2 => {
                    assert_eq!(elem.key_str().unwrap(), "key3");
                    assert_eq!(elem.as_type::<WAFArray>().unwrap().len(), 1);
                }
                3 => {
                    assert_eq!(elem.key_str().unwrap(), "key4");
                    assert_eq!(elem.get_type(), WAFObjectType::Null);
                }
                _ => unreachable!(),
            }
        }

        for (i, elem) in map.iter_mut().enumerate() {
            match i {
                0 => assert_eq!(elem.to_u64().unwrap(), 1),
                1 => {
                    assert_eq!(elem.key_str().unwrap(), "key2");
                    assert_eq!(elem.to_str().unwrap(), "foo");
                    let new_val: Keyed<WAFString> = ("new_key", "bar").into();
                    let _ = std::mem::replace(elem, new_val.into());
                }
                2 => assert_eq!(elem.key_str().unwrap(), "key3"),
                3 => assert_eq!(elem.key_str().unwrap(), "key4"),
                _ => unreachable!(),
            }
        }

        assert_eq!(map[1].key_str().unwrap(), "new_key");
        assert_eq!(map[1].to_str().unwrap(), "bar");

        for (i, elem) in map.into_iter().enumerate() {
            match i {
                0 => assert_eq!(elem.key_str().unwrap(), "key1"),
                1 => assert_eq!(elem.key_str().unwrap(), "new_key"),
                2 => assert_eq!(elem.key_str().unwrap(), "key3"),
                3 => assert_eq!(elem.key_str().unwrap(), "key4"),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn partial_iteration() {
        let arr = waf_array!(1u64, "foo");
        for elem in arr {
            if elem.get_type() == WAFObjectType::Unsigned {
                break;
            }
        }

        let map = waf_map!(("key1", 1u64), ("key2", "foo"));
        for elem in map {
            if elem.get_type() == WAFObjectType::Unsigned {
                break;
            }
        }
    }

    #[test]
    fn iteration_of_empty_containers() {
        let mut arr: WAFArray = waf_array!();
        assert!(arr.iter().next().is_none());
        assert!(arr.iter_mut().next().is_none());
        assert!(arr.into_iter().next().is_none());

        let mut map = waf_map!();
        assert!(map.iter().next().is_none());
        assert!(map.iter_mut().next().is_none());
        assert!(map.into_iter().next().is_none());
    }

    #[test]
    fn iteration_of_keyed_array() {
        let mut map = waf_map!(("key1", waf_array!(1u64, "foo")));
        let keyed_array: &mut Keyed<WAFArray> = map[0].as_type_mut().unwrap();

        for (i, elem) in keyed_array.iter().enumerate() {
            match i {
                0 => assert_eq!(elem.to_u64().unwrap(), 1),
                1 => assert_eq!(elem.to_str().unwrap(), "foo"),
                _ => unreachable!(),
            }
        }

        for (i, elem) in keyed_array.iter_mut().enumerate() {
            match i {
                0 => assert_eq!(elem.to_u64().unwrap(), 1),
                1 => {
                    assert_eq!(elem.to_str().unwrap(), "foo");
                    let new_str: WAFString = "bar".into();
                    let _ = std::mem::replace(elem, new_str.into());
                }
                _ => unreachable!(),
            }
        }

        assert_eq!(keyed_array[1].to_str().unwrap(), "bar");

        for (i, elem) in std::mem::take(keyed_array).into_iter().enumerate() {
            match i {
                0 => assert_eq!(elem.to_u64().unwrap(), 1),
                1 => assert_eq!(elem.to_str().unwrap(), "bar"),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn iteration_of_keyed_map() {
        let mut map = waf_map!(("key1", waf_map!(("key2", 1u64))));
        let keyed_map: &mut Keyed<WAFMap> = map[0].as_type_mut().unwrap();

        for (i, elem) in keyed_map.iter().enumerate() {
            match i {
                0 => {
                    assert_eq!(elem.key_str().unwrap(), "key2");
                    assert_eq!(elem.to_u64().unwrap(), 1);
                }
                _ => unreachable!(),
            }
        }

        for (i, elem) in keyed_map.iter_mut().enumerate() {
            match i {
                0 => {
                    assert_eq!(elem.key_str().unwrap(), "key2");
                    assert_eq!(elem.to_u64().unwrap(), 1);
                    let new_val: Keyed<WAFString> = ("new_key", "bar").into();
                    let _ = std::mem::replace(elem, new_val.into());
                }
                _ => unreachable!(),
            }
        }
        assert_eq!(keyed_map[0].key_str().unwrap(), "new_key");
        assert_eq!(keyed_map[0].to_str().unwrap(), "bar");

        for (i, elem) in std::mem::take(keyed_map).into_iter().enumerate() {
            match i {
                0 => {
                    assert_eq!(elem.key_str().unwrap(), "new_key");
                    assert_eq!(elem.to_str().unwrap(), "bar");
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn try_from_implementations() {
        let obj = waf_object!(42u64);
        assert!(WAFArray::try_from(obj).is_err());
        let obj = waf_object!(42u64);
        assert!(WAFUnsigned::try_from(obj).is_ok());

        let obj = waf_object!(42);
        assert!(WAFUnsigned::try_from(obj).is_err());
        let obj = waf_object!(42);
        assert!(WAFSigned::try_from(obj).is_ok());

        let obj = waf_object!(42.0);
        assert!(WAFSigned::try_from(obj).is_err());
        let obj = waf_object!(42.0);
        assert!(WAFFloat::try_from(obj).is_ok());

        let obj = waf_object!(true);
        assert!(WAFFloat::try_from(obj).is_err());
        let obj = waf_object!(true);
        assert!(WAFBool::try_from(obj).is_ok());

        let obj = waf_object!(null);
        assert!(WAFBool::try_from(obj).is_err());
        let obj = waf_object!(null);
        assert!(WAFNull::try_from(obj).is_ok());

        let obj = waf_object!("foobar");
        assert!(WAFNull::try_from(obj).is_err());
        let obj = waf_object!("foobar");
        assert!(WAFString::try_from(obj).is_ok());

        let obj: WAFObject = waf_map!().into();
        assert!(WAFString::try_from(obj).is_err());
        let obj: WAFObject = waf_map!().into();
        assert!(WAFMap::try_from(obj).is_ok());

        let obj: WAFObject = waf_array!().into();
        assert!(WAFMap::try_from(obj).is_err());
        let obj: WAFObject = waf_array!().into();
        assert!(WAFArray::try_from(obj).is_ok());
    }

    #[test]
    #[allow(clippy::float_cmp)] // No operations are done on the values, they should be the same.
    fn unsafe_changes_to_default_objects() {
        unsafe {
            let mut unsigned = WAFUnsigned::default();
            unsigned.as_raw_mut().__bindgen_anon_1.uintValue += 1;
            assert_eq!(unsigned.value(), 1);

            let mut signed = WAFSigned::default();
            signed.as_raw_mut().__bindgen_anon_1.intValue -= 1;
            assert_eq!(signed.value(), -1);

            let mut float = WAFFloat::default();
            float.as_raw_mut().__bindgen_anon_1.f64_ += 1.0;
            assert_eq!(float.value(), 1.0);

            let mut boolean = WAFBool::default();
            boolean.as_raw_mut().__bindgen_anon_1.boolean = true;
            assert!(boolean.value());

            let mut null = WAFNull::default();
            // nothing interesting to do for null; let's try manually setting
            // the parameter name
            let s = String::from_str("foobar").unwrap();
            let b: Box<[u8]> = s.as_bytes().into();
            let p = Box::<[u8]>::into_raw(b);
            let null_mut = null.as_raw_mut();
            null_mut.parameterName = p.cast();
            null_mut.parameterNameLength = s.len() as u64;
            drop(std::mem::take(null_mut.as_keyed_object_mut()));

            let mut string = WAFString::default();
            let str_mut = string.as_raw_mut();
            let b: Box<[u8]> = s.as_bytes().into();
            let p = Box::<[u8]>::into_raw(b);
            str_mut.drop_string();
            str_mut.__bindgen_anon_1.stringValue = p as *const _;
            str_mut.nbEntries = s.len() as u64;
            assert_eq!(string.as_str().unwrap(), "foobar");
            assert_eq!(string.len(), s.len());
            assert!(!string.is_empty());
        }
    }
}
