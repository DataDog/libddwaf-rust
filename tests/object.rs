use libddwaf::{object::*, waf_array, waf_map, waf_object};

#[test]
#[allow(clippy::float_cmp)] // No operations are done on the values, they should be the same.
fn defaults() {
    let obj = WAFObject::default();
    assert!(!obj.is_valid());
    assert_eq!(obj.get_type(), WAFObjectType::Invalid);

    let obj = WAFSigned::default();
    assert!(obj.is_valid());
    assert_eq!(obj.value(), 0);

    let obj = WAFUnsigned::default();
    assert!(obj.is_valid());
    assert_eq!(obj.value(), 0);

    let obj = WAFString::default();
    assert!(obj.is_valid());
    assert_eq!(obj.as_str(), Ok(""));

    let obj = WAFArray::default();
    assert!(obj.is_valid());
    assert_eq!(obj.len(), 0);

    let obj = WAFMap::default();
    assert!(obj.is_valid());
    assert_eq!(obj.len(), 0);

    let obj = WAFBool::default();
    assert!(obj.is_valid());
    assert!(!obj.value());

    let obj = WAFFloat::default();
    assert!(obj.is_valid());
    assert_eq!(obj.value(), 0.0);

    let obj = WAFNull::default();
    assert!(obj.is_valid());
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
    map[2] = ("key 3", 2_u32).into();
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
fn string_debug_value() {
    let obj = waf_map!((r#"key"hey"#, r"value\n"));
    assert_eq!(
        format!("{obj:?}"),
        r#"WAFMap{"key\"hey"=WAFString("value\\n")}"#
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
    assert_eq!(obj.to_str(), Some("Hello, world!"));

    let obj: WAFObject = b"Hello, world!"[..].into();
    assert_eq!(obj.to_str(), Some("Hello, world!"));
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
#[should_panic(expected = "index out of bounds (3 >= 3)")]
fn array_index_out_of_bounds() {
    let arr = waf_array!(1u64, "hello", waf_object!(null));
    let _ = arr[3]; // Panics
}

#[test]
#[should_panic(expected = "index out of bounds (3 >= 3)")]
fn array_index_mut_of_bounds() {
    let mut arr = waf_array!(1u64, "hello", waf_object!(null));
    arr[3] = 42u64.into(); // Panics
}

#[test]
#[should_panic(expected = "index out of bounds (3 >= 3)")]
fn map_index_out_of_bounds() {
    let arr = waf_map!(("a", 1u64), ("b", "hello"), ("c", waf_object!(null)));
    let _ = arr[3]; // Panics
}

#[test]
#[should_panic(expected = "index out of bounds (3 >= 3)")]
fn map_index_mut_of_bounds() {
    let mut arr = waf_map!(("a", 1u64), ("b", "hello"), ("c", waf_object!(null)));
    arr[3] = Keyed::from(("d", 42u64)); // Panics
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
#[allow(clippy::float_cmp)] // No operations performed on the floats, they should be identical.
fn from_implementations() {
    assert_eq!(WAFSigned::from(-123i64).value(), -123);
    assert_eq!(WAFSigned::from(-123i32).value(), -123);

    assert_eq!(WAFUnsigned::from(123u64).value(), 123);
    assert_eq!(WAFUnsigned::from(123u32).value(), 123);

    assert_eq!(
        WAFString::from("Hello, world!").as_str(),
        Ok("Hello, world!")
    );
    assert_eq!(
        WAFString::from(b"Hello, world!").as_str(),
        Ok("Hello, world!")
    );

    let arr = WAFArray::from([1u64, 2u64, 3u64]);
    for (i, elem) in arr.iter().enumerate() {
        assert_eq!(elem.to_u64().unwrap(), i as u64 + 1);
    }

    let map = WAFMap::from([("1", 1u64), ("2", 2u64)]);
    for elem in map {
        let key = elem.key_str().unwrap();
        let val = elem.to_u64().unwrap();
        assert_eq!(key, format!("{val}"));
    }

    assert!(WAFBool::from(true).value());
    assert!(!WAFBool::from(false).value());

    assert_eq!(WAFFloat::from(1.0).value(), 1.0);

    assert!(WAFNull::from(()).is_valid());
}

#[test]
fn try_from_implementations() {
    assert!(matches!(
        WAFSigned::try_from(WAFObject::default()),
        Err(ObjectTypeError {
            expected: WAFObjectType::Signed,
            actual: WAFObjectType::Invalid
        })
    ));
    assert!(matches!(
        WAFUnsigned::try_from(WAFObject::default()),
        Err(ObjectTypeError {
            expected: WAFObjectType::Unsigned,
            actual: WAFObjectType::Invalid
        })
    ));
    assert!(matches!(
        WAFString::try_from(WAFObject::default()),
        Err(ObjectTypeError {
            expected: WAFObjectType::String,
            actual: WAFObjectType::Invalid
        })
    ));
    assert!(matches!(
        WAFArray::try_from(WAFObject::default()),
        Err(ObjectTypeError {
            expected: WAFObjectType::Array,
            actual: WAFObjectType::Invalid
        })
    ));
    assert!(matches!(
        WAFMap::try_from(WAFObject::default()),
        Err(ObjectTypeError {
            expected: WAFObjectType::Map,
            actual: WAFObjectType::Invalid
        })
    ));
    assert!(matches!(
        WAFBool::try_from(WAFObject::default()),
        Err(ObjectTypeError {
            expected: WAFObjectType::Bool,
            actual: WAFObjectType::Invalid
        })
    ));
    assert!(matches!(
        WAFFloat::try_from(WAFObject::default()),
        Err(ObjectTypeError {
            expected: WAFObjectType::Float,
            actual: WAFObjectType::Invalid
        })
    ));
    assert!(matches!(
        WAFNull::try_from(WAFObject::default()),
        Err(ObjectTypeError {
            expected: WAFObjectType::Null,
            actual: WAFObjectType::Invalid
        })
    ));

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
