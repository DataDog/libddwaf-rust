#![doc = "Data model for exchanging data with the in-app WAF."]

use std::alloc::Layout;
use std::fmt;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::ptr::null_mut;

use crate::bindings;

mod iter;
#[doc(inline)]
pub use iter::*;

/// Identifies the type of the value stored in a [`WAFObject`].
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum WAFObjectType {
    /// An invalid value. This can be used as a placeholder to retain the key
    /// associated with an object that was only aprtially encoded.
    Invalid,
    /// A signed integer with 64-bit precision.
    Signed,
    /// An unsigned integer with 64-bit precision.
    Unsigned,
    /// A string value.
    String,
    /// An array of [`WAFObject`]s.
    Array,
    /// A map of string-keyed [`WAFObject`]s.
    Map,
    /// A boolean value.
    Bool,
    /// A floating point value (64-bit precision).
    Float,
    /// The null value.
    Null,
}
impl WAFObjectType {
    /// Returns the raw [`bindings::DDWAF_OBJ_TYPE`] value corresponding to this [`WAFObjectType`].
    const fn as_raw(self) -> bindings::DDWAF_OBJ_TYPE {
        match self {
            WAFObjectType::Invalid => bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_INVALID,
            WAFObjectType::Signed => bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_SIGNED,
            WAFObjectType::Unsigned => bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_UNSIGNED,
            WAFObjectType::String => bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_STRING,
            WAFObjectType::Array => bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_ARRAY,
            WAFObjectType::Map => bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_MAP,
            WAFObjectType::Bool => bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_BOOL,
            WAFObjectType::Float => bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_FLOAT,
            WAFObjectType::Null => bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_NULL,
        }
    }
}
impl TryFrom<bindings::DDWAF_OBJ_TYPE> for WAFObjectType {
    type Error = UnknownObjectTypeError;
    fn try_from(value: bindings::DDWAF_OBJ_TYPE) -> Result<Self, UnknownObjectTypeError> {
        match value {
            bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_INVALID => Ok(WAFObjectType::Invalid),
            bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_SIGNED => Ok(WAFObjectType::Signed),
            bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_UNSIGNED => Ok(WAFObjectType::Unsigned),
            bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_STRING => Ok(WAFObjectType::String),
            bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_ARRAY => Ok(WAFObjectType::Array),
            bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_MAP => Ok(WAFObjectType::Map),
            bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_BOOL => Ok(WAFObjectType::Bool),
            bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_FLOAT => Ok(WAFObjectType::Float),
            bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_NULL => Ok(WAFObjectType::Null),
            unknown => Err(UnknownObjectTypeError(unknown)),
        }
    }
}

/// The error that is returned when a [`WAFObject`] does not have a known, valid [`WAFObjectType`].
#[derive(Copy, Clone, Debug)]
pub struct UnknownObjectTypeError(bindings::DDWAF_OBJ_TYPE);
impl std::error::Error for UnknownObjectTypeError {}
impl std::fmt::Display for UnknownObjectTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown object type: {:?}", self.0)
    }
}

/// The error that is returned when a [`WAFObject`] does not have the expected [`WAFObjectType`].
#[derive(Copy, Clone, Debug)]
pub struct ObjectTypeError {
    pub expected: WAFObjectType,
    pub actual: WAFObjectType,
}
impl std::error::Error for ObjectTypeError {}
impl std::fmt::Display for ObjectTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid object type (expected {:?}, got {:?})",
            self.expected, self.actual
        )
    }
}

/// This trait allow obtaining direct mutable access to the underlying memory
/// backing a [`WAFObject`] or [`TypedWAFObject`] value.
#[doc(hidden)]
pub trait AsRawMutObject: crate::private::Sealed + AsRef<bindings::ddwaf_object> {
    /// Obtains a mutable reference to the underlying raw `ddwaf_object`]`.
    ///
    /// # Safety
    /// The caller must ensure that:
    /// - it does not change the [`bindings::ddwaf_object::type_`] field,
    /// - it does not change the pointers to values that don't outlive the [`bindings::ddwaf_object`]
    ///   itself, or whose memory cannot be recclaimed byt the destructor in the same way as the
    ///   current value,
    /// - it does not change the lengths in such a way that the object is no longer valid.
    ///
    /// Additionally, the caller would incur a memory leak if it dropped the value through the
    /// returned reference (e.g, by calling [`std::mem::replace`]), since [`bindings::ddwaf_object`] is
    /// not [`Drop`] (see swapped destructors in
    /// [this playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=aeea4aba8f960bf0c63f6185f016a94d).
    #[doc(hidden)]
    unsafe fn as_raw_mut(&mut self) -> &mut bindings::ddwaf_object;
}

/// This trait is implemented by type-safe interfaces to the [`WAFObject`], with
/// one implementation for each [`WAFObjectType`].
pub trait TypedWAFObject: AsRawMutObject {
    /// The associated [`WAFObjectType`] constant corresponding to the typed
    /// object's type discriminator.
    const TYPE: WAFObjectType;
}

