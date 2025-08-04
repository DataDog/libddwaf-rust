#![doc = "Data model for exchanging data with the in-app WAF."]

use std::alloc::Layout;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::ptr::null_mut;
use std::{cmp, fmt};

mod iter;
#[doc(inline)]
pub use iter::*;

/// Identifies the type of the value stored in a [`WafObject`].
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WafObjectType {
    /// An invalid value. This can be used as a placeholder to retain the key
    /// associated with an object that was only aprtially encoded.
    Invalid,
    /// A signed integer with 64-bit precision.
    Signed,
    /// An unsigned integer with 64-bit precision.
    Unsigned,
    /// A string value.
    String,
    /// An array of [`WafObject`]s.
    Array,
    /// A map of string-keyed [`WafObject`]s.
    Map,
    /// A boolean value.
    Bool,
    /// A floating point value (64-bit precision).
    Float,
    /// The null value.
    Null,
}
impl WafObjectType {
    /// Returns the raw [`libddwaf_sys::DDWAF_OBJ_TYPE`] value corresponding to this [`WafObjectType`].
    const fn as_raw(self) -> libddwaf_sys::DDWAF_OBJ_TYPE {
        match self {
            WafObjectType::Invalid => libddwaf_sys::DDWAF_OBJ_INVALID,
            WafObjectType::Signed => libddwaf_sys::DDWAF_OBJ_SIGNED,
            WafObjectType::Unsigned => libddwaf_sys::DDWAF_OBJ_UNSIGNED,
            WafObjectType::String => libddwaf_sys::DDWAF_OBJ_STRING,
            WafObjectType::Array => libddwaf_sys::DDWAF_OBJ_ARRAY,
            WafObjectType::Map => libddwaf_sys::DDWAF_OBJ_MAP,
            WafObjectType::Bool => libddwaf_sys::DDWAF_OBJ_BOOL,
            WafObjectType::Float => libddwaf_sys::DDWAF_OBJ_FLOAT,
            WafObjectType::Null => libddwaf_sys::DDWAF_OBJ_NULL,
        }
    }
}
impl TryFrom<libddwaf_sys::DDWAF_OBJ_TYPE> for WafObjectType {
    type Error = UnknownObjectTypeError;
    fn try_from(value: libddwaf_sys::DDWAF_OBJ_TYPE) -> Result<Self, UnknownObjectTypeError> {
        match value {
            libddwaf_sys::DDWAF_OBJ_INVALID => Ok(WafObjectType::Invalid),
            libddwaf_sys::DDWAF_OBJ_SIGNED => Ok(WafObjectType::Signed),
            libddwaf_sys::DDWAF_OBJ_UNSIGNED => Ok(WafObjectType::Unsigned),
            libddwaf_sys::DDWAF_OBJ_STRING => Ok(WafObjectType::String),
            libddwaf_sys::DDWAF_OBJ_ARRAY => Ok(WafObjectType::Array),
            libddwaf_sys::DDWAF_OBJ_MAP => Ok(WafObjectType::Map),
            libddwaf_sys::DDWAF_OBJ_BOOL => Ok(WafObjectType::Bool),
            libddwaf_sys::DDWAF_OBJ_FLOAT => Ok(WafObjectType::Float),
            libddwaf_sys::DDWAF_OBJ_NULL => Ok(WafObjectType::Null),
            unknown => Err(UnknownObjectTypeError(unknown)),
        }
    }
}

/// The error that is returned when a [`WafObject`] does not have a known, valid [`WafObjectType`].
#[derive(Copy, Clone, Debug)]
pub struct UnknownObjectTypeError(libddwaf_sys::DDWAF_OBJ_TYPE);
impl std::error::Error for UnknownObjectTypeError {}
impl std::fmt::Display for UnknownObjectTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown object type: {:?}", self.0)
    }
}

/// The error that is returned when a [`WafObject`] does not have the expected [`WafObjectType`].
#[derive(Copy, Clone, Debug)]
pub struct ObjectTypeError {
    pub expected: WafObjectType,
    pub actual: WafObjectType,
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
/// backing a [`WafObject`] or [`TypedWafObject`] value.
#[doc(hidden)]
pub trait AsRawMutObject: crate::private::Sealed + AsRef<libddwaf_sys::ddwaf_object> {
    /// Obtains a mutable reference to the underlying raw [`libddwaf_sys::ddwaf_object`].
    ///
    /// # Safety
    /// The caller must ensure that:
    /// - it does not change the [`libddwaf_sys::ddwaf_object::type_`] field,
    /// - it does not change the pointers to values that don't outlive the [`libddwaf_sys::ddwaf_object`]
    ///   itself, or whose memory cannot be recclaimed byt the destructor in the same way as the
    ///   current value,
    /// - it does not change the lengths in such a way that the object is no longer valid.
    ///
    /// Additionally, the caller would incur a memory leak if it dropped the value through the
    /// returned reference (e.g, by calling [`std::mem::replace`]), since [`libddwaf_sys::ddwaf_object`] is
    /// not [`Drop`] (see swapped destructors in
    /// [this playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=aeea4aba8f960bf0c63f6185f016a94d).
    #[doc(hidden)]
    unsafe fn as_raw_mut(&mut self) -> &mut libddwaf_sys::ddwaf_object;
}

/// This trait is implemented by type-safe interfaces to the [`WafObject`], with
/// one implementation for each [`WafObjectType`].
pub trait TypedWafObject: AsRawMutObject {
    /// The associated [`WafObjectType`] constant corresponding to the typed
    /// object's type discriminator.
    const TYPE: WafObjectType;
}

/// The low-level representation of an arbitrary WAF object.
///
/// It is usually converted to a [`TypedWafObject`] by calling [`WafObject::as_type`].
#[derive(Default)]
#[repr(transparent)]
pub struct WafObject {
    raw: libddwaf_sys::ddwaf_object,
}
impl WafObject {
    /// Creates a new [`WafObject`] from a JSON string.
    ///
    /// # Returns
    /// Returns [`None`] if parsing the JSON string into a [`WafObject`] was not
    /// possible, or if the input JSON string is larger than [`u32::MAX`] bytes.
    pub fn from_json(json: impl AsRef<[u8]>) -> Option<WafOwned<Self>> {
        let mut obj = WafOwned::<Self>::default();
        let data = json.as_ref();
        let Ok(len) = u32::try_from(data.len()) else {
            return None;
        };
        if !unsafe {
            libddwaf_sys::ddwaf_object_from_json(obj.as_raw_mut(), data.as_ptr().cast(), len)
        } {
            return None;
        }
        Some(obj)
    }

