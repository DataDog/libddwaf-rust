#![doc = "Implementations of [serde::Deserialize] for [object::WAFObject](crate::object::WAFObject) and [object::WAFMap](crate::object::WAFMap)."]

use std::borrow::Cow;

use serde::{
    de::Error,
    ser::{SerializeMap, SerializeSeq},
    Deserializer,
};

use crate::object::{
    WAFArray, WAFBool, WAFFloat, WAFMap, WAFObject, WAFObjectType, WAFSigned, WAFString,
    WAFUnsigned,
};

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
        formatter.write_str(
            "a valid WAFObject (unsigned, signed, string, array, map, bool, float, or null)",
        )
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
        let mut vec = seq.size_hint().map(Vec::with_capacity).unwrap_or_default();
        while let Some(value) = seq.next_element()? {
            vec.push(value);
        }
        let mut res = WAFArray::new(vec.len().try_into().unwrap());
        for (i, v) in vec.into_iter().enumerate() {
            res[i] = v;
        }
        Ok(res.into())
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut vec: Vec<(Cow<'de, str>, WAFObject)> =
            map.size_hint().map(Vec::with_capacity).unwrap_or_default();
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

impl serde::Serialize for WAFObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.get_type() {
            WAFObjectType::Unsigned => {
                unsafe { self.as_type_unchecked::<WAFUnsigned>() }.serialize(serializer)
            }
            WAFObjectType::Signed => {
                unsafe { self.as_type_unchecked::<WAFSigned>() }.serialize(serializer)
            }
            WAFObjectType::Bool => {
                unsafe { self.as_type_unchecked::<WAFBool>() }.serialize(serializer)
            }
            WAFObjectType::Float => {
                unsafe { self.as_type_unchecked::<WAFFloat>() }.serialize(serializer)
            }
            WAFObjectType::String => {
                unsafe { self.as_type_unchecked::<WAFString>() }.serialize(serializer)
            }
            WAFObjectType::Array => {
                unsafe { self.as_type_unchecked::<WAFArray>() }.serialize(serializer)
            }
            WAFObjectType::Map => {
                unsafe { self.as_type_unchecked::<WAFMap>() }.serialize(serializer)
            }
            WAFObjectType::Null | WAFObjectType::Invalid => serializer.serialize_unit(),
        }
    }
}

impl serde::Serialize for WAFUnsigned {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u64(self.value())
    }
}

impl serde::Serialize for WAFSigned {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i64(self.value())
    }
}

impl serde::Serialize for WAFBool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bool(self.value())
    }
}

impl serde::Serialize for WAFFloat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_f64(self.value())
    }
}

impl serde::Serialize for WAFString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&String::from_utf8_lossy(self.bytes()))
    }
}

impl serde::Serialize for WAFArray {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq_serializer = serializer.serialize_seq(Some(self.len()))?;
        for value in self.iter() {
            seq_serializer.serialize_element(value)?;
        }
        seq_serializer.end()
    }
}

impl serde::Serialize for WAFMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map_serializer = serializer.serialize_map(Some(self.len()))?;
        for keyed_val in self.iter() {
            map_serializer
                .serialize_entry(&String::from_utf8_lossy(keyed_val.key()), keyed_val.inner())?;
        }
        map_serializer.end()
    }
}