/// The low-level representation of an arbitrary WAF object.
///
/// It is usually converted to a [`TypedWAFObject`] by calling [`WAFObject::as_type`].
#[derive(Default)]
#[repr(transparent)]
pub struct WAFObject {
    raw: bindings::ddwaf_object,
}
impl WAFObject {
    /// Returns the [`WAFObjectType`] of the underlying value.
    ///
    /// Returns [`WAFObjectType::Invalid`] if the underlying value's type is not set to a
    /// known, valid [`WAFObjectType`] value.
    #[must_use]
    pub fn get_type(&self) -> WAFObjectType {
        self.as_ref()
            .type_
            .try_into()
            .unwrap_or(WAFObjectType::Invalid)
    }

    /// Returns a reference to this value as a `T` if its type corresponds.
    #[must_use]
    pub fn as_type<T: TypedWAFObject>(&self) -> Option<&T> {
        if self.get_type() == T::TYPE {
            Some(unsafe { self.as_type_unchecked::<T>() })
        } else {
            None
        }
    }

    /// Returns a reference to this value as a `T`.
    ///
    /// # Safety
    /// The caller must ensure that the [`WAFObject`] can be accurately represented by `T`.
    unsafe fn as_type_unchecked<T: TypedWAFObject>(&self) -> &T {
        self.as_ref().unchecked_as_ref::<T>()
    }

    /// Returns a mutable reference to this value as a `T` if its type corresponds.
    pub fn as_type_mut<T: TypedWAFObject>(&mut self) -> Option<&mut T> {
        if self.get_type() == T::TYPE {
            Some(unsafe { self.as_raw_mut().unchecked_as_ref_mut::<T>() })
        } else {
            None
        }
    }

    /// Returns true if this [`WAFObject`] is not [`WAFObjectType::Invalid`], meaning it can be
    /// converted to one of the [`TypedWAFObject`] implementations.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.get_type() != WAFObjectType::Invalid
    }

    /// Returns the value of this [`WAFObject`] as a [`u64`] if its type is [`WAFObjectType::Unsigned`].
    #[must_use]
    pub fn to_u64(&self) -> Option<u64> {
        self.as_type::<WAFUnsigned>().map(WAFUnsigned::value)
    }

    /// Returns the value of this [`WAFObject`] as a [`i64`] if its type is [`WAFObjectType::Signed`] (or
    /// [`WAFObjectType::Unsigned`] with a value that can be represented as an [`i64`]).
    #[must_use]
    pub fn to_i64(&self) -> Option<i64> {
        match self.get_type() {
            WAFObjectType::Unsigned => {
                let obj: &WAFUnsigned = unsafe { self.as_type_unchecked() };
                obj.value().try_into().ok()
            }
            WAFObjectType::Signed => {
                let obj: &WAFSigned = unsafe { self.as_type_unchecked() };
                Some(obj.value())
            }
            _ => None,
        }
    }

    /// Returns the value of this [`WAFObject`] as a [`f64`] if its type is [`WAFObjectType::Float`].
    #[must_use]
    pub fn to_f64(&self) -> Option<f64> {
        self.as_type::<WAFFloat>().map(WAFFloat::value)
    }

    /// Returns the value of this [`WAFObject`] as a [`bool`] if its type is [`WAFObjectType::Bool`].
    #[must_use]
    pub fn to_bool(&self) -> Option<bool> {
        self.as_type::<WAFBool>().map(WAFBool::value)
    }

    /// Returns the value of this [`WAFObject`] as a [`&str`] if its type is [`WAFObjectType::String`],
    /// and the value is valid UTF-8.
    #[must_use]
    pub fn to_str(&self) -> Option<&str> {
        self.as_type::<WAFString>().and_then(|x| x.as_str().ok())
    }
}
impl AsRef<bindings::ddwaf_object> for WAFObject {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        &self.raw
    }
}
impl AsRawMutObject for WAFObject {
    unsafe fn as_raw_mut(&mut self) -> &mut bindings::ddwaf_object {
        &mut self.raw
    }
}
impl fmt::Debug for WAFObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.get_type() {
            WAFObjectType::Invalid => write!(f, "WAFInvalid"),
            WAFObjectType::Unsigned => {
                let obj: &WAFUnsigned = self.as_type().unwrap();
                obj.fmt(f)
            }
            WAFObjectType::Signed => {
                let obj: &WAFSigned = self.as_type().unwrap();
                obj.fmt(f)
            }
            WAFObjectType::Float => {
                let obj: &WAFFloat = self.as_type().unwrap();
                obj.fmt(f)
            }
            WAFObjectType::Bool => {
                let obj: &WAFBool = self.as_type().unwrap();
                obj.fmt(f)
            }
            WAFObjectType::Null => {
                let obj: &WAFNull = self.as_type().unwrap();
                obj.fmt(f)
            }
            WAFObjectType::String => {
                let obj: &WAFString = self.as_type().unwrap();
                obj.fmt(f)
            }
            WAFObjectType::Array => {
                let obj: &WAFArray = self.as_type().unwrap();
                obj.fmt(f)
            }
            WAFObjectType::Map => {
                let obj: &WAFMap = self.as_type().unwrap();
                obj.fmt(f)
            }
        }
    }
}
impl Drop for WAFObject {
    fn drop(&mut self) {
        unsafe { self.raw.drop_object() }
    }
}
impl From<u64> for WAFObject {
    fn from(value: u64) -> Self {
        WAFUnsigned::new(value).into()
    }
}
impl From<u32> for WAFObject {
    fn from(value: u32) -> Self {
        WAFUnsigned::new(value.into()).into()
    }
}
impl From<i64> for WAFObject {
    fn from(value: i64) -> Self {
        WAFSigned::new(value).into()
    }
}
impl From<i32> for WAFObject {
    fn from(value: i32) -> Self {
        WAFSigned::new(value.into()).into()
    }
}
impl From<f64> for WAFObject {
    fn from(value: f64) -> Self {
        WAFFloat::new(value).into()
    }
}
impl From<bool> for WAFObject {
    fn from(value: bool) -> Self {
        WAFBool::new(value).into()
    }
}
impl From<&str> for WAFObject {
    fn from(value: &str) -> Self {
        WAFString::new(value).into()
    }
}
impl From<&[u8]> for WAFObject {
    fn from(value: &[u8]) -> Self {
        WAFString::new(value).into()
    }
}
impl From<()> for WAFObject {
    fn from((): ()) -> Self {
        WAFNull::new().into()
    }
}
impl<T: TypedWAFObject> From<T> for WAFObject {
    fn from(value: T) -> Self {
        let res = Self {
            raw: *value.as_ref(),
        };
        std::mem::forget(value);
        res
    }
}
impl crate::private::Sealed for WAFObject {}

