#![doc = "Implementations of [serde::Deserialize] for [object::WAFObject](crate::object::WAFObject) and [object::WAFMap](crate::object::WAFMap)."]

use std::borrow::Cow;

use serde::{
    de::Error,
    ser::{SerializeMap, SerializeSeq},
    Deserializer,
};

use crate::object::{WAFArray, WAFMap, WAFObject, WAFObjectType, WAFString};

impl<'de> serde::Deserialize<'de> for WAFObject {
    fn deserialize<D>(deserializer: D) -> Result<WAFObject, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(Visitor)
    }
}

impl<'de> serde::Deserialize<'de> for WAFMap {
    fn deserialize<D>(deserializer: D) -> Result<WAFMap, D::Error>
    where
        D: Deserializer<'de>,
    {
        let dobj = deserializer.deserialize_any(Visitor)?;
        dobj.try_into()
            .map_err(|_| serde::de::Error::custom("invalid type: not a map"))
    }
}

struct Visitor;

impl<'de> serde::de::Visitor<'de> for Visitor {
    type Value = WAFObject;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an unsigned, a map, or an array")
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(WAFObject::from(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(WAFObject::from(v))
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(WAFObject::from(v))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(WAFObject::from(()))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(WAFObject::from(v))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(WAFObject::from(WAFString::new(v)))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        if let Some(size) = seq.size_hint() {
            let mut arr = WAFArray::new(size.try_into().unwrap());
            let mut i: usize = 0;
            while let Some(value) = seq.next_element()? {
                arr[i] = value;
                i += 1;
            }
            if i != size {
                return Err(serde::de::Error::custom("size hint was wrong"));
            }
            Ok(arr.into())
        } else {
            let mut vec = Vec::<WAFObject>::new();
            while let Some(value) = seq.next_element()? {
                vec.push(value);
            }
            let mut res = WAFArray::new(vec.len().try_into().unwrap());
            for (i, v) in vec.into_iter().enumerate() {
                res[i] = v;
            }
            Ok(res.into())
        }
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        if let Some(size) = map.size_hint() {
            let mut dmap = WAFMap::new(size.try_into().unwrap());
            let mut i: usize = 0;
            while let Some((key, value)) = map.next_entry::<Cow<'de, str>, WAFObject>()? {
                let key_str: &str = &key;
                dmap[i] = (key_str, value).into();
                i += 1;
            }
            if i != size {
                return Err(serde::de::Error::custom("size hint was wrong"));
            }
            Ok(dmap.into())
        } else {
            let mut vec = Vec::<(Cow<'de, str>, WAFObject)>::new();
            while let Some((key, value)) = map.next_entry::<Cow<'de, str>, WAFObject>()? {
                vec.push((key, value));
            }
            let mut res = WAFMap::new(vec.len().try_into().unwrap());
            for (i, (k, v)) in vec.into_iter().enumerate() {
                let key_str: &str = &k;
                res[i] = (key_str, v).into();
            }
            Ok(res.into())
        }
    }
}

impl serde::Serialize for WAFObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.get_type() {
            WAFObjectType::Unsigned => serializer.serialize_u64(self.to_u64().unwrap()),
            WAFObjectType::Signed => serializer.serialize_i64(self.to_i64().unwrap()),
            WAFObjectType::Bool => serializer.serialize_bool(self.to_bool().unwrap()),
            WAFObjectType::Float => serializer.serialize_f64(self.to_f64().unwrap()),
            WAFObjectType::String => serializer.serialize_str(&String::from_utf8_lossy(
                self.as_type::<WAFString>().unwrap().bytes(),
            )),
            WAFObjectType::Null | WAFObjectType::Invalid => serializer.serialize_unit(),
            WAFObjectType::Array => {
                let array = self.as_type::<WAFArray>().unwrap();
                let mut seq_serializer = serializer.serialize_seq(Some(array.len()))?;
                for value in array.iter() {
                    seq_serializer.serialize_element(value)?;
                }
                seq_serializer.end()
            }
            WAFObjectType::Map => {
                let map = self.as_type::<WAFMap>().unwrap();
                let mut map_serializer = serializer.serialize_map(Some(map.len()))?;
                for keyed_val in map.iter() {
                    map_serializer.serialize_entry(
                        &String::from_utf8_lossy(keyed_val.key()),
                        keyed_val.inner(),
                    )?;
                }
                map_serializer.end()
            }
        }
    }
}

impl serde::Serialize for WAFArray {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let dobj = self.as_object();
        dobj.serialize(serializer)
    }
}

impl serde::Serialize for WAFMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let dobj = self.as_object();
        dobj.serialize(serializer)
    }
}

#[cfg(feature = "serde_test")]
#[cfg(test)]
mod tests {
    use crate::object::{WAFArray, WAFMap, WAFObject, WAFObjectType};
    use crate::{waf_array, waf_map, waf_object};
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
}
