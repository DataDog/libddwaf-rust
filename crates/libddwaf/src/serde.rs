//! Implementations of [`serde::Deserialize`] for [`object::WafObject`](crate::object::WafObject) and
//! [`object::WafMap`](crate::object::WafMap).
//!
//! This module also provides [`Limits`] for applying constraints during deserialization,
//! similar to the PHP extension's `dd_mpack_limits` structure.

use std::cell::Cell;

use serde::{
    Deserializer,
    de::Error,
    ser::{SerializeMap, SerializeSeq},
};

use crate::object::{
    Keyed, WafArray, WafBool, WafFloat, WafMap, WafNull, WafObject, WafObjectType, WafSigned,
    WafString, WafUnsigned,
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
        Ok(WafObject::from(WafString::from(v)))
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
        let mut vec: Vec<(WafObject, WafObject)> =
            map.size_hint().map(Vec::with_capacity).unwrap_or_default();
        while let Some((key, value)) = map.next_entry::<WafObject, WafObject>()? {
            vec.push((key, value));
        }
        let mut res = WafMap::new(vec.len().try_into().map_err(A::Error::custom)?);
        for (i, (k, v)) in vec.into_iter().enumerate() {
            res[i] = Keyed::new(k, v);
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
        serializer.serialize_str(&String::from_utf8_lossy(self.as_bytes()))
    }
}

impl serde::Serialize for WafArray {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq_serializer = serializer.serialize_seq(Some(self.len() as usize))?;
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
        let mut map_serializer = serializer.serialize_map(Some(self.len() as usize))?;
        for keyed_val in self.iter() {
            // Key is serialized as WafObject; formats requiring string keys (e.g. JSON)
            // will error if the key is not a WafString
            map_serializer.serialize_entry(keyed_val.key(), keyed_val.value())?;
        }
        map_serializer.end()
    }
}

/// Default maximum string length (4096 bytes).
pub const DEFAULT_MAX_STRING_LENGTH: u32 = 4096;

/// Default maximum depth (21 levels).
pub const DEFAULT_MAX_DEPTH: usize = 21;

/// Default maximum elements (2048 elements).
pub const DEFAULT_MAX_ELEMENTS: usize = 2048;

/// Limits applied during deserialization to prevent excessive resource usage.
///
/// This is modeled after the PHP extension's `dd_mpack_limits` structure:
/// - `max_string_length`: Strings longer than this are truncated
/// - `max_depth`: Maximum nesting depth; deeper structures become null
/// - `max_elements`: Maximum total elements; excess elements are skipped
///
/// # Example
/// ```
/// use libddwaf::serde::Limits;
///
/// let limits = Limits::default();
/// assert_eq!(limits.max_string_length, 4096);
/// assert_eq!(limits.max_depth, 21);
/// assert_eq!(limits.max_elements, 2048);
///
/// // Create custom limits
/// let custom = Limits {
///     max_string_length: 1024,
///     max_depth: 10,
///     max_elements: 100,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct Limits {
    pub max_string_length: u32,
    pub max_depth: usize,
    pub max_elements: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_string_length: DEFAULT_MAX_STRING_LENGTH,
            max_depth: DEFAULT_MAX_DEPTH,
            max_elements: DEFAULT_MAX_ELEMENTS,
        }
    }
}

/// The result of deserializing with limits.
#[derive(Debug)]
pub struct LimitedResult<T> {
    /// The deserialized value.
    pub value: T,
    /// Whether any limits were reached during deserialization.
    pub truncated: bool,
}

/// Deserialize a [`WafObject`] from a deserializer with the specified limits.
///
/// Returns a [`LimitedResult`] containing the deserialized value and whether
/// truncation occurred.
///
/// # Example
/// ```
/// use libddwaf::serde::{Limits, deserialize_with_limits};
/// use libddwaf::object::WafObject;
///
/// let json = r#"{"key": "value"}"#;
/// let limits = Limits {
///     max_string_length: 3,
///     max_depth: 10,
///     max_elements: 100,
/// };
/// let mut deserializer = serde_json::Deserializer::from_str(json);
/// let result = deserialize_with_limits(&mut deserializer, &limits).unwrap();
///
/// assert!(result.truncated); // "value" was truncated to "val"
/// ```
/// # Errors
/// Returns an error if the deserializer returns an error.
pub fn deserialize_with_limits<'de, D>(
    deserializer: D,
    limits: &Limits,
) -> Result<LimitedResult<WafObject>, D::Error>
where
    D: Deserializer<'de>,
{
    let state = LimitedState::new(limits);
    let visitor = LimitedVisitor { state: &state };
    let value = deserializer.deserialize_any(visitor)?;
    Ok(LimitedResult {
        value,
        truncated: state.truncated.get(),
    })
}

struct LimitedState<'a> {
    limits: &'a Limits,
    depth_remaining: Cell<usize>,
    elements_remaining: Cell<usize>,
    truncated: Cell<bool>,
}

impl<'a> LimitedState<'a> {
    fn new(limits: &'a Limits) -> Self {
        Self {
            limits,
            depth_remaining: Cell::new(limits.max_depth),
            elements_remaining: Cell::new(limits.max_elements),
            truncated: Cell::new(false),
        }
    }

    /// Consumes one element from the remaining count.
    /// Returns true if the element can be processed, false if limit reached.
    fn consume_element(&self) -> bool {
        let remaining = self.elements_remaining.get();
        if remaining == 0 {
            self.truncated.set(true);
            false
        } else {
            self.elements_remaining.set(remaining - 1);
            true
        }
    }