/// A WAF-owned [`WAFObject`] or [`TypedWAFObject`] value.
///
/// This has different [`Drop`] behavior than a rust-owned [`WAFObject`] value.
pub struct WAFOwned<T: AsRawMutObject> {
    inner: std::mem::ManuallyDrop<T>,
}
impl<T: AsRawMutObject + Default> Default for WAFOwned<T> {
    fn default() -> Self {
        Self {
            inner: std::mem::ManuallyDrop::new(Default::default()),
        }
    }
}
impl<T: AsRawMutObject> Deref for WAFOwned<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl<T: AsRawMutObject> DerefMut for WAFOwned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl<T: AsRawMutObject> Drop for WAFOwned<T> {
    fn drop(&mut self) {
        unsafe { bindings::ddwaf_object_free(self.inner.as_raw_mut()) };
    }
}

/// Allocates memory for the given [`Layout`], calling [`std::alloc::handle_alloc_error`] if the
/// allocation failed.
///
/// # Safety
/// The requirements as for [`std::alloc::alloc`] apply.
unsafe fn no_fail_alloc(layout: Layout) -> *mut u8 {
    if layout.size() == 0 {
        return null_mut();
    }
    let ptr = unsafe { std::alloc::alloc(layout) };
    if ptr.is_null() {
        std::alloc::handle_alloc_error(layout);
    }
    ptr
}

macro_rules! typed_object {
    ($type:expr => $name:ident $({ $($impl:tt)* })?) => {
        #[doc = concat!("The WAF object representation of a value of type [", stringify!($type), "]")]
        #[repr(transparent)]
        pub struct $name {
            raw: $crate::bindings::ddwaf_object,
        }
        impl $name {
            #[doc = concat!("Returns true if this [", stringify!($name), "] is indeed [", stringify!($type), "].")]
            #[must_use]
            pub const fn is_valid(&self) -> bool {
                self.raw.type_ == $type.as_raw()
            }

            /// Returns a reference to this value as a [`WAFObject`].
            #[must_use]
            pub fn as_object(&self) -> &WAFObject{
                let obj: &bindings::ddwaf_object = self.as_ref();
                obj.as_object_ref()
            }
            $(
            $($impl)*)?
        }
        impl AsRef<$crate::bindings::ddwaf_object> for $name {
            fn as_ref(&self) -> &$crate::bindings::ddwaf_object {
                &self.raw
            }
        }
        impl AsRawMutObject for $name {
            unsafe fn as_raw_mut(&mut self) -> &mut $crate::bindings::ddwaf_object {
                &mut self.raw
            }
        }
        impl Default for $name {
            fn default() -> Self {
                Self {
                    raw: $crate::bindings::ddwaf_object {
                        type_: $type.as_raw(),
                        ..Default::default()
                    },
                }
            }
        }
        impl TryFrom<WAFObject> for $name {
            type Error = ObjectTypeError;
            fn try_from(obj: WAFObject) -> Result<Self, Self::Error> {
                if obj.get_type() != Self::TYPE {
                    return Err(ObjectTypeError {
                        expected: $type,
                        actual: obj.get_type(),
                    });
                }
                let res = Self { raw: obj.raw };
                std::mem::forget(obj);
                Ok(res)
            }
        }
        impl crate::private::Sealed for $name {}
        impl TypedWAFObject for $name {
            const TYPE: WAFObjectType = $type;
        }
    };
}