    /// Returns the [`WafObjectType`] of the underlying value.
    ///
    /// Returns [`WafObjectType::Invalid`] if the underlying value's type is not set to a
    /// known, valid [`WafObjectType`] value.
    #[must_use]
    pub fn get_type(&self) -> WafObjectType {
        self.as_ref()
            .type_
            .try_into()
            .unwrap_or(WafObjectType::Invalid)
    }

    /// Returns a reference to this value as a `T` if its type corresponds.
    #[must_use]
    pub fn as_type<T: TypedWafObject>(&self) -> Option<&T> {
        if self.get_type() == T::TYPE {
            Some(unsafe { self.as_type_unchecked::<T>() })
        } else {
            None
        }
    }

    /// Returns a reference to this value as a `T`.
    ///
    /// # Safety
    /// The caller must ensure that the [`WafObject`] can be accurately represented by `T`.
    pub(crate) unsafe fn as_type_unchecked<T: TypedWafObject>(&self) -> &T {
        unsafe { self.as_ref().unchecked_as_ref::<T>() }
    }

    /// Returns a mutable reference to this value as a `T` if its type corresponds.
    pub fn as_type_mut<T: TypedWafObject>(&mut self) -> Option<&mut T> {
        if self.get_type() == T::TYPE {
            Some(unsafe { self.as_raw_mut().unchecked_as_ref_mut::<T>() })
        } else {
            None
        }
    }

    /// Returns true if this [`WafObject`] is not [`WafObjectType::Invalid`], meaning it can be
    /// converted to one of the [`TypedWafObject`] implementations.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.get_type() != WafObjectType::Invalid
    }

    /// Returns the value of this [`WafObject`] as a [`u64`] if its type is [`WafObjectType::Unsigned`].
    #[must_use]
    pub fn to_u64(&self) -> Option<u64> {
        self.as_type::<WafUnsigned>().map(WafUnsigned::value)
    }

    /// Returns the value of this [`WafObject`] as a [`i64`] if its type is [`WafObjectType::Signed`] (or
    /// [`WafObjectType::Unsigned`] with a value that can be represented as an [`i64`]).
    #[must_use]
    pub fn to_i64(&self) -> Option<i64> {
        match self.get_type() {
            WafObjectType::Unsigned => {
                let obj: &WafUnsigned = unsafe { self.as_type_unchecked() };
                obj.value().try_into().ok()
            }
            WafObjectType::Signed => {
                let obj: &WafSigned = unsafe { self.as_type_unchecked() };
                Some(obj.value())
            }
            _ => None,
        }
    }

    /// Returns the value of this [`WafObject`] as a [`f64`] if its type is [`WafObjectType::Float`].
    #[must_use]
    pub fn to_f64(&self) -> Option<f64> {
        self.as_type::<WafFloat>().map(WafFloat::value)
    }

    /// Returns the value of this [`WafObject`] as a [`bool`] if its type is [`WafObjectType::Bool`].
    #[must_use]
    pub fn to_bool(&self) -> Option<bool> {
        self.as_type::<WafBool>().map(WafBool::value)
    }

