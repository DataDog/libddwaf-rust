#![cfg(feature = "serde")]

use libddwaf::{
    object::{WAFArray, WAFMap, WAFObject, WAFObjectType},
    waf_array, waf_map, waf_object,
};
use serde_json::from_str;

#[test]
fn sample_json_deserialization() {
    // Test for a simple unsigned integer
    let json = "42";
    let ddwaf_obj: WAFObject = from_str(json).expect("Failed to deserialize u64");
    assert_eq!(ddwaf_obj.to_u64().unwrap(), 42);

    // Test for a signed integer
    let json = "-42";
    let ddwaf_obj: WAFObject = from_str(json).expect("Failed to deserialize i64");
    assert_eq!(ddwaf_obj.to_i64().unwrap(), -42);

    // Test for a boolean
    let json = "true";
    let ddwaf_obj: WAFObject = from_str(json).expect("Failed to deserialize bool");
    assert!(ddwaf_obj.to_bool().unwrap());

    // Test for null
    let json = "null";
    let ddwaf_obj: WAFObject = from_str(json).expect("Failed to deserialize null");
    assert_eq!(ddwaf_obj.get_type(), WAFObjectType::Null);

    // Test for a string
    let json = "\"hello\"";
    let ddwaf_obj: WAFObject = from_str(json).expect("Failed to deserialize string");
    assert_eq!(ddwaf_obj.to_str().unwrap(), "hello");

    // Test for an array
    let json = "[1, 2, 3]";
    let array: WAFArray = from_str::<WAFObject>(json)
        .expect("Failed to deserialize array")
        .try_into()
        .unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0].to_u64().unwrap(), 1);
    assert_eq!(array[1].to_u64().unwrap(), 2);
    assert_eq!(array[2].to_u64().unwrap(), 3);

    // Test for a map
    let json = "{\"key1\": \"value1\", \"key2\": 42}";
    let map: WAFMap = from_str::<WAFObject>(json)
        .expect("Failed to deserialize map")
        .try_into()
        .unwrap();
    assert_eq!(map.len(), 2);
    assert_eq!(map.get_str("key1").unwrap().to_str().unwrap(), "value1");
    assert_eq!(map.get_str("key2").unwrap().to_u64().unwrap(), 42);
}

#[test]
fn map_deserialization_ok() {
    let json = "{\"key1\": 42, \"key2\": 43}";
    let map: WAFMap = from_str::<WAFMap>(json).expect("Failed to deserialize map");
    assert_eq!(map.len(), 2);
    assert_eq!(map.get_str("key1").unwrap().to_u64().unwrap(), 42);
    assert_eq!(map.get_str("key2").unwrap().to_u64().unwrap(), 43);
}

#[test]
fn map_deserialization_wrong_type() {
    let json = "[42]";
    let maybe_map = from_str::<WAFMap>(json);

    assert!(maybe_map.is_err());
    assert_eq!(
        maybe_map.err().unwrap().to_string(),
        "invalid type: not a map"
    );
}

#[test]
fn sample_json_serialization() {
    let root = waf_array!(
        "Hello, world!",
        123_u64,
        waf_map!(
            ("key 1", "value 1"),
            ("key 2", -2_i64),
            ("key 3", 2_u64),
            ("key 4", 5.2),
            ("key 5", waf_object!(null)),
            ("key 5", waf_object!(true)),
        ),
        waf_array!(),
        waf_map!(),
    );

    let res = serde_json::to_string_pretty(&root).unwrap();
    let expected_string = r#"
[
  "Hello, world!",
  123,
  {
    "key 1": "value 1",
    "key 2": -2,
    "key 3": 2,
    "key 4": 5.2,
    "key 5": null,
    "key 5": true
  },
  [],
  {}
]
"#;
    assert_eq!(res, expected_string.trim());
}