typed_object!(WAFObjectType::Invalid => WAFInvalid);
typed_object!(WAFObjectType::Signed => WAFSigned {
    /// Creates a new [WAFSigned] with the provided value.
    #[must_use]
    pub const fn new(val: i64) -> Self {
        Self {
            raw: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_SIGNED,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 { intValue: val },
                nbEntries: 0,
                parameterName: null_mut(),
                parameterNameLength: 0,
            }
        }
    }

    /// Returns the value of this [WAFSigned].
    #[must_use]
    pub const fn value(&self) -> i64 {
        unsafe { self.raw.__bindgen_anon_1.intValue }
    }
});
typed_object!(WAFObjectType::Unsigned => WAFUnsigned {
    /// Creates a new [WAFUnsigned] with the provided value.
    #[must_use]
    pub const fn new (val: u64) -> Self {
        Self {
            raw: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_UNSIGNED,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 { uintValue: val },
                nbEntries: 0,
                parameterName: null_mut(),
                parameterNameLength: 0,
            }
        }
    }

    /// Retuns the value of this [WAFUnsigned].
    #[must_use]
    pub const fn value(&self) -> u64 {
        unsafe { self.raw.__bindgen_anon_1.uintValue }
    }
});
typed_object!(WAFObjectType::String => WAFString {
    /// Creates a new [WAFString] with the provided value.
    pub fn new(val: impl AsRef<[u8]>) -> Self {
        let val = val.as_ref();
        let ptr = if val.is_empty() {
            null_mut()
        } else {
            let b: Box<[u8]> = val.into();
            Box::into_raw(b).cast()
        };
        Self {
            raw: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_STRING,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 {
                    stringValue: ptr,
                },
                nbEntries: val.len() as u64,
                ..Default::default()
            },
        }
    }

    /// Returns the length of this [WAFString], in bytes.
    ///
    /// # Panics
    /// Panics if the string is larger than [`usize::MAX`] bytes. This can only happen on
    /// platforms where [`usize`] is 32-bit wide.
    #[must_use]
    pub fn len(&self) -> usize {
        usize::try_from(self.raw.nbEntries).expect("string is too large for this platform")
    }

    /// Returns true if this [WAFString] is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a slice of the bytes from this [WAFString].
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        debug_assert_eq!(self.raw.type_, bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_STRING);
        unsafe {
            std::slice::from_raw_parts(
                self.raw.__bindgen_anon_1.stringValue.cast(),
                self.len(),
            )
        }
    }

    /// Returns a string slice from this [WAFString].
    ///
    /// # Errors
    /// Returns an error if the underlying data is not a valid UTF-8 string, under the same conditions as
    /// [`std::str::from_utf8`].
    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.bytes())
    }
});
typed_object!(WAFObjectType::Array => WAFArray {
    /// Creates a new [WAFArray] with the provided size. All values in the array are initialized
    /// to an invalid [WAFObject] instance.
    ///
    /// # Panics
    /// Panics when the provided `nb_entries` is larger than what the current platform can represent in an [`usize`].
    /// This can only happen on platforms where [`usize`] is 32-bit wide.
    #[must_use]
    pub fn new(nb_entries: u64) -> Self {
        let size = usize::try_from(nb_entries).expect("size is too large for this platform");
        let layout = Layout::array::<bindings::ddwaf_object>(size).unwrap();
        let array = unsafe { no_fail_alloc(layout).cast() };
        unsafe { std::ptr::write_bytes(array, 0, size)};
        Self {
            raw: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_ARRAY,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 { array },
                nbEntries: nb_entries,
                ..Default::default()
            }
        }
    }

    /// Returns the length of this [WAFArray].
    ///
    /// # Panics
    /// Panics if the array is larger than [`usize::MAX`] elements. This can only happen on
    /// platforms where [`usize`] is 32-bit wide.
    #[must_use]
    pub fn len(&self) -> usize {
        usize::try_from(self.raw.nbEntries).expect("array is too large for this platform")
    }

    /// Returns true if this [WAFArray] is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the [`Keyed<WAFObject>`]s in this [WAFMap].
    pub fn iter(&self) -> impl Iterator<Item = &WAFObject> {
        let slice : &[WAFObject] = self.as_ref();
        slice.iter()
    }

    /// Returns a mutable iterator over the [`Keyed<WAFObject>`]s in this [WAFMap].
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut WAFObject> {
        let slice : &mut [WAFObject] = AsMut::as_mut(self);
        slice.iter_mut()
    }
});
typed_object!(WAFObjectType::Map => WAFMap {
    /// Creates a new [WAFMap] with the provided size. All values in the map are initialized
    /// to an invalid [WAFObject] instance with a blank key.
    ///
    /// # Panics
    /// Panics when the provided `nbEntries` is larger than what the current platform can represent in an [`usize`].
    /// This can only happen on platforms where [`usize`] is 32-bit wide.
    #[must_use]
    pub fn new(nb_entries: u64) -> Self {
        let size = usize::try_from(nb_entries).expect("size is too large for this platform");
        let layout = Layout::array::<bindings::ddwaf_object>(size).unwrap();
        let array = unsafe { no_fail_alloc(layout).cast() };
        unsafe { std::ptr::write_bytes(array, 0, size)};
        Self {
            raw: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_MAP,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 {  array },
                nbEntries: nb_entries,
                ..Default::default()
            }
        }
    }

    /// Returns the length of this [WAFMap].
    ///
    /// # Panics
    /// Panics if the map is larger than [`usize::MAX`] elements. This can only happen on platforms
    /// where [`usize`] is 32-bit wide.
    #[must_use]
    pub fn len(&self) -> usize {
        usize::try_from(self.raw.nbEntries).expect("map is too large for this platform")
    }

    /// Returns true if this [WAFMap] is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the [`Keyed<WAFObject>`]s in this [WAFMap].
    pub fn iter(&self) -> impl Iterator<Item = &Keyed<WAFObject>> {
        let slice : &[Keyed<WAFObject>] = self.as_ref();
        slice.iter()
    }

    /// Returns a mutable iterator over the [`Keyed<WAFObject>`]s in this [WAFMap].
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Keyed<WAFObject>> {
        let slice : &mut [Keyed<WAFObject>] = AsMut::as_mut(self);
        slice.iter_mut()
    }

    /// Returns a reference to the [`Keyed<WAFObject>`] with the provided key, if one exists.
    ///
    /// If multiple such objects exist in the receiver, the first match is returned.
    #[must_use]
    pub fn get(&self, key: &'_ [u8]) -> Option<&Keyed<WAFObject>> {
        for o in self.iter() {
            if o.key() == key {
                return Some(o)
            }
        }
        None
    }

    /// Returns a mutable reference to the [`Keyed<WAFObject>`] with the provided key, if one exists.
    ///
    /// If multiple such objects exist in the receiver, the first match is returned.
    pub fn get_mut(&mut self, key: &'_ [u8]) -> Option<&mut Keyed<WAFObject>> {
        for o in self.iter_mut() {
            if o.key() == key {
                return Some(o)
            }
        }
        None
    }

    /// Returns a reference to the [`Keyed<WAFObject>`] with the provided key, if one exists.
    #[must_use]
    pub fn get_str(&self, key: &'_ str) -> Option<&Keyed<WAFObject>> {
        self.get(key.as_bytes())
    }

    /// Returns a mutable reference to the [`Keyed<WAFObject>`] with the provided key, if one exists.
    pub fn get_str_mut(&mut self, key: &'_ str) -> Option<&mut Keyed<WAFObject>> {
        self.get_mut(key.as_bytes())
    }
});
typed_object!(WAFObjectType::Bool => WAFBool {
    /// Creates a new [WAFBool] with the provided value.
    #[must_use]
    pub const fn new(val: bool) -> Self {
        Self {
            raw: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_BOOL,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 { boolean: val },
                nbEntries: 0,
                parameterName: null_mut(),
                parameterNameLength: 0,
            }
        }
    }

    /// Returns the value of this [WAFBool].
    #[must_use]
    pub const fn value(&self) -> bool {
        unsafe { self.raw.__bindgen_anon_1.boolean }
    }
});
typed_object!(WAFObjectType::Float => WAFFloat {
    /// Creates a new [WAFFloat] with the provided value.
    #[must_use]
    pub const fn new(val: f64) -> Self {
        Self {
            raw: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_FLOAT,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 { f64_: val },
                nbEntries: 0,
                parameterName: null_mut(),
                parameterNameLength: 0,
            }
        }
    }

    /// Returns the value of this [WAFFloat].
    #[must_use]
    pub const fn value(&self) -> f64 {
        unsafe { self.raw.__bindgen_anon_1.f64_ }
    }
});
typed_object!(WAFObjectType::Null => WAFNull {
    /// Creates a new [WAFNull].
    #[must_use]
    pub const fn new() -> Self {
        Self { raw: bindings::ddwaf_object {
            type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_NULL,
            __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 { uintValue: 0},
            nbEntries: 0,
            parameterName: null_mut(),
            parameterNameLength: 0,
        } }
    }
});