    /// Returns the value of this [`WafObject`] as a [`&str`] if its type is [`WafObjectType::String`],
    /// and the value is valid UTF-8.
    #[must_use]
    pub fn to_str(&self) -> Option<&str> {
        self.as_type::<WafString>().and_then(|x| x.as_str().ok())
    }
}
impl AsRef<libddwaf_sys::ddwaf_object> for WafObject {
    fn as_ref(&self) -> &libddwaf_sys::ddwaf_object {
        &self.raw
    }
}
impl AsRawMutObject for WafObject {
    unsafe fn as_raw_mut(&mut self) -> &mut libddwaf_sys::ddwaf_object {
        &mut self.raw
    }
}
impl fmt::Debug for WafObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.get_type() {
            WafObjectType::Invalid => write!(f, "WafInvalid"),
            WafObjectType::Unsigned => {
                let obj: &WafUnsigned = self.as_type().unwrap();
                obj.fmt(f)
            }
            WafObjectType::Signed => {
                let obj: &WafSigned = self.as_type().unwrap();
                obj.fmt(f)
            }
            WafObjectType::Float => {
                let obj: &WafFloat = self.as_type().unwrap();
                obj.fmt(f)
            }
            WafObjectType::Bool => {
                let obj: &WafBool = self.as_type().unwrap();
                obj.fmt(f)
            }
            WafObjectType::Null => {
                let obj: &WafNull = self.as_type().unwrap();
                obj.fmt(f)
            }
            WafObjectType::String => {
                let obj: &WafString = self.as_type().unwrap();
                obj.fmt(f)
            }
            WafObjectType::Array => {
                let obj: &WafArray = self.as_type().unwrap();
                obj.fmt(f)
            }
            WafObjectType::Map => {
                let obj: &WafMap = self.as_type().unwrap();
                obj.fmt(f)
            }
        }
    }
}
impl Drop for WafObject {
    fn drop(&mut self) {
        unsafe { self.raw.drop_object() }
    }
}
impl From<u64> for WafObject {
    fn from(value: u64) -> Self {
        WafUnsigned::new(value).into()
    }
}
impl From<u32> for WafObject {
    fn from(value: u32) -> Self {
        WafUnsigned::new(value.into()).into()
    }
}
impl From<i64> for WafObject {
    fn from(value: i64) -> Self {
        WafSigned::new(value).into()
    }
}
impl From<i32> for WafObject {
    fn from(value: i32) -> Self {
        WafSigned::new(value.into()).into()
    }
}
impl From<f64> for WafObject {
    fn from(value: f64) -> Self {
        WafFloat::new(value).into()
    }
}
impl From<bool> for WafObject {
    fn from(value: bool) -> Self {
        WafBool::new(value).into()
    }
}
impl From<&str> for WafObject {
    fn from(value: &str) -> Self {
        WafString::new(value).into()
    }
}
impl From<&[u8]> for WafObject {
    fn from(value: &[u8]) -> Self {
        WafString::new(value).into()
    }
}
impl From<()> for WafObject {
    fn from((): ()) -> Self {
        WafNull::new().into()
    }
}
impl<T: TypedWafObject> From<T> for WafObject {
    fn from(value: T) -> Self {
        let res = Self {
            raw: *value.as_ref(),
        };
        std::mem::forget(value);
        res
    }
}
impl<T: AsRef<libddwaf_sys::ddwaf_object>> cmp::PartialEq<T> for WafObject {
    fn eq(&self, other: &T) -> bool {
        self.raw == *other.as_ref()
    }
}
impl crate::private::Sealed for WafObject {}

