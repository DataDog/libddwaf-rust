#![cfg(feature = "serde")]

use libddwaf::{
    object::{WafArray, WafMap, WafObject, WafObjectType, WafString},
    serde::{deserialize_with_limits, Limits},
    waf_array, waf_map, waf_object,
};
use serde_json::from_str;

#[test]
fn sample_json_deserialization() {
    // Test for a simple unsigned integer
    let json = "42";
    let ddwaf_obj: WafObject = from_str(json).expect("Failed to deserialize u64");
    assert_eq!(ddwaf_obj.to_u64().unwrap(), 42);

    // Test for a signed integer
    let json = "-42";
    let ddwaf_obj: WafObject = from_str(json).expect("Failed to deserialize i64");
    assert_eq!(ddwaf_obj.to_i64().unwrap(), -42);

    // Test for a boolean
    let json = "true";
    let ddwaf_obj: WafObject = from_str(json).expect("Failed to deserialize bool");
    assert!(ddwaf_obj.to_bool().unwrap());

    // Test for null
    let json = "null";
    let ddwaf_obj: WafObject = from_str(json).expect("Failed to deserialize null");
    assert_eq!(ddwaf_obj.object_type(), WafObjectType::Null);

    // Test for a string
    let json = "\"hello\"";
    let ddwaf_obj: WafObject = from_str(json).expect("Failed to deserialize string");
    assert_eq!(ddwaf_obj.to_str().unwrap(), "hello");

    // Test for an array
    let json = "[1, 2, 3]";
    let array: WafArray = from_str::<WafObject>(json)
        .expect("Failed to deserialize array")
        .try_into()
        .unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0].to_u64().unwrap(), 1);
    assert_eq!(array[1].to_u64().unwrap(), 2);
    assert_eq!(array[2].to_u64().unwrap(), 3);

    // Test for a map
    let json = "{\"key1\": \"value1\", \"key2\": 42}";
    let map: WafMap = from_str::<WafObject>(json)
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
    let map: WafMap = from_str::<WafMap>(json).expect("Failed to deserialize map");
    assert_eq!(map.len(), 2);
    assert_eq!(map.get_str("key1").unwrap().to_u64().unwrap(), 42);
    assert_eq!(map.get_str("key2").unwrap().to_u64().unwrap(), 43);
}

