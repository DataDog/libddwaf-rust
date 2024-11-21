use std::borrow::Cow;

use serde::{
    de::Error,
    ser::{SerializeMap, SerializeSeq},
    Deserializer,
};

use crate::{
    CommonDdwafObj, CommonDdwafObjMut, DdwafObj, DdwafObjArray, DdwafObjMap, DdwafObjString,
    DdwafObjType,
};

impl<'de> serde::Deserialize<'de> for DdwafObj {
    fn deserialize<D>(deserializer: D) -> Result<DdwafObj, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(DdwafObjVisitor)
    }
}

impl<'de> serde::Deserialize<'de> for DdwafObjMap {
    fn deserialize<D>(deserializer: D) -> Result<DdwafObjMap, D::Error>
    where
        D: Deserializer<'de>,
    {
        let dobj = deserializer.deserialize_any(DdwafObjVisitor)?;
        dobj.try_into()
            .map_err(|_| serde::de::Error::custom("invalid type: not a map"))
    }
}

struct DdwafObjVisitor;

impl<'de> serde::de::Visitor<'de> for DdwafObjVisitor {
    type Value = DdwafObj;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an unsigned, a map, or an array")
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(DdwafObj::from(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(DdwafObj::from(v))
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(DdwafObj::from(v))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(DdwafObj::from(()))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(DdwafObj::from(v))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(DdwafObj::from(DdwafObjString::new(v)))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        if let Some(size) = seq.size_hint() {
            let mut arr = DdwafObjArray::new(size.try_into().unwrap());
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
            let mut vec = Vec::<DdwafObj>::new();
            while let Some(value) = seq.next_element()? {
                vec.push(value);
            }
            let mut res = DdwafObjArray::new(vec.len().try_into().unwrap());
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
            let mut dmap = DdwafObjMap::new(size.try_into().unwrap());
            let mut i: usize = 0;
            while let Some((key, value)) = map.next_entry::<Cow<'de, str>, DdwafObj>()? {
                dmap[i] = value;
                dmap[i].set_key_str(&key);
                i += 1;
            }
            if i != size {
                return Err(serde::de::Error::custom("size hint was wrong"));
            }
            Ok(dmap.into())
        } else {
            let mut vec = Vec::<(Cow<'de, str>, DdwafObj)>::new();
            while let Some((key, value)) = map.next_entry::<Cow<'de, str>, DdwafObj>()? {
                vec.push((key, value));
            }
            let mut res = DdwafObjMap::new(vec.len().try_into().unwrap());
            for (i, (k, v)) in vec.into_iter().enumerate() {
                res[i] = v;
                res[i].set_key_str(&k);
            }
            Ok(res.into())
        }
    }
}

impl serde::Serialize for DdwafObj {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.get_type() {
            DdwafObjType::Unsigned => serializer.serialize_u64(self.to_u64().unwrap()),
            DdwafObjType::Signed => serializer.serialize_i64(self.to_i64().unwrap()),
            DdwafObjType::Bool => serializer.serialize_bool(self.to_bool().unwrap()),
            DdwafObjType::Float => serializer.serialize_f64(self.to_f64().unwrap()),
            DdwafObjType::String => serializer.serialize_str(&String::from_utf8_lossy(
                self.as_type::<DdwafObjString>().unwrap().as_slice(),
            )),
            DdwafObjType::Null => serializer.serialize_unit(),
            DdwafObjType::Invalid => serializer.serialize_unit(),
            DdwafObjType::Array => {
                let array = self.as_type::<DdwafObjArray>().unwrap();
                let mut seq_serializer = serializer.serialize_seq(Some(array.len()))?;
                for value in array {
                    seq_serializer.serialize_element(value)?;
                }
                seq_serializer.end()
            }
            DdwafObjType::Map => {
                let map = self.as_type::<DdwafObjMap>().unwrap();
                let mut map_serializer = serializer.serialize_map(Some(map.len()))?;
                for (k, v) in map {
                    map_serializer.serialize_entry(&String::from_utf8_lossy(k), v)?;
                }
                map_serializer.end()
            }
        }
    }
}