/// A WAF-owned [`WafObject`] or [`TypedWafObject`] value.
///
/// This has different [`Drop`] behavior than a rust-owned [`WafObject`] value.
pub struct WafOwned<T: AsRawMutObject> {
    inner: std::mem::ManuallyDrop<T>,
}
impl<T: AsRawMutObject + fmt::Debug> fmt::Debug for WafOwned<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.deref().fmt(f)
    }
}
impl<T: AsRawMutObject + Default> Default for WafOwned<T> {
    fn default() -> Self {
        Self {
            inner: std::mem::ManuallyDrop::new(Default::default()),
        }
    }
}
impl<T: AsRawMutObject> Deref for WafOwned<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl<T: AsRawMutObject> DerefMut for WafOwned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl<T: AsRawMutObject> Drop for WafOwned<T> {
    fn drop(&mut self) {
        unsafe { libddwaf_sys::ddwaf_object_free(self.inner.as_raw_mut()) };
    }
}
impl<T: AsRawMutObject> PartialEq<T> for WafOwned<T>
where
    T: PartialEq<T>,
{
    fn eq(&self, other: &T) -> bool {
        *self.inner == *other
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
            raw: libddwaf_sys::ddwaf_object,
        }
        impl $name {
            #[doc = concat!("Returns true if this [", stringify!($name), "] is indeed [", stringify!($type), "].")]
            #[must_use]
            pub const fn is_valid(&self) -> bool {
                self.raw.type_ == $type.as_raw()
            }

            /// Returns a reference to this value as a [`WafObject`].
            #[must_use]
            pub fn as_object(&self) -> &WafObject{
                let obj: &libddwaf_sys::ddwaf_object = self.as_ref();
                obj.as_object_ref()
            }
            $(
            $($impl)*)?
        }
        impl AsRef<libddwaf_sys::ddwaf_object> for $name {
            fn as_ref(&self) -> &libddwaf_sys::ddwaf_object {
                &self.raw
            }
        }
        impl AsRawMutObject for $name {
            unsafe fn as_raw_mut(&mut self) -> &mut libddwaf_sys::ddwaf_object {
                &mut self.raw
            }
        }
        impl Default for $name {
            fn default() -> Self {
                Self {
                    raw: libddwaf_sys::ddwaf_object {
                        type_: $type.as_raw(),
                        ..Default::default()
                    },
                }
            }
        }
        impl TryFrom<WafObject> for $name {
            type Error = ObjectTypeError;
            fn try_from(obj: WafObject) -> Result<Self, Self::Error> {
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
        impl<T: AsRef<libddwaf_sys::ddwaf_object>> cmp::PartialEq<T> for $name {
            fn eq(&self, other: &T) -> bool {
                self.raw == *other.as_ref()
            }
        }
        impl crate::private::Sealed for $name {}
        impl TypedWafObject for $name {
            const TYPE: WafObjectType = $type;
        }
    };
}

typed_object!(WafObjectType::Invalid => WafInvalid);
typed_object!(WafObjectType::Signed => WafSigned {
    /// Creates a new [`WafSigned`] with the provided value.
    #[must_use]
    pub const fn new(val: i64) -> Self {
        Self {
            raw: libddwaf_sys::ddwaf_object {
                type_: libddwaf_sys::DDWAF_OBJ_SIGNED,
                #[allow(clippy::used_underscore_items)]
                __bindgen_anon_1: libddwaf_sys::_ddwaf_object__bindgen_ty_1 { intValue: val },
                nbEntries: 0,
                parameterName: null_mut(),
                parameterNameLength: 0,
            }
        }
    }

    /// Returns the value of this [`WafSigned`].
    #[must_use]
    pub const fn value(&self) -> i64 {
        unsafe { self.raw.__bindgen_anon_1.intValue }
    }
});
typed_object!(WafObjectType::Unsigned => WafUnsigned {
    /// Creates a new [`WafUnsigned`] with the provided value.
    #[must_use]
    pub const fn new (val: u64) -> Self {
        Self {
            raw: libddwaf_sys::ddwaf_object {
                type_: libddwaf_sys::DDWAF_OBJ_UNSIGNED,
                #[allow(clippy::used_underscore_items)]
                __bindgen_anon_1: libddwaf_sys::_ddwaf_object__bindgen_ty_1 { uintValue: val },
                nbEntries: 0,
                parameterName: null_mut(),
                parameterNameLength: 0,
            }
        }
    }

    /// Retuns the value of this [`WafUnsigned`].
    #[must_use]
    pub const fn value(&self) -> u64 {
        unsafe { self.raw.__bindgen_anon_1.uintValue }
    }
});
typed_object!(WafObjectType::String => WafString {
    /// Creates a new [`WafString`] with the provided value.
    pub fn new(val: impl AsRef<[u8]>) -> Self {
        let val = val.as_ref();
        let ptr = if val.is_empty() {
            null_mut()
        } else {
            let b: Box<[u8]> = val.into();
            Box::into_raw(b).cast()
        };
        Self {
            raw: libddwaf_sys::ddwaf_object {
                type_: libddwaf_sys::DDWAF_OBJ_STRING,
                #[allow(clippy::used_underscore_items)]
                __bindgen_anon_1: libddwaf_sys::_ddwaf_object__bindgen_ty_1 {
                    stringValue: ptr,
                },
                nbEntries: val.len() as u64,
                ..Default::default()
            },
        }
    }

    /// Returns the length of this [`WafString`], in bytes.
    ///
    /// # Panics
    /// Panics if the string is larger than [`usize::MAX`] bytes. This can only happen on
    /// platforms where [`usize`] is 32-bit wide.
    #[must_use]
    pub fn len(&self) -> usize {
        usize::try_from(self.raw.nbEntries).expect("string is too large for this platform")
    }

    /// Returns true if this [`WafString`] is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a slice of the bytes from this [`WafString`].
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        debug_assert_eq!(self.raw.type_, libddwaf_sys::DDWAF_OBJ_STRING);
        let len = self.len();
        if len == 0 {
            return &[];
        }
        unsafe {
            std::slice::from_raw_parts(
                self.raw.__bindgen_anon_1.stringValue.cast(),
                len,
            )
        }
    }

    /// Returns a string slice from this [`WafString`].
    ///
    /// # Errors
    /// Returns an error if the underlying data is not a valid UTF-8 string, under the same conditions as
    /// [`std::str::from_utf8`].
    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.bytes())
    }
});
typed_object!(WafObjectType::Array => WafArray {
    /// Creates a new [`WafArray`] with the provided size. All values in the array are initialized
    /// to an invalid [`WafObject`] instance.
    ///
    /// # Panics
    /// Panics when the provided `nb_entries` is larger than what the current platform can represent in an [`usize`].
    /// This can only happen on platforms where [`usize`] is 32-bit wide.
    #[must_use]
    pub fn new(nb_entries: u64) -> Self {
        let size = usize::try_from(nb_entries).expect("size is too large for this platform");
        let layout = Layout::array::<libddwaf_sys::ddwaf_object>(size).unwrap();
        let array = unsafe { no_fail_alloc(layout).cast() };
        unsafe { std::ptr::write_bytes(array, 0, size)};
        Self {
            raw: libddwaf_sys::ddwaf_object {
                type_: libddwaf_sys::DDWAF_OBJ_ARRAY,
                #[allow(clippy::used_underscore_items)]
                __bindgen_anon_1: libddwaf_sys::_ddwaf_object__bindgen_ty_1 { array },
                nbEntries: nb_entries,
                ..Default::default()
            }
        }
    }

    /// Returns the length of this [`WafArray`].
    ///
    /// # Panics
    /// Panics if the array is larger than [`usize::MAX`] elements. This can only happen on
    /// platforms where [`usize`] is 32-bit wide.
    #[must_use]
    pub fn len(&self) -> usize {
        usize::try_from(self.raw.nbEntries).expect("array is too large for this platform")
    }

    /// Returns true if this [`WafArray`] is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the [`Keyed<WafObject>`]s in this [`WafMap`].
    pub fn iter(&self) -> impl Iterator<Item = &WafObject> {
        let slice : &[WafObject] = self.as_ref();
        slice.iter()
    }

    /// Returns a mutable iterator over the [`Keyed<WafObject>`]s in this [`WafMap`].
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut WafObject> {
        let slice : &mut [WafObject] = AsMut::as_mut(self);
        slice.iter_mut()
    }
});
typed_object!(WafObjectType::Map => WafMap {
    /// Creates a new [`WafMap`] with the provided size. All values in the map are initialized
    /// to an invalid [`WafObject`] instance with a blank key.
    ///
    /// # Panics
    /// Panics when the provided `nbEntries` is larger than what the current platform can represent in an [`usize`].
    /// This can only happen on platforms where [`usize`] is 32-bit wide.
    #[must_use]
    pub fn new(nb_entries: u64) -> Self {
        let size = usize::try_from(nb_entries).expect("size is too large for this platform");
        let layout = Layout::array::<libddwaf_sys::ddwaf_object>(size).unwrap();
        let array = unsafe { no_fail_alloc(layout).cast() };
        unsafe { std::ptr::write_bytes(array, 0, size)};
        Self {
            raw: libddwaf_sys::ddwaf_object {
                type_: libddwaf_sys::DDWAF_OBJ_MAP,
                #[allow(clippy::used_underscore_items)]
                __bindgen_anon_1: libddwaf_sys::_ddwaf_object__bindgen_ty_1 {  array },
                nbEntries: nb_entries,
                ..Default::default()
            }
        }
    }

    /// Returns the length of this [`WafMap`].
    ///
    /// # Panics
    /// Panics if the map is larger than [`usize::MAX`] elements. This can only happen on platforms
    /// where [`usize`] is 32-bit wide.
    #[must_use]
    pub fn len(&self) -> usize {
        usize::try_from(self.raw.nbEntries).expect("map is too large for this platform")
    }

    /// Returns true if this [`WafMap`] is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the [`Keyed<WafObject>`]s in this [`WafMap`].
    pub fn iter(&self) -> impl Iterator<Item = &Keyed<WafObject>> {
        let slice : &[Keyed<WafObject>] = self.as_ref();
        slice.iter()
    }

    /// Returns a mutable iterator over the [`Keyed<WafObject>`]s in this [`WafMap`].
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Keyed<WafObject>> {
        let slice : &mut [Keyed<WafObject>] = AsMut::as_mut(self);
        slice.iter_mut()
    }

    /// Returns a reference to the [`Keyed<WafObject>`] with the provided key, if one exists.
    ///
    /// If multiple such objects exist in the receiver, the first match is returned.
    #[must_use]
    pub fn get(&self, key: &'_ [u8]) -> Option<&Keyed<WafObject>> {
        for o in self.iter() {
            if o.key() == key {
                return Some(o)
            }
        }
        None
    }

    /// Returns a mutable reference to the [`Keyed<WafObject>`] with the provided key, if one exists.
    ///
    /// If multiple such objects exist in the receiver, the first match is returned.
    pub fn get_mut(&mut self, key: &'_ [u8]) -> Option<&mut Keyed<WafObject>> {
        for o in self.iter_mut() {
            if o.key() == key {
                return Some(o)
            }
        }
        None
    }

    /// Returns a reference to the [`Keyed<WafObject>`] with the provided key, if one exists.
    #[must_use]
    pub fn get_str(&self, key: &'_ str) -> Option<&Keyed<WafObject>> {
        self.get(key.as_bytes())
    }

    /// Returns a mutable reference to the [`Keyed<WafObject>`] with the provided key, if one exists.
    pub fn get_str_mut(&mut self, key: &'_ str) -> Option<&mut Keyed<WafObject>> {
        self.get_mut(key.as_bytes())
    }
});
typed_object!(WafObjectType::Bool => WafBool {
    /// Creates a new [`WafBool`] with the provided value.
    #[must_use]
    pub const fn new(val: bool) -> Self {
        Self {
            raw: libddwaf_sys::ddwaf_object {
                type_: libddwaf_sys::DDWAF_OBJ_BOOL,
                #[allow(clippy::used_underscore_items)]
                __bindgen_anon_1: libddwaf_sys::_ddwaf_object__bindgen_ty_1 { boolean: val },
                nbEntries: 0,
                parameterName: null_mut(),
                parameterNameLength: 0,
            }
        }
    }

    /// Returns the value of this [`WafBool`].
    #[must_use]
    pub const fn value(&self) -> bool {
        unsafe { self.raw.__bindgen_anon_1.boolean }
    }
});
typed_object!(WafObjectType::Float => WafFloat {
    /// Creates a new [`WafFloat`] with the provided value.
    #[must_use]
    pub const fn new(val: f64) -> Self {
        Self {
            raw: libddwaf_sys::ddwaf_object {
                type_: libddwaf_sys::DDWAF_OBJ_FLOAT,
                #[allow(clippy::used_underscore_items)]
                __bindgen_anon_1: libddwaf_sys::_ddwaf_object__bindgen_ty_1 { f64_: val },
                nbEntries: 0,
                parameterName: null_mut(),
                parameterNameLength: 0,
            }
        }
    }

    /// Returns the value of this [`WafFloat`].
    #[must_use]
    pub const fn value(&self) -> f64 {
        unsafe { self.raw.__bindgen_anon_1.f64_ }
    }
});
typed_object!(WafObjectType::Null => WafNull {
    /// Creates a new [`WafNull`].
    #[must_use]
    pub const fn new() -> Self {
        Self { raw: libddwaf_sys::ddwaf_object {
            type_: libddwaf_sys::DDWAF_OBJ_NULL,
            #[allow(clippy::used_underscore_items)]
            __bindgen_anon_1: libddwaf_sys::_ddwaf_object__bindgen_ty_1 { uintValue: 0},
            nbEntries: 0,
            parameterName: null_mut(),
            parameterNameLength: 0,
        } }
    }
});