#[test]
fn map_deserialization_wrong_type() {
    let json = "[42]";
    let maybe_map = from_str::<WafMap>(json);

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

// ============================================================================
// Tests for deserialization with limits
// ============================================================================

#[test]
fn limits_default_values() {
    let limits = Limits::default();
    assert_eq!(limits.max_string_length, 4096);
    assert_eq!(limits.max_depth, 21);
    assert_eq!(limits.max_elements, 2048);
}

#[test]
fn limits_no_truncation_when_within_limits() {
    let json = r#"{"key": "value", "number": 42}"#;
    let limits = Limits {
        max_string_length: 100,
        max_depth: 10,
        max_elements: 100,
    };
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();

    assert!(!result.truncated);
    let expected = waf_map!(("key", "value"), ("number", 42_u64));
    assert_eq!(result.value, expected);
}

#[test]
fn limits_string_truncation() {
    let json = r#"{"key": "this is a long string that should be truncated"}"#;
    let limits = Limits {
        max_string_length: 10,
        max_depth: 10,
        max_elements: 100,
    };
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();

    assert!(result.truncated);
    let expected = waf_map!(("key", "this is a "));
    assert_eq!(result.value, expected);
}

#[test]
fn limits_string_truncation_utf8_boundary() {
    // Test that string truncation respects UTF-8 character boundaries
    // "日本語" = 9 bytes (3 bytes per character)
    let json = r#""日本語test""#;
    let limits = Limits {
        max_string_length: 7,
        max_depth: 10,
        max_elements: 100,
    };
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();

    assert!(result.truncated);
    // Should truncate to valid UTF-8 boundary: "日本" (6 bytes)
    let expected = waf_object!("日本");
    assert_eq!(result.value, expected);
}

#[test]
fn limits_depth_exceeded() {
    let json = r#"{"a": {"b": {"c": {"d": "deep"}}}}"#;
    let limits = Limits {
        max_string_length: 100,
        max_depth: 2,
        max_elements: 100,
    };
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();

    assert!(result.truncated);
    let expected = waf_map!(("a", waf_map!(("b", waf_object!(null)))));
    assert_eq!(result.value, expected);
}

#[test]
fn limits_depth_zero() {
    let json = r#"{"key": "value"}"#;
    let limits = Limits {
        max_string_length: 100,
        max_depth: 0,
        max_elements: 100,
    };
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();

    assert!(result.truncated);
    let expected = waf_object!(null);
    assert_eq!(result.value, expected);
}

#[test]
fn limits_elements_exceeded_in_array() {
    let json = r#"[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]"#;
    let limits = Limits {
        max_string_length: 100,
        max_depth: 10,
        max_elements: 5,
    };
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();

    assert!(result.truncated);
    // 5 elements: 1 for array + 4 values
    let expected = waf_array!(1_u64, 2_u64, 3_u64, 4_u64);
    assert_eq!(result.value, expected);
}

#[test]
fn limits_elements_exceeded_in_map() {
    let json = r#"{"a": 1, "b": 2, "c": 3, "d": 4, "e": 5}"#;
    let limits = Limits {
        max_string_length: 100,
        max_depth: 10,
        max_elements: 5,
    };
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();

    assert!(result.truncated);
    // 5 elements: 1 for map + 2 entries (key+value each)
    let expected = waf_map!(("a", 1_u64), ("b", 2_u64));
    assert_eq!(result.value, expected);
}

#[test]
fn limits_combined_depth_and_elements() {
    let json = r#"{"level1": {"level2": [1, 2, 3, 4, 5]}}"#;
    let limits = Limits {
        max_string_length: 100,
        max_depth: 3,
        max_elements: 15,
    };
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();

    assert!(!result.truncated);
    let expected = waf_map!((
        "level1",
        waf_map!(("level2", waf_array!(1_u64, 2_u64, 3_u64, 4_u64, 5_u64)))
    ));
    assert_eq!(result.value, expected);
}

#[test]
fn limits_siblings_preserved_when_nested_truncated() {
    // When a nested structure hits depth limit, siblings should still be preserved
    let json = r#"["before", {"a": {"b": {"c": "deep"}}}, "after"]"#;
    let limits = Limits {
        max_string_length: 100,
        max_depth: 3,
        max_elements: 100,
    }; // depth=3: array->map->map->null
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();

    assert!(result.truncated);
    let expected = waf_array!(
        "before",
        waf_map!(("a", waf_map!(("b", waf_object!(null))))),
        "after"
    );
    assert_eq!(result.value, expected);
}

#[test]
fn limits_depth_exactly_at_limit() {
    // 3 levels deep with depth=3 should work (array -> map -> map -> string)
    let json = r#"[{"a": {"b": "value"}}]"#;
    let limits = Limits {
        max_string_length: 100,
        max_depth: 3,
        max_elements: 100,
    };
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();

    assert!(!result.truncated);
    let expected = waf_array!(waf_map!(("a", waf_map!(("b", "value")))));
    assert_eq!(result.value, expected);
}

#[test]
fn limits_depth_one_over_limit() {
    // 4 levels deep with depth=3 should truncate deepest
    let json = r#"[{"a": {"b": {"c": "value"}}}]"#;
    let limits = Limits {
        max_string_length: 100,
        max_depth: 3,
        max_elements: 100,
    };
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();

    assert!(result.truncated);
    let expected = waf_array!(waf_map!(("a", waf_map!(("b", waf_object!(null))))));
    assert_eq!(result.value, expected);
}

#[test]
fn limits_nested_arrays_depth_truncated() {
    let json = r#"[[1, 2], [3, 4], [5, 6]]"#;
    let limits = Limits {
        max_string_length: 100,
        max_depth: 1,
        max_elements: 100,
    };
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();

    assert!(result.truncated);
    let expected = waf_array!(waf_object!(null), waf_object!(null), waf_object!(null));
    assert_eq!(result.value, expected);
}

#[test]
fn test_zero_element_limit() {
    let json = r#"[1, 2, 3]"#;
    let limits = Limits {
        max_string_length: 100,
        max_depth: 10,
        max_elements: 0,
    };
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits);

    // With 0 elements allowed, we can't even create the array
    // This should still succeed but truncate everything
    assert!(result.is_ok());
    let res = result.unwrap();
    assert!(res.truncated);
    assert_eq!(res.value, waf_object!(null));
}

#[test]
fn limits_visit_i64_with_element_exhaustion() {
    let json = "-42";
    let limits = Limits {
        max_string_length: 100,
        max_depth: 10,
        max_elements: 0,
    };
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();

    assert!(result.truncated);
    assert_eq!(result.value, waf_object!(null));
}

#[test]
fn limits_visit_f64_with_element_exhaustion() {
    let json = "3.14";
    let limits = Limits {
        max_string_length: 100,
        max_depth: 10,
        max_elements: 0,
    };
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();

    assert!(result.truncated);
    assert_eq!(result.value, waf_object!(null));
}