impl fmt::Debug for WAFSigned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", stringify!(WAFSigned), self.value())
    }
}
impl From<i64> for WAFSigned {
    fn from(value: i64) -> Self {
        Self::new(value)
    }
}
impl From<i32> for WAFSigned {
    fn from(value: i32) -> Self {
        Self::new(value.into())
    }
}

impl fmt::Debug for WAFUnsigned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", stringify!(WAFUnsigned), self.value())
    }
}
impl From<u64> for WAFUnsigned {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}
impl From<u32> for WAFUnsigned {
    fn from(value: u32) -> Self {
        Self::new(value.into())
    }
}

impl<T: AsRef<[u8]>> From<T> for WAFString {
    fn from(val: T) -> Self {
        Self::new(val)
    }
}
impl fmt::Debug for WAFString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}(\"{:?}\")",
            stringify!(WAFString),
            fmt_bin_str(self.bytes())
        )
    }
}
impl Drop for WAFString {
    fn drop(&mut self) {
        unsafe { self.raw.drop_string() }
    }
}

impl AsRef<[WAFObject]> for WAFArray {
    fn as_ref(&self) -> &[WAFObject] {
        if self.is_empty() {
            return &[];
        }
        let array = unsafe { self.raw.__bindgen_anon_1.array.cast() };
        unsafe { std::slice::from_raw_parts(array, self.len()) }
    }
}
impl AsMut<[WAFObject]> for WAFArray {
    fn as_mut(&mut self) -> &mut [WAFObject] {
        if self.is_empty() {
            return &mut [];
        }
        let array = unsafe { self.raw.__bindgen_anon_1.array.cast() };
        unsafe { std::slice::from_raw_parts_mut(array, self.len()) }
    }
}
impl fmt::Debug for WAFArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[", stringify!(WAFArray))?;
        let mut first = true;
        for obj in self.iter() {
            if first {
                first = false;
            } else {
                write!(f, ", ")?;
            }
            write!(f, "{obj:?}")?;
        }
        write!(f, "]")
    }
}
impl Drop for WAFArray {
    fn drop(&mut self) {
        unsafe { self.raw.drop_array() }
    }
}
impl<'a, T> From<&'a [T]> for WAFArray
where
    &'a T: Into<WAFObject>,
{
    fn from(value: &'a [T]) -> Self {
        let mut array = Self::new(value.len() as u64);
        for (i, obj) in value.iter().enumerate() {
            array[i] = obj.into();
        }
        array
    }
}
impl Index<usize> for WAFArray {
    type Output = WAFObject;
    fn index(&self, index: usize) -> &Self::Output {
        let obj: &bindings::ddwaf_object = self.as_ref();
        assert!(
            // The object might be larger than [usize::MAX], but we ignore this for simplicity's sake.
            index < usize::try_from(obj.nbEntries).unwrap_or(usize::MAX),
            "Index out of bounds ({} >= {})",
            index,
            obj.nbEntries
        );
        let array = unsafe { obj.__bindgen_anon_1.array };
        unsafe { &*(array.add(index) as *const _) }
    }
}
impl IndexMut<usize> for WAFArray {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let obj: &bindings::ddwaf_object = self.as_ref();
        assert!(
            // The object might be larger than [usize::MAX], but we ignore this for simplicity's sake.
            index < usize::try_from(obj.nbEntries).unwrap_or(usize::MAX),
            "Index out of bounds ({} >= {})",
            index,
            obj.nbEntries
        );
        let array = unsafe { obj.__bindgen_anon_1.array };
        unsafe { &mut *(array.add(index).cast()) }
    }
}