impl fmt::Debug for WafSigned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", stringify!(WafSigned), self.value())
    }
}
impl From<i64> for WafSigned {
    fn from(value: i64) -> Self {
        Self::new(value)
    }
}
impl From<i32> for WafSigned {
    fn from(value: i32) -> Self {
        Self::new(value.into())
    }
}

impl fmt::Debug for WafUnsigned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", stringify!(WafUnsigned), self.value())
    }
}
impl From<u64> for WafUnsigned {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}
impl From<u32> for WafUnsigned {
    fn from(value: u32) -> Self {
        Self::new(value.into())
    }
}

impl<T: AsRef<[u8]>> From<T> for WafString {
    fn from(val: T) -> Self {
        Self::new(val)
    }
}
impl fmt::Debug for WafString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}(\"{:?}\")",
            stringify!(WafString),
            fmt_bin_str(self.bytes())
        )
    }
}
impl Drop for WafString {
    fn drop(&mut self) {
        unsafe { self.raw.drop_string() }
    }
}

impl AsRef<[WafObject]> for WafArray {
    fn as_ref(&self) -> &[WafObject] {
        if self.is_empty() {
            return &[];
        }
        let array = unsafe { self.raw.__bindgen_anon_1.array.cast() };
        unsafe { std::slice::from_raw_parts(array, self.len()) }
    }
}
impl AsMut<[WafObject]> for WafArray {
    fn as_mut(&mut self) -> &mut [WafObject] {
        if self.is_empty() {
            return &mut [];
        }
        let array = unsafe { self.raw.__bindgen_anon_1.array.cast() };
        unsafe { std::slice::from_raw_parts_mut(array, self.len()) }
    }
}
impl fmt::Debug for WafArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[", stringify!(WafArray))?;
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
impl Drop for WafArray {
    fn drop(&mut self) {
        unsafe { self.raw.drop_array() }
    }
}
impl<T: Into<WafObject>, const N: usize> From<[T; N]> for WafArray {
    fn from(value: [T; N]) -> Self {
        let mut array = Self::new(value.len() as u64);
        for (i, obj) in value.into_iter().enumerate() {
            array[i] = obj.into();
        }
        array
    }
}
impl Index<usize> for WafArray {
    type Output = WafObject;
    fn index(&self, index: usize) -> &Self::Output {
        let obj: &libddwaf_sys::ddwaf_object = self.as_ref();
        assert!(
            // The object might be larger than [usize::MAX], but we ignore this for simplicity's sake.
            index < usize::try_from(obj.nbEntries).unwrap_or(usize::MAX),
            "index out of bounds ({} >= {})",
            index,
            obj.nbEntries
        );
        let array = unsafe { obj.__bindgen_anon_1.array };
        unsafe { &*(array.add(index) as *const _) }
    }
}
impl IndexMut<usize> for WafArray {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let obj: &libddwaf_sys::ddwaf_object = self.as_ref();
        assert!(
            // The object might be larger than [usize::MAX], but we ignore this for simplicity's sake.
            index < usize::try_from(obj.nbEntries).unwrap_or(usize::MAX),
            "index out of bounds ({} >= {})",
            index,
            obj.nbEntries
        );
        let array = unsafe { obj.__bindgen_anon_1.array };
        unsafe { &mut *(array.add(index).cast()) }
    }
}