impl serde::Serialize for DdwafObjMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let dobj: &DdwafObj = unsafe { &*(self as *const _ as *const DdwafObj) };
        dobj.serialize(serializer)
    }
}

#[cfg(feature = "serde_test")]
mod tests {
    #[allow(unused_imports)]
    use crate::CommonDdwafObjMut;
    #[allow(unused_imports)]
    use crate::{ddwaf_obj, ddwaf_obj_array, ddwaf_obj_map};
    #[allow(unused_imports)]
    use crate::{CommonDdwafObj, DdwafObj, DdwafObjArray, DdwafObjMap, DdwafObjType};
    #[allow(unused_imports)]
    use serde_json::from_str;

    #[test]
    fn sample_json_deserialization() {
        // Test for a simple unsigned integer
        let json = "42";
        let ddwaf_obj: DdwafObj = from_str(json).expect("Failed to deserialize u64");
        assert_eq!(ddwaf_obj.to_u64().unwrap(), 42);

        // Test for a signed integer
        let json = "-42";
        let ddwaf_obj: DdwafObj = from_str(json).expect("Failed to deserialize i64");
        assert_eq!(ddwaf_obj.to_i64().unwrap(), -42);

        // Test for a boolean
        let json = "true";
        let ddwaf_obj: DdwafObj = from_str(json).expect("Failed to deserialize bool");
        assert_eq!(ddwaf_obj.to_bool().unwrap(), true);

        // Test for null
        let json = "null";
        let ddwaf_obj: DdwafObj = from_str(json).expect("Failed to deserialize null");
        assert_eq!(ddwaf_obj.get_type(), DdwafObjType::Null);

        // Test for a string
        let json = "\"hello\"";
        let ddwaf_obj: DdwafObj = from_str(json).expect("Failed to deserialize string");
        assert_eq!(ddwaf_obj.to_str().unwrap(), "hello");

        // Test for an array
        let json = "[1, 2, 3]";
        let array: DdwafObjArray = from_str::<DdwafObj>(json)
            .expect("Failed to deserialize array")
            .try_into()
            .unwrap();
        assert_eq!(array.len(), 3);
        assert_eq!(array[0].to_u64().unwrap(), 1);
        assert_eq!(array[1].to_u64().unwrap(), 2);
        assert_eq!(array[2].to_u64().unwrap(), 3);

        // Test for a map
        let json = "{\"key1\": \"value1\", \"key2\": 42}";
        let map: DdwafObjMap = from_str::<DdwafObj>(json)
            .expect("Failed to deserialize map")
            .try_into()
            .unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map.gets("key1").unwrap().to_str().unwrap(), "value1");
        assert_eq!(map.gets("key2").unwrap().to_u64().unwrap(), 42);
    }

    #[test]
    fn map_deserialization_ok() {
        let json = "{\"key1\": 42, \"key2\": 43}";
        let map: DdwafObjMap = from_str::<DdwafObjMap>(json).expect("Failed to deserialize map");
        assert_eq!(map.len(), 2);
        assert_eq!(map.gets("key1").unwrap().to_u64().unwrap(), 42);
        assert_eq!(map.gets("key2").unwrap().to_u64().unwrap(), 43);
    }

    #[test]
    fn map_deserialization_wrong_type() {
        let json = "[42]";
        let maybe_map = from_str::<DdwafObjMap>(json);

        assert!(maybe_map.is_err());
        assert_eq!(
            maybe_map.err().unwrap().to_string(),
            "invalid type: not a map"
        );
    }

    #[test]
    fn sample_json_serialization() {
        let root = ddwaf_obj_array!(
            "Hello, world!",
            123_u64,
            ddwaf_obj_map!(
                ("key 1", "value 1"),
                ("key 2", -2_i64),
                ("key 3", 2_u64),
                ("key 4", 5.2),
                ("key 5", ddwaf_obj!(null)),
                ("key 5", ddwaf_obj!(true)),
            ),
            ddwaf_obj_array!(),
            ddwaf_obj_map!(),
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