impl AsRef<[Keyed<WAFObject>]> for WAFMap {
    fn as_ref(&self) -> &[Keyed<WAFObject>] {
        if self.is_empty() {
            return &[];
        }
        let array = unsafe { self.raw.__bindgen_anon_1.array as *const _ };
        unsafe { std::slice::from_raw_parts(array, self.len()) }
    }
}
impl AsMut<[Keyed<WAFObject>]> for WAFMap {
    fn as_mut(&mut self) -> &mut [Keyed<WAFObject>] {
        if self.is_empty() {
            return &mut [];
        }
        let array = unsafe { self.raw.__bindgen_anon_1.array.cast() };
        unsafe { std::slice::from_raw_parts_mut(array, self.len()) }
    }
}
impl fmt::Debug for WAFMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{{", stringify!(WAFMap))?;
        let mut first = true;
        for keyed_obj in self.iter() {
            if first {
                first = false;
            } else {
                write!(f, ", ")?;
            }
            write!(f, "{keyed_obj:?}")?;
        }
        write!(f, "}}")
    }
}
impl Drop for WAFMap {
    fn drop(&mut self) {
        unsafe { self.raw.drop_map() }
    }
}
impl Index<usize> for WAFMap {
    type Output = Keyed<WAFObject>;
    fn index(&self, index: usize) -> &Self::Output {
        let obj: &bindings::ddwaf_object = self.as_ref();
        assert!(
            // The object might be larger than [usize::MAX], but we ignore this for simplicity's sake.
            index < usize::try_from(obj.nbEntries).unwrap_or(usize::MAX),
            "Index out of bounds ({} >= {})",
            index,
            obj.nbEntries
        );
        let array = unsafe { obj.__bindgen_anon_1.array };
        unsafe { &*array.add(index) }.as_keyed_object_ref()
    }
}
impl IndexMut<usize> for WAFMap {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let obj: &bindings::ddwaf_object = self.as_ref();
        assert!(
            // The object might be larger than [usize::MAX], but we ignore this for simplicity's sake.
            index < usize::try_from(obj.nbEntries).unwrap_or(usize::MAX),
            "Index out of bounds ({} >= {})",
            index,
            obj.nbEntries
        );
        let array = unsafe { obj.__bindgen_anon_1.array };
        unsafe { (*array.add(index)).as_keyed_object_mut() }
    }
}

