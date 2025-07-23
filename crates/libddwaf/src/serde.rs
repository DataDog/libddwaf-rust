//! Implementations of [`serde::Deserialize`] for [`object::WafObject`](crate::object::WafObject) and
//! [`object::WafMap`](crate::object::WafMap).

use std::borrow::Cow;

use serde::{
    de::Error,
    ser::{SerializeMap, SerializeSeq},
    Deserializer,
};

use crate::object::{
    WafArray, WafBool, WafFloat, WafMap, WafObject, WafObjectType, WafSigned, WafString,
    WafUnsigned,
};

impl<'de> serde::Deserialize<'de> for WafObject {
    fn deserialize<D>(deserializer: D) -> Result<WafObject, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(Visitor)
    }
}

impl<'de> serde::Deserialize<'de> for WafMap {
    fn deserialize<D>(deserializer: D) -> Result<WafMap, D::Error>
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
    type Value = WafObject;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(
            "a valid WafObject (unsigned, signed, string, array, map, bool, float, or null)",
        )
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(WafObject::from(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(WafObject::from(v))
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(WafObject::from(v))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(WafObject::from(()))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(WafObject::from(v))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(WafObject::from(WafString::new(v)))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut vec = seq.size_hint().map(Vec::with_capacity).unwrap_or_default();
        while let Some(value) = seq.next_element()? {
            vec.push(value);
        }
        let mut res = WafArray::new(vec.len().try_into().unwrap());
        for (i, v) in vec.into_iter().enumerate() {
            res[i] = v;
        }
        Ok(res.into())
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut vec: Vec<(Cow<'de, str>, WafObject)> =
            map.size_hint().map(Vec::with_capacity).unwrap_or_default();
        while let Some((key, value)) = map.next_entry::<Cow<'de, str>, WafObject>()? {
            vec.push((key, value));
        }
        let mut res = WafMap::new(vec.len().try_into().unwrap());
        for (i, (k, v)) in vec.into_iter().enumerate() {
            let key_str: &str = &k;
            res[i] = (key_str, v).into();
        }
        Ok(res.into())
    }
}

impl serde::Serialize for WafObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.get_type() {
            WafObjectType::Unsigned => {
                unsafe { self.as_type_unchecked::<WafUnsigned>() }.serialize(serializer)
            }
            WafObjectType::Signed => {
                unsafe { self.as_type_unchecked::<WafSigned>() }.serialize(serializer)
            }
            WafObjectType::Bool => {
                unsafe { self.as_type_unchecked::<WafBool>() }.serialize(serializer)
            }
            WafObjectType::Float => {
                unsafe { self.as_type_unchecked::<WafFloat>() }.serialize(serializer)
            }
            WafObjectType::String => {
                unsafe { self.as_type_unchecked::<WafString>() }.serialize(serializer)
            }
            WafObjectType::Array => {
                unsafe { self.as_type_unchecked::<WafArray>() }.serialize(serializer)
            }
            WafObjectType::Map => {
                unsafe { self.as_type_unchecked::<WafMap>() }.serialize(serializer)
            }
            WafObjectType::Null | WafObjectType::Invalid => serializer.serialize_unit(),
        }
    }
}

impl serde::Serialize for WafUnsigned {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u64(self.value())
    }
}

impl serde::Serialize for WafSigned {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i64(self.value())
    }
}

impl serde::Serialize for WafBool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bool(self.value())
    }
}

impl serde::Serialize for WafFloat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_f64(self.value())
    }
}

impl serde::Serialize for WafString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&String::from_utf8_lossy(self.bytes()))
    }
}

impl serde::Serialize for WafArray {
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

impl serde::Serialize for WafMap {
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