impl AsRef<[Keyed<WafObject>]> for WafMap {
    fn as_ref(&self) -> &[Keyed<WafObject>] {
        if self.is_empty() {
            return &[];
        }
        let array = unsafe { self.raw.__bindgen_anon_1.array as *const _ };
        unsafe { std::slice::from_raw_parts(array, self.len()) }
    }
}
impl AsMut<[Keyed<WafObject>]> for WafMap {
    fn as_mut(&mut self) -> &mut [Keyed<WafObject>] {
        if self.is_empty() {
            return &mut [];
        }
        let array = unsafe { self.raw.__bindgen_anon_1.array.cast() };
        unsafe { std::slice::from_raw_parts_mut(array, self.len()) }
    }
}
impl fmt::Debug for WafMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{{", stringify!(WafMap))?;
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
impl Drop for WafMap {
    fn drop(&mut self) {
        unsafe { self.raw.drop_map() }
    }
}
impl Index<usize> for WafMap {
    type Output = Keyed<WafObject>;
    fn index(&self, index: usize) -> &Self::Output {
        let obj: &libddwaf_sys::ddwaf_object = self.as_ref();
        assert!(
            // The object might be larger than [usize::MAX], but we ignore this for simplicity's sake.
            index < usize::try_from(obj.nbEntries).unwrap_or(usize::MAX),
            "index out of bounds ({} >= {})",
            index,
            obj.nbEntries
        );
        let array = unsafe { obj.__bindgen_anon_1.array };
        unsafe { &*array.add(index) }.as_keyed_object_ref()
    }
}
impl IndexMut<usize> for WafMap {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let obj: &libddwaf_sys::ddwaf_object = self.as_ref();
        assert!(
            // The object might be larger than [usize::MAX], but we ignore this for simplicity's sake.
            index < usize::try_from(obj.nbEntries).unwrap_or(usize::MAX),
            "index out of bounds ({} >= {})",
            index,
            obj.nbEntries
        );
        let array = unsafe { obj.__bindgen_anon_1.array };
        unsafe { (*array.add(index)).as_keyed_object_mut() }
    }
}
impl<K: AsRef<[u8]>, V: Into<WafObject>, const N: usize> From<[(K, V); N]> for WafMap {
    fn from(vals: [(K, V); N]) -> Self {
        let mut map = WafMap::new(N as u64);
        for (i, (k, v)) in vals.into_iter().enumerate() {
            map[i] = Keyed::from((k.as_ref(), v));
        }
        map
    }
}

impl fmt::Debug for WafBool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", stringify!(WafBool), self.value())
    }
}
impl From<bool> for WafBool {
    fn from(value: bool) -> Self {
        Self::new(value)
    }
}

impl fmt::Debug for WafFloat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", stringify!(WafFloat), self.value())
    }
}
impl From<f64> for WafFloat {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl fmt::Debug for WafNull {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(WafNull))
    }
}
impl From<()> for WafNull {
    fn from((): ()) -> Self {
        Self::new()
    }
}

/// An [`WafObject`] or [`TypedWafObject`] associated with a key.
#[repr(transparent)]
pub struct Keyed<T: AsRawMutObject> {
    value: T,
}
impl<T: AsRawMutObject> Keyed<T> {
    /// Creates a new [`Keyed<WafObject>`] wrapping the provided value, without setting the key..
    pub(crate) fn new(value: T) -> Self {
        Self { value }
    }

    /// Obtains a reference to the wrapped value.
    pub fn inner(&self) -> &T {
        &self.value
    }