impl fmt::Debug for WAFBool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", stringify!(WAFBool), self.value())
    }
}
impl From<bool> for WAFBool {
    fn from(value: bool) -> Self {
        Self::new(value)
    }
}

impl fmt::Debug for WAFFloat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", stringify!(WAFFloat), self.value())
    }
}
impl From<f64> for WAFFloat {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl fmt::Debug for WAFNull {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(WAFNull))
    }
}
impl From<()> for WAFNull {
    fn from((): ()) -> Self {
        Self::new()
    }
}

/// An [`WAFObject`] or [`TypedWAFObject`] associated with a key.
#[repr(transparent)]
pub struct Keyed<T: AsRawMutObject> {
    value: T,
}
impl<T: AsRawMutObject> Keyed<T> {
    /// Creates a new [`Keyed<WAFObject>`] wrapping the provided value, without setting the key..
    pub(crate) fn new(value: T) -> Self {
        Self { value }
    }

    /// Obtains a reference to the wrapped value.
    pub fn inner(&self) -> &T {
        &self.value
    }

    /// Obtains the key associated with this [`Keyed<WAFObject>`], as-is.
    ///
    /// # Panics
    /// Panics if the underlying object's key is too long to be represented on this platform. This can only happen on
    /// platforms where [`usize`] is 32-bit wide.
    pub fn key(&self) -> &[u8] {
        let obj = self.as_ref();
        if obj.parameterNameLength == 0 {
            return &[];
        }
        let len =
            usize::try_from(obj.parameterNameLength).expect("key is too long for this platform");
        unsafe { std::slice::from_raw_parts(obj.parameterName.cast(), len) }
    }

    /// Obtains the key associated with this [`Keyed<WAFObject>`] as a string.
    ///
    /// # Errors
    /// Returns an error if the underlying key data is not a valid UTF-8 string, under the same conditions as
    /// [`std::str::from_utf8`].
    pub fn key_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.key())
    }

    /// Sets the key associated with this [`Keyed<WAFObject>`].
    pub(crate) fn set_key(&mut self, key: &[u8]) -> &mut Self {
        let obj = unsafe { self.as_raw_mut() };
        unsafe { obj.drop_key() };
        if key.is_empty() {
            obj.parameterName = null_mut();
            obj.parameterNameLength = 0;
            return self;
        }

        let b: Box<[u8]> = key.into();
        let ptr = Box::into_raw(b);
        obj.parameterName = ptr.cast();
        obj.parameterNameLength = key.len() as u64;
        self
    }

    /// Sets the key associated with this [`Keyed<WAFObject>`] to the provided string.
    pub(crate) fn set_key_str(&mut self, key: &str) -> &mut Self {
        self.set_key(key.as_bytes())
    }
}
impl Keyed<WAFObject> {
    #[must_use]
    pub fn as_type<T: TypedWAFObject>(&self) -> Option<&Keyed<T>> {
        if self.value.get_type() == T::TYPE {
            Some(unsafe { &*(std::ptr::from_ref(self).cast()) })
        } else {
            None
        }
    }