#[test]
fn limits_visit_bool_with_element_exhaustion() {
    let json = "true";
    let limits = Limits {
        max_string_length: 100,
        max_depth: 10,
        max_elements: 0,
    };
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();

    assert!(result.truncated);
    assert_eq!(result.value, waf_object!(null));
}

#[test]
fn limits_visit_unit_with_element_exhaustion() {
    let json = "null";
    let limits = Limits {
        max_string_length: 100,
        max_depth: 10,
        max_elements: 0,
    };
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();

    assert!(result.truncated);
    assert_eq!(result.value, waf_object!(null));
}

#[test]
fn limits_visit_primitives_success_paths() {
    // Test that primitive values are properly created when element limit allows
    let limits = Limits {
        max_string_length: 100,
        max_depth: 10,
        max_elements: 10,
    };

    // Test i64
    let json = "-42";
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();
    assert!(!result.truncated);
    assert_eq!(result.value.to_i64().unwrap(), -42);

    // Test f64
    let json = "3.28";
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();
    assert!(!result.truncated);
    assert_eq!(result.value.to_f64().unwrap(), 3.28);

    // Test bool
    let json = "false";
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();
    assert!(!result.truncated);
    assert!(!result.value.to_bool().unwrap());

    // Test null (unit)
    let json = "null";
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();
    assert!(!result.truncated);
    assert_eq!(result.value, waf_object!(null));
}

#[test]
fn limits_visit_str_with_element_exhaustion() {
    let json = r#""hello""#;
    let limits = Limits {
        max_string_length: 100,
        max_depth: 10,
        max_elements: 0,
    };
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();

    assert!(result.truncated);
    assert_eq!(result.value, waf_object!(null));
}

#[test]
fn limits_visit_u64_with_element_exhaustion() {
    let json = "42";
    let limits = Limits {
        max_string_length: 100,
        max_depth: 10,
        max_elements: 0,
    };
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();

    assert!(result.truncated);
    assert_eq!(result.value, waf_object!(null));
}

#[test]
fn test_visit_bytes_directly() {
    // Test the basic Visitor's visit_bytes method by creating a custom deserializer
    use serde::de::Visitor;

    struct BytesDeserializer(&'static [u8]);

    impl<'de> serde::Deserializer<'de> for BytesDeserializer {
        type Error = serde::de::value::Error;

        fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            visitor.visit_bytes(self.0)
        }

        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }

    // Test basic visitor
    let bytes: &[u8] = b"test bytes";
    let deserializer = BytesDeserializer(bytes);
    let result: WafObject = serde::Deserialize::deserialize(deserializer).unwrap();
    assert_eq!(result.to_str().unwrap(), "test bytes");
}

#[test]
fn limits_visit_bytes_with_truncation() {
    // Test LimitedVisitor's visit_bytes with truncation
    use serde::de::Visitor;

    struct BytesDeserializer(&'static [u8]);

    impl<'de> serde::Deserializer<'de> for BytesDeserializer {
        type Error = serde::de::value::Error;

        fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            visitor.visit_bytes(self.0)
        }

        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }

    let bytes: &[u8] = b"This is a long byte string that should be truncated";
    let limits = Limits {
        max_string_length: 10,
        max_depth: 10,
        max_elements: 100,
    };
    let deserializer = BytesDeserializer(bytes);
    let result = deserialize_with_limits(deserializer, &limits).unwrap();

    assert!(result.truncated);
    // Should be truncated to 10 bytes
    assert_eq!(
        result
            .value
            .as_type::<WafString>()
            .unwrap()
            .as_bytes()
            .len(),
        10
    );
}

#[test]
fn limits_visit_bytes_with_element_exhaustion() {
    // Test LimitedVisitor's visit_bytes with element exhaustion
    use serde::de::Visitor;

    struct BytesDeserializer(&'static [u8]);

    impl<'de> serde::Deserializer<'de> for BytesDeserializer {
        type Error = serde::de::value::Error;

        fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            visitor.visit_bytes(self.0)
        }

        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map struct enum identifier ignored_any
        }
    }

    let bytes: &[u8] = b"test";
    let limits = Limits {
        max_string_length: 100,
        max_depth: 10,
        max_elements: 0,
    };
    let deserializer = BytesDeserializer(bytes);
    let result = deserialize_with_limits(deserializer, &limits).unwrap();

    assert!(result.truncated);
    assert_eq!(result.value, waf_object!(null));
}
