use libddwaf::{object::*, waf_array, waf_map};

#[test]
#[allow(clippy::float_cmp)]
fn clone_simple_types() {
    let orig = WafSigned::new(42);
    let copy = orig;
    assert_eq!(orig.value(), copy.value());
    assert_eq!(orig.value(), 42);

    let orig = WafUnsigned::new(123);
    let cloned = orig;
    assert_eq!(orig, cloned);
    assert_eq!(cloned.value(), 123);

    let invalid = WafInvalid::default();
    let invalid_clone = invalid;
    assert!(invalid == invalid_clone);

    let signed = WafSigned::new(-42);
    let signed_clone = signed;
    assert_eq!(signed, signed_clone);

    let unsigned = WafUnsigned::new(42);
    let unsigned_clone = unsigned;
    assert_eq!(unsigned, unsigned_clone);

    let boolean = WafBool::new(true);
    let boolean_clone = boolean;
    assert_eq!(boolean, boolean_clone);

    let float = WafFloat::new(3.28);
    let float_clone = float;
    assert_eq!(float, float_clone);

    let null = WafNull::new();
    let null_clone = null;
    assert_eq!(null, null_clone);
}

#[test]
fn clone_string_variants() {
    // SMALL_STRING (< 14 bytes)
    let small = WafString::new("hello").unwrap();
    let cloned = small.clone();
    assert_eq!(small.as_bytes(), cloned.as_bytes());
    drop(small);
    assert_eq!(cloned.as_bytes(), b"hello");

    // STRING (heap-allocated, > 14 bytes)
    let large = WafString::new("a".repeat(100)).unwrap();
    let cloned = large.clone();
    assert_eq!(large.as_bytes(), cloned.as_bytes());
    drop(large);
    assert_eq!(cloned.len(), 100);

    // LITERAL_STRING
    const STATIC_BYTES: &[u8] = b"static";
    let literal = WafString::new_literal(STATIC_BYTES);
    let cloned = literal.clone();
    assert_eq!(literal.as_bytes(), cloned.as_bytes());
    drop(literal);
    assert_eq!(cloned.as_bytes(), b"static");

    // Empty strings
    let empty = WafString::new("").unwrap();
    let cloned = empty.clone();
    assert_eq!(cloned.len(), 0);
    assert!(cloned.is_empty());
}

#[test]
fn clone_empty_containers() {
    let empty_arr = WafArray::new(0);
    let cloned_arr = empty_arr.clone();
    assert_eq!(cloned_arr.len(), 0);
    assert!(cloned_arr.is_empty());

    let empty_map = WafMap::new(0);
    let cloned_map = empty_map.clone();
    assert_eq!(cloned_map.len(), 0);
    assert!(cloned_map.is_empty());
}

#[test]
fn clone_array_shallow() {
    let mut orig = WafArray::new(3);
    orig[0] = 42u64.into();
    orig[1] = "hello".into();
    orig[2] = true.into();

    let cloned = orig.clone();

    // Verify values are equal
    assert_eq!(orig[0].to_u64(), cloned[0].to_u64());
    assert_eq!(orig[1].to_str(), cloned[1].to_str());
    assert_eq!(orig[2].to_bool(), cloned[2].to_bool());

    // Verify independence
    drop(orig);
    assert_eq!(cloned[0].to_u64(), Some(42));
    assert_eq!(cloned[1].to_str(), Some("hello"));
    assert_eq!(cloned[2].to_bool(), Some(true));
}

#[test]
fn clone_array_deep() {
    let mut orig = WafArray::new(3);
    orig[0] = 42u64.into();
    orig[1] = "hello".into();
    orig[2] = waf_array!(1u64, 2u64).into();

    let cloned = orig.clone();

    drop(orig);
    assert_eq!(cloned[0].to_u64(), Some(42));
    assert_eq!(cloned[1].to_str(), Some("hello"));

    let nested = cloned[2].as_type::<WafArray>().unwrap();
    assert_eq!(nested.len(), 2);
    assert_eq!(nested[0].to_u64(), Some(1));
    assert_eq!(nested[1].to_u64(), Some(2));
}

#[test]
fn clone_map_shallow() {
    let orig = waf_map!(("key1", 42u64), ("key2", "hello"));
    let cloned = orig.clone();

    assert_eq!(cloned.get_str("key1").unwrap().to_u64(), Some(42));
    assert_eq!(cloned.get_str("key2").unwrap().to_str(), Some("hello"));

    drop(orig);
    assert_eq!(cloned.get_str("key1").unwrap().to_u64(), Some(42));
}