    pub fn as_type_mut<T: TypedWAFObject>(&mut self) -> Option<&mut Keyed<T>> {
        if self.value.get_type() == T::TYPE {
            Some(unsafe { &mut *(std::ptr::from_mut(self).cast()) })
        } else {
            None
        }
    }
}
// Note - We are not implementing DerefMut for Keyed as it'd allow leaking the key if it is used
// through [std::mem::take] or [std::mem::replace].
impl Keyed<WAFArray> {
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut WAFObject> {
        self.value.iter_mut()
    }
}
// Note - We are not implementing DerefMut for Keyed as it'd allow leaking the key if it is used
// through [std::mem::take] or [std::mem::replace].
impl Keyed<WAFMap> {
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Keyed<WAFObject>> {
        self.value.iter_mut()
    }
}
impl<T: AsRawMutObject> AsRawMutObject for Keyed<T> {
    unsafe fn as_raw_mut(&mut self) -> &mut bindings::ddwaf_object {
        self.value.as_raw_mut()
    }
}
impl<T: AsRawMutObject> crate::private::Sealed for Keyed<T> {}
impl<T: AsRawMutObject> AsRef<bindings::ddwaf_object> for Keyed<T> {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        self.value.as_ref()
    }
}
impl<T: Default + AsRawMutObject> std::default::Default for Keyed<T> {
    fn default() -> Self {
        Self {
            value: T::default(),
        }
    }
}
impl<T: AsRawMutObject> Deref for Keyed<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
impl<T: AsRawMutObject> std::ops::Drop for Keyed<T> {
    fn drop(&mut self) {
        unsafe { self.as_raw_mut().drop_key() };
        // self.value implicitly dropped
    }
}
impl<T: AsRawMutObject + fmt::Debug> fmt::Debug for Keyed<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{:?}\"={:?}", fmt_bin_str(self.key()), self.value)
    }
}
impl<T, U: AsRawMutObject> From<(&str, T)> for Keyed<U>
where
    T: Into<U>,
{
    fn from((key, value): (&str, T)) -> Self {
        let unkeyed = value.into();
        let mut keyed = Keyed::new(unkeyed);
        keyed.set_key_str(key);
        keyed
    }
}
impl<T, U: AsRawMutObject> From<(&[u8], T)> for Keyed<U>
where
    T: Into<U>,
{
    fn from((key, value): (&[u8], T)) -> Self {
        let unkeyed = value.into();
        let mut keyed = Keyed::new(unkeyed);
        keyed.set_key(key);
        keyed
    }
}
impl<T: TypedWAFObject> From<Keyed<T>> for Keyed<WAFObject> {
    fn from(value: Keyed<T>) -> Self {
        let res = Self {
            value: WAFObject {
                raw: *value.as_ref(),
            },
        };
        std::mem::forget(value);
        res
    }
}

impl crate::bindings::ddwaf_object {
    /// Converts a naked reference to a [`bindings::ddwaf_object`] into a reference to one of the
    /// user-friendlier types.
    ///
    /// # Safety
    /// The type `T` must be able to represent this [`bindings::ddwaf_object`]'s type (per its
    /// associated [`bindings::DDWAF_OBJ_TYPE`] value).
    pub(crate) unsafe fn unchecked_as_ref<
        T: AsRef<bindings::ddwaf_object> + crate::private::Sealed,
    >(
        &self,
    ) -> &T {
        &*(std::ptr::from_ref(self).cast())
    }

    /// Converts a naked mutable reference to a `ddwaf_object` into a mutable reference to one of the
    ///
    /// # Safety
    /// - The type `T` must be able to represent this [`bindings::ddwaf_object`]'s type (per its
    ///   associated [`bindings::DDWAF_OBJ_TYPE`] value).
    /// - The destructor of `T` must be compatible with the value of self.
    pub(crate) unsafe fn unchecked_as_ref_mut<
        T: AsRef<bindings::ddwaf_object> + crate::private::Sealed,
    >(
        &mut self,
    ) -> &mut T {
        &mut *(std::ptr::from_mut(self).cast())
    }

    /// Converts a naked reference to a [`bindings::ddwaf_object`] into a reference to an [`WAFObject`].
    pub(crate) fn as_object_ref(&self) -> &WAFObject {
        unsafe { self.unchecked_as_ref::<WAFObject>() }
    }

    /// Converts a naked reference to a [`bindings::ddwaf_object`] into a reference to an opaque
    /// [`Keyed<WAFObject>`].
    pub(crate) fn as_keyed_object_ref(&self) -> &Keyed<WAFObject> {
        // SAFETY: Keyed<WAFObject> is compatible with all valid [bindings::ddwaf_object] values,
        // event if their key is not set.
        unsafe { self.unchecked_as_ref::<Keyed<WAFObject>>() }
    }

    /// Converts a naked mutable reference to a [`bindings::ddwaf_object`] into a mutable reference to
    /// an opaque [`Keyed<WAFObject>`].
    ///
    /// # Safety
    /// The caller must ensure that the destructor of [`Keyed<WAFObject>`]
    /// ([`bindings::ddwaf_object::drop_key`] and [`bindings::ddwaf_object::drop_object`]) can be called
    /// on self.
    pub(crate) unsafe fn as_keyed_object_mut(&mut self) -> &mut Keyed<WAFObject> {
        self.unchecked_as_ref_mut::<Keyed<WAFObject>>()
    }
}

/// Formats a byte slice as an ASCII string, hex-escaping any non-printable characters.
fn fmt_bin_str(bytes: &[u8]) -> impl fmt::Debug + '_ {
    struct BinFormatter<'a>(&'a [u8]);
    impl fmt::Debug for BinFormatter<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            for &c in self.0 {
                if c.is_ascii_graphic() || c == b' ' {
                    write!(f, "{}", c as char)?;
                } else if c == b'"' || c == b'\\' {
                    write!(f, "\\{}", c as char)?;
                } else {
                    write!(f, "\\x{c:02X}")?;
                }
            }
            Ok(())
        }
    }
    BinFormatter(bytes)
}