    /// Obtains the key associated with this [`Keyed<WafObject>`], as-is.
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

    /// Obtains the key associated with this [`Keyed<WafObject>`] as a string.
    ///
    /// # Errors
    /// Returns an error if the underlying key data is not a valid UTF-8 string, under the same conditions as
    /// [`std::str::from_utf8`].
    pub fn key_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.key())
    }

    /// Sets the key associated with this [`Keyed<WafObject>`].
    pub fn set_key(&mut self, key: &[u8]) -> &mut Self {
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

    /// Sets the key associated with this [`Keyed<WafObject>`] to the provided string.
    pub fn set_key_str(&mut self, key: &str) -> &mut Self {
        self.set_key(key.as_bytes())
    }
}
impl Keyed<WafObject> {
    #[must_use]
    pub fn as_type<T: TypedWafObject>(&self) -> Option<&Keyed<T>> {
        if self.value.get_type() == T::TYPE {
            Some(unsafe { &*(std::ptr::from_ref(self).cast()) })
        } else {
            None
        }
    }

    pub fn as_type_mut<T: TypedWafObject>(&mut self) -> Option<&mut Keyed<T>> {
        if self.value.get_type() == T::TYPE {
            Some(unsafe { &mut *(std::ptr::from_mut(self).cast()) })
        } else {
            None
        }
    }
}
// Note - We are not implementing DerefMut for Keyed as it'd allow leaking the key if it is used
// through [std::mem::take] or [std::mem::replace].
impl Keyed<WafArray> {
    pub fn iter(&self) -> impl Iterator<Item = &WafObject> {
        self.value.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut WafObject> {
        self.value.iter_mut()
    }
}
// Note - We are not implementing DerefMut for Keyed as it'd allow leaking the key if it is used
// through [std::mem::take] or [std::mem::replace].
impl Keyed<WafMap> {
    pub fn iter(&self) -> impl Iterator<Item = &Keyed<WafObject>> {
        self.value.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Keyed<WafObject>> {
        self.value.iter_mut()
    }
}
impl<T: AsRawMutObject> AsRawMutObject for Keyed<T> {
    unsafe fn as_raw_mut(&mut self) -> &mut libddwaf_sys::ddwaf_object {
        unsafe { self.value.as_raw_mut() }
    }
}
impl<T: AsRawMutObject> crate::private::Sealed for Keyed<T> {}
impl<T: AsRawMutObject> AsRef<libddwaf_sys::ddwaf_object> for Keyed<T> {
    fn as_ref(&self) -> &libddwaf_sys::ddwaf_object {
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
impl<T: TypedWafObject> From<Keyed<T>> for Keyed<WafObject> {
    fn from(value: Keyed<T>) -> Self {
        let res = Self {
            value: WafObject {
                raw: *value.as_ref(),
            },
        };
        std::mem::forget(value);
        res
    }
}

trait UncheckedAsRef: crate::private::Sealed {
    /// Converts a naked reference to a [`libddwaf_sys::ddwaf_object`] into a reference to one of the
    /// user-friendlier types.
    ///
    /// # Safety
    /// The type `T` must be able to represent this [`libddwaf_sys::ddwaf_object`]'s type (per its
    /// associated [`libddwaf_sys::DDWAF_OBJ_TYPE`] value).
    unsafe fn unchecked_as_ref<T: AsRef<libddwaf_sys::ddwaf_object> + crate::private::Sealed>(
        &self,
    ) -> &T;

    /// Converts a naked mutable reference to a `ddwaf_object` into a mutable reference to one of the
    ///
    /// # Safety
    /// - The type `T` must be able to represent this [`libddwaf_sys::ddwaf_object`]'s type (per its
    ///   associated [`libddwaf_sys::DDWAF_OBJ_TYPE`] value).
    /// - The destructor of `T` must be compatible with the value of self.
    unsafe fn unchecked_as_ref_mut<T: AsRef<libddwaf_sys::ddwaf_object> + crate::private::Sealed>(
        &mut self,
    ) -> &mut T;
}
impl crate::private::Sealed for libddwaf_sys::ddwaf_object {}
impl UncheckedAsRef for libddwaf_sys::ddwaf_object {
    unsafe fn unchecked_as_ref<T: AsRef<libddwaf_sys::ddwaf_object> + crate::private::Sealed>(
        &self,
    ) -> &T {
        unsafe { &*(std::ptr::from_ref(self).cast()) }
    }

    unsafe fn unchecked_as_ref_mut<
        T: AsRef<libddwaf_sys::ddwaf_object> + crate::private::Sealed,
    >(
        &mut self,
    ) -> &mut T {
        unsafe { &mut *(std::ptr::from_mut(self).cast()) }
    }
}
trait UncheckedAsWafObject: crate::private::Sealed {
    /// Converts a naked reference to a [`libddwaf_sys::ddwaf_object`] into a reference to an [`WafObject`].
    fn as_object_ref(&self) -> &WafObject;

    /// Converts a naked reference to a [`libddwaf_sys::ddwaf_object`] into a reference to an opaque
    /// [`Keyed<WafObject>`].
    fn as_keyed_object_ref(&self) -> &Keyed<WafObject>;

    /// Converts a naked mutable reference to a [`libddwaf_sys::ddwaf_object`] into a mutable reference to
    /// an opaque [`Keyed<WafObject>`].
    ///
    /// # Safety
    /// The caller must ensure that the destructor of [`Keyed<WafObject>`]
    /// ([`libddwaf_sys::ddwaf_object::drop_key`] and [`libddwaf_sys::ddwaf_object::drop_object`]) can be called
    /// on self.
    unsafe fn as_keyed_object_mut(&mut self) -> &mut Keyed<WafObject>;
}
impl<T: UncheckedAsRef> UncheckedAsWafObject for T {
    /// Converts a naked reference to a [`libddwaf_sys::ddwaf_object`] into a reference to an [`WafObject`].
    fn as_object_ref(&self) -> &WafObject {
        unsafe { self.unchecked_as_ref::<WafObject>() }
    }

    /// Converts a naked reference to a [`libddwaf_sys::ddwaf_object`] into a reference to an opaque
    /// [`Keyed<WafObject>`].
    fn as_keyed_object_ref(&self) -> &Keyed<WafObject> {
        // SAFETY: Keyed<WafObject> is compatible with all valid [libddwaf_sys::ddwaf_object] values,
        // event if their key is not set.
        unsafe { self.unchecked_as_ref::<Keyed<WafObject>>() }
    }

    /// Converts a naked mutable reference to a [`libddwaf_sys::ddwaf_object`] into a mutable reference to
    /// an opaque [`Keyed<WafObject>`].
    ///
    /// # Safety
    /// The caller must ensure that the destructor of [`Keyed<WafObject>`]
    /// ([`libddwaf_sys::ddwaf_object::drop_key`] and [`libddwaf_sys::ddwaf_object::drop_object`]) can be called
    /// on self.
    unsafe fn as_keyed_object_mut(&mut self) -> &mut Keyed<WafObject> {
        unsafe { self.unchecked_as_ref_mut::<Keyed<WafObject>>() }
    }
}

/// Formats a byte slice as an ASCII string, hex-escaping any non-printable characters.
fn fmt_bin_str(bytes: &[u8]) -> impl fmt::Debug + '_ {
    struct BinFormatter<'a>(&'a [u8]);
    impl fmt::Debug for BinFormatter<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            for &c in self.0 {
                if c == b'"' || c == b'\\' {
                    write!(f, "\\{}", c as char)?;
                } else if c.is_ascii_graphic() || c == b' ' {
                    write!(f, "{}", c as char)?;
                } else {
                    write!(f, "\\x{c:02X}")?;
                }
            }
            Ok(())
        }
    }
    BinFormatter(bytes)
}