#[test]
fn clone_map_deep() {
    let orig = waf_map!(("key1", 42u64), ("key2", waf_array!(1u64, 2u64)));
    let cloned = orig.clone();

    drop(orig);

    assert_eq!(cloned.get_str("key1").unwrap().to_u64(), Some(42));
    let nested_arr = cloned
        .get_str("key2")
        .unwrap()
        .as_type::<WafArray>()
        .unwrap();
    assert_eq!(nested_arr.len(), 2);
    assert_eq!(nested_arr[0].to_u64(), Some(1));
    assert_eq!(nested_arr[1].to_u64(), Some(2));
}

#[test]
fn clone_waf_object_dispatch() {
    let objects = vec![
        WafObject::from(42u64),
        WafObject::from(-42i64),
        WafObject::from(3.28),
        WafObject::from(true),
        WafObject::from("hello"),
        WafObject::from(()),
        WafObject::from(waf_array!(1u64)),
        WafObject::from(waf_map!(("k", "v"))),
    ];

    for obj in objects {
        let cloned = obj.clone();
        assert_eq!(obj, cloned);
        drop(obj);
        assert!(cloned.is_valid());
    }
}

#[test]
fn clone_keyed() {
    let orig: Keyed<WafObject> = ("key", 42u64).into();
    let cloned = orig.clone();

    assert_eq!(orig.key_str().unwrap(), cloned.key_str().unwrap());
    assert_eq!(orig.value().to_u64(), cloned.value().to_u64());

    drop(orig);
    assert_eq!(cloned.key_str().unwrap(), "key");
    assert_eq!(cloned.value().to_u64(), Some(42));
}

#[test]
fn clone_complex_nested() {
    let complex = waf_map!(
        (
            "users",
            waf_array!(
                waf_map!(("name", "Alice"), ("age", 30u64), ("active", true)),
                waf_map!(("name", "Bob"), ("age", 25u64), ("active", false))
            )
        ),
        ("metadata", waf_map!(("version", "1.0"), ("count", 2u64)))
    );

    let cloned = complex.clone();
    drop(complex);

    let users = cloned
        .get_str("users")
        .unwrap()
        .as_type::<WafArray>()
        .unwrap();
    assert_eq!(users.len(), 2);

    let alice = users[0].as_type::<WafMap>().unwrap();
    assert_eq!(alice.get_str("name").unwrap().to_str(), Some("Alice"));
    assert_eq!(alice.get_str("age").unwrap().to_u64(), Some(30));
    assert_eq!(alice.get_str("active").unwrap().to_bool(), Some(true));

    let bob = users[1].as_type::<WafMap>().unwrap();
    assert_eq!(bob.get_str("name").unwrap().to_str(), Some("Bob"));

    let metadata = cloned
        .get_str("metadata")
        .unwrap()
        .as_type::<WafMap>()
        .unwrap();
    assert_eq!(metadata.get_str("version").unwrap().to_str(), Some("1.0"));
    assert_eq!(metadata.get_str("count").unwrap().to_u64(), Some(2));
}

#[test]
fn clone_memory_independence() {
    let mut orig = waf_array!(1u64, 2u64, 3u64);
    let cloned = orig.clone();

    // Modify original
    orig[0] = 999u64.into();

    // Cloned should be unchanged
    assert_eq!(cloned[0].to_u64(), Some(1));
    assert_eq!(orig[0].to_u64(), Some(999));
}

#[test]
fn clone_array_size_not_capacity() {
    // Create array with capacity > size
    let mut orig = WafArray::new(5);
    orig[0] = 1u64.into();
    orig[1] = 2u64.into();
    orig.truncate(2);

    let cloned = orig.clone();

    // Cloned should have size == capacity == 2
    assert_eq!(cloned.len(), 2);
    assert_eq!(cloned[0].to_u64(), Some(1));
    assert_eq!(cloned[1].to_u64(), Some(2));
}

#[test]
fn clone_map_size_not_capacity() {
    // Create map with capacity > size
    let mut orig = WafMap::new(5);
    orig[0] = ("key1", 1u64).into();
    orig[1] = ("key2", 2u64).into();
    orig.truncate(2);

    assert_eq!(orig.capacity(), 5);

    let cloned = orig.clone();

    // Cloned should have size == capacity == 2
    assert_eq!(cloned.len(), 2);
    assert_eq!(cloned.capacity(), 2);
    assert_eq!(cloned.get_str("key1").unwrap().to_u64(), Some(1));
    assert_eq!(cloned.get_str("key2").unwrap().to_u64(), Some(2));
}