    fn can_descend(&self) -> bool {
        self.depth_remaining.get() > 0
    }

    fn enter_depth(&self) {
        let depth = self.depth_remaining.get();
        if depth > 0 {
            self.depth_remaining.set(depth - 1);
        }
    }

    fn exit_depth(&self) {
        self.depth_remaining.set(self.depth_remaining.get() + 1);
    }

    fn truncate_string<'b>(&self, s: &'b str) -> &'b str {
        if s.len() > self.limits.max_string_length as usize {
            self.truncated.set(true);
            // Find a valid UTF-8 boundary
            let mut end = self.limits.max_string_length as usize;
            while end > 0 && !s.is_char_boundary(end) {
                end -= 1;
            }
            &s[..end]
        } else {
            s
        }
    }

    fn truncate_bytes<'b>(&self, b: &'b [u8]) -> &'b [u8] {
        if b.len() > self.limits.max_string_length as usize {
            self.truncated.set(true);
            &b[..self.limits.max_string_length as usize]
        } else {
            b
        }
    }
}

/// A visitor that applies limits during deserialization.
struct LimitedVisitor<'a> {
    state: &'a LimitedState<'a>,
}

impl<'de, 'a> serde::de::Visitor<'de> for LimitedVisitor<'a> {
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
        if !self.state.consume_element() {
            return Ok(WafNull::new().into());
        }
        Ok(WafObject::from(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if !self.state.consume_element() {
            return Ok(WafNull::new().into());
        }
        Ok(WafObject::from(v))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if !self.state.consume_element() {
            return Ok(WafNull::new().into());
        }
        Ok(WafObject::from(v))
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if !self.state.consume_element() {
            return Ok(WafNull::new().into());
        }
        Ok(WafObject::from(v))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if !self.state.consume_element() {
            return Ok(WafNull::new().into());
        }
        Ok(WafObject::from(()))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if !self.state.consume_element() {
            return Ok(WafNull::new().into());
        }
        let truncated = self.state.truncate_string(v);
        Ok(WafObject::from(truncated))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if !self.state.consume_element() {
            return Ok(WafNull::new().into());
        }
        let truncated = self.state.truncate_bytes(v);
        Ok(WafObject::from(WafString::from(truncated)))
    }

    #[allow(clippy::cast_possible_truncation)]
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        if !self.state.can_descend() {
            self.state.truncated.set(true);
            // Drain the sequence
            while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
            return Ok(WafNull::new().into());
        }

        // Consume element for the array itself
        if !self.state.consume_element() {
            // Drain the sequence
            while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
            return Ok(WafNull::new().into());
        }

        self.state.enter_depth();

        let mut vec = seq.size_hint().map(Vec::with_capacity).unwrap_or_default();
        while self.state.elements_remaining.get() > 0 {
            match seq.next_element_seed(LimitedSeed { state: self.state })? {
                Some(value) => vec.push(value),
                None => break,
            }
        }

        // If there are remaining elements, drain them and mark as truncated
        if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
            self.state.truncated.set(true);
            while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        }

        self.state.exit_depth();

        let len = vec.len().min(u16::MAX as usize);
        let mut res = WafArray::new(len as u16);
        for (i, v) in vec.into_iter().take(len).enumerate() {
            res[i] = v;
        }
        Ok(res.into())
    }

    #[allow(clippy::cast_possible_truncation)]
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        // Check if we can descend
        if !self.state.can_descend() {
            self.state.truncated.set(true);
            // Drain the map
            while map
                .next_entry::<serde::de::IgnoredAny, serde::de::IgnoredAny>()?
                .is_some()
            {}
            return Ok(WafNull::new().into());
        }

        // Consume element for the map itself
        if !self.state.consume_element() {
            // Drain the map
            while map
                .next_entry::<serde::de::IgnoredAny, serde::de::IgnoredAny>()?
                .is_some()
            {}
            return Ok(WafNull::new().into());
        }

        self.state.enter_depth();

        let mut vec: Vec<Keyed<WafObject>> =
            map.size_hint().map(Vec::with_capacity).unwrap_or_default();

        while self.state.elements_remaining.get() > 0 {
            match map.next_entry_seed(LimitedSeed { state: self.state }, LimitedSeed {
                state: self.state,
            })? {
                Some(pair @ (_, _)) => vec.push(pair.into()),
                None => break,
            }
        }

        // If there are remaining entries, drain them and mark as truncated
        if map
            .next_entry::<serde::de::IgnoredAny, serde::de::IgnoredAny>()?
            .is_some()
        {
            self.state.truncated.set(true);
            while map
                .next_entry::<serde::de::IgnoredAny, serde::de::IgnoredAny>()?
                .is_some()
            {}
        }

        self.state.exit_depth();

        let len = vec.len().min(u16::MAX as usize);
        let mut res = WafMap::new(len as u16);
        for (i, keyed) in vec.into_iter().take(len).enumerate() {
            res[i] = keyed;
        }
        Ok(res.into())
    }
}

/// A `DeserializeSeed` that uses the limited visitor.
struct LimitedSeed<'a> {
    state: &'a LimitedState<'a>,
}

impl<'de, 'a> serde::de::DeserializeSeed<'de> for LimitedSeed<'a>
where
    'de: 'a,
{
    type Value = WafObject;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let visitor = LimitedVisitor { state: self.state };
        deserializer.deserialize_any(visitor)
    }
}