/// Helper macro to create [`WafObject`]s.
#[macro_export]
macro_rules! waf_object {
    (null) => {
        $crate::object::WafObject::from(())
    };
    ($l:expr) => {
        $crate::object::WafObject::from($l)
    };
}

/// Helper macro to create [`WafArray`]s.
#[macro_export]
macro_rules! waf_array {
    () => { $crate::object::WafArray::new(0) };
    ($($e:expr),* $(,)?) => {
        {
            let size = [$($crate::__repl_expr_with_unit!($e)),*].len();
            let mut res = $crate::object::WafArray::new(size as u64);
            let mut i = usize::MAX;
            $(
                i = i.wrapping_add(1);
                res[i] = $crate::waf_object!($e);
            )*
            res
        }
    };
}

/// Helper macro to create [`WafMap`]s.
#[macro_export]
macro_rules! waf_map {
    () => { $crate::object::WafMap::new(0) };
    ($(($k:literal, $v:expr)),* $(,)?) => {
        {
            let size = [$($crate::__repl_expr_with_unit!($v)),*].len();
            let mut res = $crate::object::WafMap::new(size as u64);
            let mut i = usize::MAX;
            $(
                i = i.wrapping_add(1);
                let k: &str = $k.into();
                let obj = $crate::object::Keyed::<$crate::object::WafObject>::from((k, $v));
                res[i] = obj.into();
            )*
            res
        }
    };
}

/// Helper macro to facilitate counting token trees within other macros.
///
/// Not intended for use outside of this crate, but must be exported as it is used by macros in this crate.
#[doc(hidden)]
#[macro_export]
macro_rules! __repl_expr_with_unit {
    ($e:expr) => {
        ()
    };
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    #[allow(clippy::float_cmp)] // No operations are done on the values, they should be the same.
    fn unsafe_changes_to_default_objects() {
        unsafe {
            let mut unsigned = WafUnsigned::default();
            unsigned.as_raw_mut().__bindgen_anon_1.uintValue += 1;
            assert_eq!(unsigned.value(), 1);

            let mut signed = WafSigned::default();
            signed.as_raw_mut().__bindgen_anon_1.intValue -= 1;
            assert_eq!(signed.value(), -1);

            let mut float = WafFloat::default();
            float.as_raw_mut().__bindgen_anon_1.f64_ += 1.0;
            assert_eq!(float.value(), 1.0);

            let mut boolean = WafBool::default();
            boolean.as_raw_mut().__bindgen_anon_1.boolean = true;
            assert!(boolean.value());

            let mut null = WafNull::default();
            // nothing interesting to do for null; let's try manually setting
            // the parameter name
            let s = String::from_str("foobar").unwrap();
            let b: Box<[u8]> = s.as_bytes().into();
            let p = Box::<[u8]>::into_raw(b);
            let null_mut = null.as_raw_mut();
            null_mut.parameterName = p.cast();
            null_mut.parameterNameLength = s.len() as u64;
            drop(std::mem::take(null_mut.as_keyed_object_mut()));

            let mut string = WafString::default();
            let str_mut = string.as_raw_mut();
            let b: Box<[u8]> = s.as_bytes().into();
            let p = Box::<[u8]>::into_raw(b);
            str_mut.drop_string();
            str_mut.__bindgen_anon_1.stringValue = p as *const _;
            str_mut.nbEntries = s.len() as u64;
            assert_eq!(string.as_str().unwrap(), "foobar");
            assert_eq!(string.len(), s.len());
            assert!(!string.is_empty());
        }
    }
}
