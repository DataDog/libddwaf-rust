use core::slice;
use std::{
    alloc::Layout,
    ffi::{CStr, CString},
    fmt::Write,
    mem::MaybeUninit,
    ops::{Deref, DerefMut, Fn, Index, IndexMut},
    ptr::{addr_of_mut, null, null_mut, NonNull},
    sync::{Arc, Mutex},
};

use arc_swap::ArcSwap;

#[allow(unused)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

mod private {
    pub trait Sealed {}
}
#[cfg(feature = "serde")]
pub mod serde;
pub mod shallow;

/// # Safety
///
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

#[repr(u32)]
#[derive(Debug, PartialEq)]
pub enum DdwafObjType {
    Invalid = bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_INVALID,
    Signed = bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_SIGNED,
    Unsigned = bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_UNSIGNED,
    String = bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_STRING,
    Array = bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_ARRAY,
    Map = bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_MAP,
    Bool = bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_BOOL,
    Float = bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_FLOAT,
    Null = bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_NULL,
}
impl TryFrom<::std::os::raw::c_uint> for DdwafObjType {
    type Error = DdwafObjTypeError;
    fn try_from(value: ::std::os::raw::c_uint) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(DdwafObjType::Invalid),
            1 => Ok(DdwafObjType::Signed),
            2 => Ok(DdwafObjType::Unsigned),
            4 => Ok(DdwafObjType::String),
            8 => Ok(DdwafObjType::Array),
            16 => Ok(DdwafObjType::Map),
            32 => Ok(DdwafObjType::Bool),
            64 => Ok(DdwafObjType::Float),
            128 => Ok(DdwafObjType::Null),
            _ => Err(Self::Error {
                message: "Invalid DDWAFObjType value",
            }),
        }
    }
}

// One of: DdwafObjUnsignedInt, DdwafObjSignedInt, DdwafObjFloat,
// DdwafObjString, DdwafObjArray, DdwafObjMap, DdwafObjBool, DdwafObjNull
pub trait TypedDdwafObj: private::Sealed + AsRef<bindings::ddwaf_object> {
    const TYPE: DdwafObjType;
}

#[derive(Debug)]
pub struct DdwafObjTypeError {
    message: &'static str,
}
impl std::fmt::Display for DdwafObjTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for DdwafObjTypeError {}

#[derive(Debug)]
pub struct DdwafGenericError {
    message: &'static str,
}
impl std::fmt::Display for DdwafGenericError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for DdwafGenericError {}
impl From<&'static str> for DdwafGenericError {
    fn from(message: &'static str) -> Self {
        DdwafGenericError { message }
    }
}

#[repr(C)]
pub struct DdwafObj {
    _obj: bindings::ddwaf_object,
}

#[repr(C)]
pub struct DdwafObjUnsignedInt {
    _obj: bindings::ddwaf_object,
}

#[repr(C)]
pub struct DdwafObjSignedInt {
    _obj: bindings::ddwaf_object,
}

#[repr(C)]
pub struct DdwafObjFloat {
    _obj: bindings::ddwaf_object,
}

#[repr(C)]
pub struct DdwafObjString {
    _obj: bindings::ddwaf_object,
}

#[repr(C)]
pub struct DdwafObjBool {
    _obj: bindings::ddwaf_object,
}

#[repr(C)]
pub struct DdwafObjNull {
    _obj: bindings::ddwaf_object,
}

#[repr(C)]
pub struct DdwafObjArray {
    _obj: bindings::ddwaf_object,
}

#[repr(C)]
pub struct DdwafObjMap {
    _obj: bindings::ddwaf_object,
}

pub trait AsRawDdwafObjMut: private::Sealed {
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - it doesn't change the type of the object,
    /// - it doesn't change the pointers to values that don't live till the end of the object,
    ///   or whose memory can't be reclaimed the same way in the destructor,
    /// - it doesn't change the lengths in such a way that the object is no longer valid.
    ///
    /// Additionally, the caller would leak memory if it dropped the value through
    /// the returned reference (e.g. by calling `std::mem::replace`), since
    /// bindings::ddwaf_object is not `Drop` (see swapped destructors in
    /// https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=aeea4aba8f960bf0c63f6185f016a94d )
    unsafe fn as_mut(&mut self) -> &mut bindings::ddwaf_object;
}

#[repr(C)]
pub struct Keyed<T: AsRef<bindings::ddwaf_object> + AsRawDdwafObjMut> {
    value: T,
}
impl<T> Default for Keyed<T>
where
    T: Default + AsRef<bindings::ddwaf_object> + AsRawDdwafObjMut,
{
    fn default() -> Self {
        Self {
            value: T::default(),
        }
    }
}
impl<T> private::Sealed for Keyed<T> where T: AsRef<bindings::ddwaf_object> + AsRawDdwafObjMut {}

// implement Send + Sync on ddwaf_object to avoid haing to implement it on each struct
// In any case, there's nothing thread unsafe about bindings::ddwaf_object
// (as long as its pointers are not dereferenced, which is unsafe anyway)
unsafe impl Send for bindings::ddwaf_object {}
unsafe impl Sync for bindings::ddwaf_object {}

impl AsRef<bindings::ddwaf_object> for DdwafObj {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        &self._obj
    }
}
impl private::Sealed for DdwafObj {}
impl AsRawDdwafObjMut for DdwafObj {
    unsafe fn as_mut(&mut self) -> &mut bindings::ddwaf_object {
        &mut self._obj
    }
}
impl<T> AsRef<bindings::ddwaf_object> for Keyed<T>
where
    T: AsRef<bindings::ddwaf_object> + AsRawDdwafObjMut,
{
    fn as_ref(&self) -> &bindings::ddwaf_object {
        self.value.as_ref()
    }
}
impl<T> AsRawDdwafObjMut for Keyed<T>
where
    T: AsRef<bindings::ddwaf_object> + AsRawDdwafObjMut,
{
    unsafe fn as_mut(&mut self) -> &mut bindings::ddwaf_object {
        self.value.as_mut()
    }
}
impl AsRef<bindings::ddwaf_object> for DdwafObjUnsignedInt {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        &self._obj
    }
}
impl AsRawDdwafObjMut for DdwafObjUnsignedInt {
    unsafe fn as_mut(&mut self) -> &mut bindings::ddwaf_object {
        &mut self._obj
    }
}
impl AsRef<bindings::ddwaf_object> for DdwafObjSignedInt {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        &self._obj
    }
}
impl AsRawDdwafObjMut for DdwafObjSignedInt {
    unsafe fn as_mut(&mut self) -> &mut bindings::ddwaf_object {
        &mut self._obj
    }
}
impl AsRef<bindings::ddwaf_object> for DdwafObjFloat {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        &self._obj
    }
}
impl AsRawDdwafObjMut for DdwafObjFloat {
    unsafe fn as_mut(&mut self) -> &mut bindings::ddwaf_object {
        &mut self._obj
    }
}
impl AsRef<bindings::ddwaf_object> for DdwafObjBool {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        &self._obj
    }
}
impl AsRawDdwafObjMut for DdwafObjBool {
    unsafe fn as_mut(&mut self) -> &mut bindings::ddwaf_object {
        &mut self._obj
    }
}
impl AsRef<bindings::ddwaf_object> for DdwafObjNull {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        &self._obj
    }
}
impl AsRawDdwafObjMut for DdwafObjNull {
    unsafe fn as_mut(&mut self) -> &mut bindings::ddwaf_object {
        &mut self._obj
    }
}
impl AsRef<bindings::ddwaf_object> for DdwafObjString {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        &self._obj
    }
}
impl AsRawDdwafObjMut for DdwafObjString {
    unsafe fn as_mut(&mut self) -> &mut bindings::ddwaf_object {
        &mut self._obj
    }
}
impl AsRef<bindings::ddwaf_object> for DdwafObjArray {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        &self._obj
    }
}
impl AsRawDdwafObjMut for DdwafObjArray {
    unsafe fn as_mut(&mut self) -> &mut bindings::ddwaf_object {
        &mut self._obj
    }
}
impl AsRef<bindings::ddwaf_object> for DdwafObjMap {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        &self._obj
    }
}
impl AsRawDdwafObjMut for DdwafObjMap {
    unsafe fn as_mut(&mut self) -> &mut bindings::ddwaf_object {
        &mut self._obj
    }
}

// generic
impl bindings::ddwaf_object {
    /// Converts a naked reference to a ddwaf_object into a reference to one of the user-friendly
    /// types. The caller must guarantee the conversion is valid.
    ///
    /// # Safety
    /// The type [`T`] can represent this ddwaf_object type ([`bindings::DDWAF_OBJ_TYPE`]).
    unsafe fn unchecked_as_ref<T: AsRef<bindings::ddwaf_object>>(&self) -> &T {
        &*(self as *const bindings::ddwaf_object as *const T)
    }

    /// Converts a naked mutable reference to a ddwaf_object into a mutable reference to one of the
    ///
    /// # Safety
    /// - The type [`T`] can represent this ddwaf_object type ([`bindings::DDWAF_OBJ_TYPE`]).
    /// - The destructor of [`T`] must be compatible with the value of self.
    unsafe fn unchecked_as_ref_mut<T: AsRawDdwafObjMut>(&mut self) -> &mut T {
        &mut *(self as *mut bindings::ddwaf_object as *mut T)
    }

    fn as_ddwaf_obj_ref(&self) -> &DdwafObj {
        // SAFETY: DdwafObj is compatible with all valid ddwaf_objects.
        unsafe { self.unchecked_as_ref::<DdwafObj>() }
    }

    fn as_keyed_ddwaf_obj_ref(&self) -> &Keyed<DdwafObj> {
        // SAFETY: Keyed<DdwafObj> is compatible with all valid ddwaf_objects, even if their key
        // is not set.
        unsafe { self.unchecked_as_ref::<Keyed<DdwafObj>>() }
    }

    /// # Safety
    ///
    /// The caller must ensure that the destructor of Keyed<DdwafObj>
    /// ([`drop_key()`] + [`drop_ddwaf_object()`]) can be called on self.
    unsafe fn as_keyed_ddwaf_obj_ref_mut(&mut self) -> &mut Keyed<DdwafObj> {
        self.unchecked_as_ref_mut::<Keyed<DdwafObj>>()
    }
}
impl Default for bindings::ddwaf_object {
    fn default() -> Self {
        Self {
            parameterName: null(),
            parameterNameLength: Default::default(),
            __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 { uintValue: 0 },
            nbEntries: Default::default(),
            type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_INVALID,
        }
    }
}

pub trait CommonDdwafObj {
    fn get_type(&self) -> DdwafObjType;
    fn as_ddwaf_obj(&self) -> &DdwafObj;
    fn debug_str(&self, indent: i32) -> String;
}

impl<T> CommonDdwafObj for T
where
    T: AsRef<bindings::ddwaf_object>,
{
    fn get_type(&self) -> DdwafObjType {
        (self.as_ref().type_).try_into().unwrap()
    }

    fn debug_str(&self, indent: i32) -> String {
        let mut s: String = String::default();
        let raw_self = self.as_ref();
        if raw_self.parameterNameLength != 0 {
            let key = unsafe {
                slice::from_raw_parts(
                    raw_self.parameterName as *const u8,
                    raw_self.parameterNameLength as usize,
                )
            };
            s += &format!("{:?}", fmt_bin(key));
            s += ": ";
        }
        match self.get_type() {
            DdwafObjType::String => {
                let sobj = unsafe { self.as_ref().unchecked_as_ref::<DdwafObjString>() };
                s += &format!("\"{:?}\"", fmt_bin(sobj.as_slice()));
                s += "\n";
            }
            DdwafObjType::Unsigned => {
                let obj = unsafe { self.as_ref().unchecked_as_ref::<DdwafObjUnsignedInt>() };
                s += &format!("{}", obj.value());
                s += "\n";
            }
            DdwafObjType::Signed => {
                let obj = unsafe { self.as_ref().unchecked_as_ref::<DdwafObjSignedInt>() };
                s += &format!("{}", obj.value());
                s += "\n";
            }
            DdwafObjType::Float => {
                let obj = unsafe { self.as_ref().unchecked_as_ref::<DdwafObjFloat>() };
                s += &format!("{}", obj.value());
                s += "\n";
            }
            DdwafObjType::Bool => {
                let obj = unsafe { self.as_ref().unchecked_as_ref::<DdwafObjBool>() };
                s += if obj.value() { "true\n" } else { "false\n" };
            }
            DdwafObjType::Null => {
                s += "null\n";
            }
            DdwafObjType::Array => {
                let obj = unsafe { self.as_ref().unchecked_as_ref::<DdwafObjArray>() };
                if obj.is_empty() {
                    s += "[]\n";
                } else {
                    s += "\n";
                    for i in 0..obj.len() {
                        s.extend(std::iter::repeat(" ").take(indent as usize));
                        s += "- ";
                        s += &obj[i].debug_str(indent + 2);
                    }
                }
            }
            DdwafObjType::Map => {
                let obj = unsafe { self.as_ref().unchecked_as_ref::<DdwafObjMap>() };
                if obj.is_empty() {
                    s += "{}\n";
                } else {
                    s += "\n";
                    for i in 0..obj.len() {
                        s.extend(std::iter::repeat(" ").take(indent as usize));
                        s += &obj[i].debug_str(indent + 2);
                    }
                }
            }
            DdwafObjType::Invalid => {
                s += "Invalid\n";
            }
            #[allow(unreachable_patterns)]
            other_type => {
                s += format!("Unknown type {:?}", other_type).as_str();
            }
        }
        s
    }

    fn as_ddwaf_obj(&self) -> &DdwafObj {
        self.as_ref().as_ddwaf_obj_ref()
    }
}

fn fmt_bin(vec: &[u8]) -> impl std::fmt::Debug + '_ {
    struct BinFormatter<'a>(&'a [u8]);
    impl<'a> std::fmt::Debug for BinFormatter<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            for &c in self.0 {
                if c.is_ascii_graphic() || c == b' ' {
                    write!(f, "{}", c as char)?;
                } else if c == b'"' || c == b'\\' {
                    write!(f, "\\{}", c as char)?;
                } else {
                    write!(f, "\\x{:02x}", c)?;
                }
            }
            Ok(())
        }
    }

    BinFormatter(vec)
}

// keyed
impl<T: AsRef<bindings::ddwaf_object> + AsRawDdwafObjMut> Keyed<T> {
    fn new(value: T) -> Self {
        Self { value }
    }

    pub fn inner(&self) -> &T {
        self
    }

    pub fn key(&self) -> &[u8] {
        let obj = self.as_ref();
        if obj.parameterNameLength == 0 {
            return &[];
        }
        unsafe {
            std::slice::from_raw_parts(obj.parameterName.cast(), obj.parameterNameLength as usize)
        }
    }

    pub fn key_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.key())
    }

    fn set_key(&mut self, key: &[u8]) -> &mut Self {
        let obj: &mut bindings::ddwaf_object = unsafe { self.as_mut() };
        unsafe { drop_key(obj) };
        if key.is_empty() {
            obj.parameterName = null_mut();
            obj.parameterNameLength = 0;
            return self;
        }

        let b: Box<[u8]> = key.into();
        let ptr = Box::<[u8]>::into_raw(b);
        obj.parameterName = ptr.cast();
        obj.parameterNameLength = key.len() as u64;
        self
    }

    fn set_key_str(&mut self, key: &str) -> &mut Self {
        self.set_key(key.as_bytes());
        self
    }
}
impl Keyed<DdwafObj> {
    pub fn as_keyed_type<T>(&self) -> Option<&Keyed<T>>
    where
        T: AsRef<bindings::ddwaf_object> + AsRawDdwafObjMut + TypedDdwafObj,
    {
        if self.get_type() == T::TYPE {
            // SAFETY: the representation is compatible and we checked the type
            // so the correct union member will be fetched
            Some(unsafe { &*(self as *const _ as *const _) })
        } else {
            None
        }
    }

    pub fn as_keyed_type_mut<T>(&mut self) -> Option<&mut Keyed<T>>
    where
        T: AsRef<bindings::ddwaf_object> + AsRawDdwafObjMut + TypedDdwafObj,
    {
        if self.get_type() == T::TYPE {
            // SAFETY: the representation is compatible and we checked the type
            // so the correct union member will be fetched. Additionally, since
            // we're converting Keyed<DdwafObj> and Keyed<impl TypedDdwafObj>,
            // the destructors  are compatible, so swaps are safe.
            Some(unsafe { &mut *(self as *mut _ as *mut _) })
        } else {
            None
        }
    }
}
impl Keyed<DdwafObjArray> {
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut DdwafObj> {
        self.value.iter_mut()
    }
}
impl IntoIterator for Keyed<DdwafObjArray> {
    type Item = DdwafObj;
    type IntoIter = DdwafArrayIter<DdwafObj>;

    fn into_iter(mut self) -> Self::IntoIter {
        // essentially convert Keyed<DdwafObjArray> into DdwafObjArray
        let mut dobj_arr = std::mem::take(&mut self.value);
        // now the destructor of Keyed<> will no longer be called, so the
        // key would be leaked if it was not dropped
        let raw_obj = unsafe { AsRawDdwafObjMut::as_mut(&mut dobj_arr) };
        unsafe {
            drop_key(raw_obj);
        }
        raw_obj.parameterName = null_mut();
        raw_obj.parameterNameLength = 0;

        dobj_arr.into_iter()
    }
}
impl Keyed<DdwafObjMap> {
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Keyed<DdwafObj>> {
        self.value.iter_mut()
    }
}
impl IntoIterator for Keyed<DdwafObjMap> {
    type Item = Keyed<DdwafObj>;
    type IntoIter = DdwafArrayIter<Keyed<DdwafObj>>;

    fn into_iter(mut self) -> Self::IntoIter {
        // essentially convert Keyed<DdwafObjMap> into DdwafObjMap
        let mut dobj_map = std::mem::take(&mut self.value);
        // now the destructor of Keyed<> will no longer be called, so the
        // key would be leaked if it was not dropped
        let raw_obj = unsafe { AsRawDdwafObjMut::as_mut(&mut dobj_map) };
        unsafe {
            drop_key(raw_obj);
        }
        raw_obj.parameterName = null_mut();
        raw_obj.parameterNameLength = 0;

        dobj_map.into_iter()
    }
}
impl<T: AsRef<bindings::ddwaf_object> + AsRawDdwafObjMut> Deref for Keyed<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
// do not implement DerefMut as as converting from &mut Keyed<T> to &mut T
// would allow leaking the key if the T is consumed through take or replace

impl<T: AsRef<bindings::ddwaf_object> + AsRawDdwafObjMut> Drop for Keyed<T> {
    fn drop(&mut self) {
        unsafe { drop_key(self.as_mut()) };
        // self.value implicitly dropped
    }
}
impl<T: AsRef<bindings::ddwaf_object> + AsRawDdwafObjMut + std::fmt::Debug> std::fmt::Debug
    for Keyed<T>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let key = self.key();
        f.write_fmt(format_args!("\"{:?}\": {:?}", fmt_bin(key), self.value))
    }
}

/// # Safety
///
/// The key, if present, must be a raw-converted Box<[u8]>.
/// Afterwards, the object must either be discarded or parameterName /
/// parameterNameLength must be set, since they are now garbage.
pub(crate) unsafe fn drop_key(obj: &mut bindings::ddwaf_object) {
    if !obj.parameterName.is_null() {
        let slice = std::slice::from_raw_parts_mut(
            obj.parameterName as *const u8 as *mut u8,
            obj.parameterNameLength as usize,
        );
        drop(Box::from_raw(slice as *mut _));
    }
}

// ddwaf_obj
impl DdwafObj {
    pub fn as_type<T: TypedDdwafObj>(&self) -> Option<&T> {
        if self.get_type() == T::TYPE {
            Some(unsafe { self.as_ref().unchecked_as_ref::<T>() })
        } else {
            None
        }
    }
    pub fn as_type_mut<T: TypedDdwafObj + AsRawDdwafObjMut>(&mut self) -> Option<&mut T> {
        if self.get_type() == T::TYPE {
            // SAFETY: TypedDdwafObj is a closed trait; if the conversion
            // succeeds, it's guaranteed that the destructor is compatible with
            // that of DdwafObj, so a swap is safe.
            Some(unsafe { self.as_mut().unchecked_as_ref_mut::<T>() })
        } else {
            None
        }
    }
    pub fn to_u64(&self) -> Option<u64> {
        self.as_type::<DdwafObjUnsignedInt>().map(|x| x.value())
    }

    pub fn to_i64(&self) -> Option<i64> {
        match self.get_type() {
            DdwafObjType::Unsigned => {
                let obj: &DdwafObjUnsignedInt = self.as_type().unwrap();
                obj.value().try_into().ok()
            }
            DdwafObjType::Signed => {
                let obj: &DdwafObjSignedInt = self.as_type().unwrap();
                Some(obj.value())
            }
            _ => None,
        }
    }

    pub fn to_f64(&self) -> Option<f64> {
        self.as_type::<DdwafObjFloat>().map(|x| x.value())
    }

    pub fn to_bool(&self) -> Option<bool> {
        self.as_type::<DdwafObjBool>().map(|x| x.value())
    }

    pub fn to_str(&self) -> Option<&str> {
        self.as_type::<DdwafObjString>()
            .and_then(|x| x.as_str().ok())
    }
}
impl Default for DdwafObj {
    fn default() -> Self {
        Self {
            _obj: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_INVALID,
                __bindgen_anon_1: unsafe {
                    MaybeUninit::<bindings::_ddwaf_object__bindgen_ty_1>::zeroed().assume_init()
                },
                parameterName: null_mut(),
                parameterNameLength: 0,
                nbEntries: 0,
            },
        }
    }
}
impl std::fmt::Debug for DdwafObj {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.get_type() {
            DdwafObjType::Unsigned => {
                let obj: &DdwafObjUnsignedInt = self.as_type().unwrap();
                obj.fmt(f)
            }
            DdwafObjType::Signed => {
                let obj: &DdwafObjSignedInt = self.as_type().unwrap();
                obj.fmt(f)
            }
            DdwafObjType::Float => {
                let obj: &DdwafObjFloat = self.as_type().unwrap();
                obj.fmt(f)
            }
            DdwafObjType::Bool => {
                let obj: &DdwafObjBool = self.as_type().unwrap();
                obj.fmt(f)
            }
            DdwafObjType::Null => {
                let obj: &DdwafObjNull = self.as_type().unwrap();
                obj.fmt(f)
            }
            DdwafObjType::String => {
                let obj: &DdwafObjString = self.as_type().unwrap();
                obj.fmt(f)
            }
            DdwafObjType::Array => {
                let obj: &DdwafObjArray = self.as_type().unwrap();
                obj.fmt(f)
            }
            DdwafObjType::Map => {
                let obj: &DdwafObjMap = self.as_type().unwrap();
                obj.fmt(f)
            }
            _ => f.write_fmt(format_args!("DdwafObj({:?})", self.get_type())),
        }
    }
}
// can't do:
// impl<R, T> From<T> for DdwafObj
// where
//     T: Into<R> + Clone,
//     R: AsRef<bindings::ddwaf_object> + TypedDdwafObj,
// {
//     fn from(value: T) -> Self {
//         let r: R = value.into();
//         r.into()
//     }
// }
impl From<u64> for DdwafObj {
    fn from(value: u64) -> Self {
        DdwafObjUnsignedInt::from(value).into()
    }
}
impl From<i64> for DdwafObj {
    fn from(value: i64) -> Self {
        DdwafObjSignedInt::from(value).into()
    }
}
// TODO: use crate for number traits
impl From<i32> for DdwafObj {
    fn from(value: i32) -> Self {
        DdwafObjSignedInt::from(value as i64).into()
    }
}
impl From<f64> for DdwafObj {
    fn from(value: f64) -> Self {
        DdwafObjFloat::from(value).into()
    }
}
impl From<bool> for DdwafObj {
    fn from(value: bool) -> Self {
        DdwafObjBool::from(value).into()
    }
}
impl From<&str> for DdwafObj {
    fn from(value: &str) -> Self {
        DdwafObjString::from(value).into()
    }
}
impl From<&[u8]> for DdwafObj {
    fn from(value: &[u8]) -> Self {
        DdwafObjString::from(value).into()
    }
}
impl From<()> for DdwafObj {
    fn from(_: ()) -> Self {
        let ret: DdwafObjNull = ().into();
        ret.into()
    }
}
impl<T, U> From<(&str, T)> for Keyed<U>
where
    T: Into<U>,
    U: AsRef<bindings::ddwaf_object> + AsRawDdwafObjMut,
{
    fn from(value: (&str, T)) -> Self {
        let unkeyed = value.1.into();
        let mut keyed = Keyed::new(unkeyed);
        keyed.set_key_str(value.0);
        keyed
    }
}

impl<T, U> From<(&[u8], T)> for Keyed<U>
where
    T: Into<U>,
    U: AsRef<bindings::ddwaf_object> + AsRawDdwafObjMut,
{
    fn from(value: (&[u8], T)) -> Self {
        let unkeyed = value.1.into();
        let mut keyed = Keyed::new(unkeyed);
        keyed.set_key(value.0);
        keyed
    }
}

impl<T> From<Keyed<T>> for Keyed<DdwafObj>
where
    T: AsRef<bindings::ddwaf_object> + AsRawDdwafObjMut + TypedDdwafObj,
{
    fn from(value: Keyed<T>) -> Self {
        let dobj = DdwafObj {
            _obj: *value.as_ref(),
        };
        std::mem::forget(value);
        Self { value: dobj }
    }
}

impl<T> From<T> for DdwafObj
where
    T: AsRef<bindings::ddwaf_object> + TypedDdwafObj,
{
    fn from(value: T) -> Self {
        let res = Self {
            _obj: *value.as_ref(),
        };
        std::mem::forget(value);
        res
    }
}

/// # Safety
///
/// If the object is a string, array or map, the requirements of the [`drop_ddwaf_object_string`],
/// [`drop_ddwaf_object_array`], or [`drop_ddwaf_object_map`].
unsafe fn drop_ddwaf_object(obj: &mut bindings::ddwaf_object) {
    if obj.type_ == bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_STRING {
        drop_ddwaf_object_string(obj);
    } else if obj.type_ == bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_ARRAY {
        drop_ddwaf_object_array(obj);
    } else if obj.type_ == bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_MAP {
        drop_ddwaf_object_map(obj);
    }
}
impl Drop for DdwafObj {
    fn drop(&mut self) {
        unsafe { drop_ddwaf_object(&mut self._obj) }
    }
}

// unsigned int
impl DdwafObjUnsignedInt {
    pub fn new(value: u64) -> Self {
        Self {
            _obj: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_UNSIGNED,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 { uintValue: value },
                ..Default::default()
            },
        }
    }
    pub fn value(&self) -> u64 {
        unsafe { self._obj.__bindgen_anon_1.uintValue }
    }
}
impl Default for DdwafObjUnsignedInt {
    fn default() -> Self {
        Self::new(0u64)
    }
}
impl private::Sealed for DdwafObjUnsignedInt {}
impl TypedDdwafObj for DdwafObjUnsignedInt {
    const TYPE: DdwafObjType = DdwafObjType::Unsigned;
}
impl std::fmt::Debug for DdwafObjUnsignedInt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("DdwafObjUnsignedInt({})", self.value()))
    }
}
impl From<u64> for DdwafObjUnsignedInt {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}
impl TryFrom<DdwafObj> for DdwafObjUnsignedInt {
    type Error = DdwafObjTypeError;
    fn try_from(value: DdwafObj) -> Result<Self, Self::Error> {
        if value.get_type() != DdwafObjType::Unsigned {
            return Err(Self::Error {
                message: "Invalid DDWAFObjType value (not an unsigned int)",
            });
        }
        let res = Ok(Self { _obj: value._obj });
        std::mem::forget(value);
        res
    }
}

// signed int
impl DdwafObjSignedInt {
    pub fn new(value: i64) -> Self {
        Self {
            _obj: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_SIGNED,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 { intValue: value },
                ..Default::default()
            },
        }
    }
    pub fn value(&self) -> i64 {
        unsafe { self._obj.__bindgen_anon_1.intValue }
    }
}
impl Default for DdwafObjSignedInt {
    fn default() -> Self {
        Self::new(0i64)
    }
}
impl private::Sealed for DdwafObjSignedInt {}
impl TypedDdwafObj for DdwafObjSignedInt {
    const TYPE: DdwafObjType = DdwafObjType::Signed;
}
impl std::fmt::Debug for DdwafObjSignedInt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("DdwafObjSignedInt({})", self.value()))
    }
}
impl From<i64> for DdwafObjSignedInt {
    fn from(value: i64) -> Self {
        Self::new(value)
    }
}
impl TryFrom<DdwafObj> for DdwafObjSignedInt {
    type Error = DdwafObjTypeError;
    fn try_from(value: DdwafObj) -> Result<Self, Self::Error> {
        if value.get_type() != DdwafObjType::Signed {
            return Err(Self::Error {
                message: "Invalid DDWAFObjType value (not a signed int)",
            });
        }
        let res = Ok(Self { _obj: value._obj });
        std::mem::forget(value);
        res
    }
}

// float
impl DdwafObjFloat {
    pub fn new(value: f64) -> Self {
        Self {
            _obj: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_FLOAT,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 { f64_: value },
                ..Default::default()
            },
        }
    }

    pub fn value(&self) -> f64 {
        unsafe { self._obj.__bindgen_anon_1.f64_ }
    }
}
impl Default for DdwafObjFloat {
    fn default() -> Self {
        Self::new(0.0)
    }
}
impl private::Sealed for DdwafObjFloat {}
impl TypedDdwafObj for DdwafObjFloat {
    const TYPE: DdwafObjType = DdwafObjType::Float;
}
impl std::fmt::Debug for DdwafObjFloat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("DdwafObjFloat({})", self.value()))
    }
}
impl From<f64> for DdwafObjFloat {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}
impl TryFrom<DdwafObj> for DdwafObjFloat {
    type Error = DdwafObjTypeError;
    fn try_from(value: DdwafObj) -> Result<Self, Self::Error> {
        if value.get_type() != DdwafObjType::Float {
            return Err(Self::Error {
                message: "Invalid DDWAFObjType value (not a floating point number)",
            });
        }
        let res = Ok(Self { _obj: value._obj });
        std::mem::forget(value);
        res
    }
}

// bool
impl DdwafObjBool {
    pub fn new(value: bool) -> Self {
        Self {
            _obj: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_BOOL,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 { boolean: value },
                ..Default::default()
            },
        }
    }
    pub fn value(&self) -> bool {
        unsafe { self._obj.__bindgen_anon_1.boolean }
    }
}
impl Default for DdwafObjBool {
    fn default() -> Self {
        Self::new(false)
    }
}
impl private::Sealed for DdwafObjBool {}
impl TypedDdwafObj for DdwafObjBool {
    const TYPE: DdwafObjType = DdwafObjType::Bool;
}
impl std::fmt::Debug for DdwafObjBool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("DdwafObjBool({})", self.value()))
    }
}
impl From<bool> for DdwafObjBool {
    fn from(value: bool) -> Self {
        Self::new(value)
    }
}
impl TryFrom<DdwafObj> for DdwafObjBool {
    type Error = DdwafObjTypeError;
    fn try_from(value: DdwafObj) -> Result<Self, Self::Error> {
        if value.get_type() != DdwafObjType::Bool {
            return Err(Self::Error {
                message: "Invalid DDWAFObjType value (not a bool)",
            });
        }
        let res = Ok(Self { _obj: value._obj });
        std::mem::forget(value);
        res
    }
}

// null
impl private::Sealed for DdwafObjNull {}
impl TypedDdwafObj for DdwafObjNull {
    const TYPE: DdwafObjType = DdwafObjType::Null;
}
impl DdwafObjNull {
    pub fn new() -> Self {
        Self {
            _obj: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_NULL,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 {
                    uintValue: 0, // any would do
                },
                ..Default::default()
            },
        }
    }
}
impl Default for DdwafObjNull {
    fn default() -> Self {
        Self::new()
    }
}
impl std::fmt::Debug for DdwafObjNull {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DdwafObjNull")
    }
}
impl From<()> for DdwafObjNull {
    fn from(_: ()) -> Self {
        DdwafObjNull::new()
    }
}
impl TryFrom<DdwafObj> for DdwafObjNull {
    type Error = DdwafObjTypeError;
    fn try_from(value: DdwafObj) -> Result<Self, Self::Error> {
        if value.get_type() != DdwafObjType::Null {
            return Err(Self::Error {
                message: "Invalid DDWAFObjType value (not a null)",
            });
        }
        let res = Ok(Self { _obj: value._obj });
        std::mem::forget(value);
        res
    }
}

// string
impl DdwafObjString {
    pub fn new(value: &[u8]) -> Self {
        let mem = if value.is_empty() {
            null_mut()
        } else {
            let boxed_slice: Box<[u8]> = value.into();
            Box::into_raw(boxed_slice).cast()
        };

        Self {
            _obj: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_STRING,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 {
                    stringValue: mem as *const std::os::raw::c_char,
                },
                nbEntries: value.len() as u64,
                ..Default::default()
            },
        }
    }

    pub fn len(&self) -> usize {
        self._obj.nbEntries as usize
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_slice(&self) -> &[u8] {
        let obj = self.as_ref();
        let sval = unsafe { obj.__bindgen_anon_1.stringValue };
        let sval_len = obj.nbEntries;
        unsafe { std::slice::from_raw_parts(sval as *const u8, sval_len as usize) }
    }

    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.as_slice())
    }
}
impl Default for DdwafObjString {
    fn default() -> Self {
        Self::new(&[])
    }
}
impl private::Sealed for DdwafObjString {}
impl TypedDdwafObj for DdwafObjString {
    const TYPE: DdwafObjType = DdwafObjType::String;
}
impl std::fmt::Debug for DdwafObjString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "DdwafObjString(\"{:?}\")",
            fmt_bin(self.as_slice())
        ))
    }
}
impl From<&str> for DdwafObjString {
    fn from(value: &str) -> Self {
        Self::new(value.as_bytes())
    }
}
impl From<&[u8]> for DdwafObjString {
    fn from(value: &[u8]) -> Self {
        Self::new(value)
    }
}
impl TryFrom<DdwafObj> for DdwafObjString {
    type Error = DdwafObjTypeError;
    fn try_from(value: DdwafObj) -> Result<Self, Self::Error> {
        if value.get_type() != DdwafObjType::String {
            return Err(Self::Error {
                message: "Invalid DDWAFObjType value (not a string)",
            });
        }
        let res = Ok(Self { _obj: value._obj });
        std::mem::forget(value);
        res
    }
}

/// # Safety
///
/// - The ddwaf_object must be a a valid representation of a string.
/// - The stringValue must be a raw-converted Box<[u8]>.
unsafe fn drop_ddwaf_object_string(obj: &mut bindings::ddwaf_object) {
    debug_assert!(obj.type_ == bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_STRING);
    let sval = obj.__bindgen_anon_1.stringValue;
    let sval_len = obj.nbEntries;
    if !sval.is_null() {
        let slice: &mut [u8] = std::slice::from_raw_parts_mut(sval as *mut _, sval_len as usize);
        drop(Box::from_raw(slice));
    }
}
impl Drop for DdwafObjString {
    fn drop(&mut self) {
        unsafe { drop_ddwaf_object_string(&mut self._obj) }
    }
}

// array
impl DdwafObjArray {
    pub fn new(size: u64) -> Self {
        let layout = Layout::array::<bindings::ddwaf_object>(size as usize).unwrap();
        let array = unsafe { no_fail_alloc(layout) } as *mut bindings::ddwaf_object;
        if !array.is_null() {
            // zero initialize the allocated memory to avoid UB in drop()
            unsafe {
                std::ptr::write_bytes(array, 0, size as usize);
            }
        }
        Self {
            _obj: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_ARRAY,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 { array },
                nbEntries: size,
                ..Default::default()
            },
        }
    }

    pub fn len(&self) -> usize {
        self._obj.nbEntries as usize
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> std::slice::Iter<'_, DdwafObj> {
        let slice: &[DdwafObj] = self.as_ref();
        slice.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, DdwafObj> {
        let slice: &mut [DdwafObj] = AsMut::as_mut(self);
        slice.iter_mut()
    }
}
impl Default for DdwafObjArray {
    fn default() -> Self {
        Self::new(0)
    }
}
impl private::Sealed for DdwafObjArray {}
impl TypedDdwafObj for DdwafObjArray {
    const TYPE: DdwafObjType = DdwafObjType::Array;
}
impl Index<usize> for DdwafObjArray {
    type Output = DdwafObj;
    fn index(&self, index: usize) -> &Self::Output {
        let obj: &bindings::ddwaf_object = self.as_ref();
        let array = unsafe { obj.__bindgen_anon_1.array };
        let array_len = obj.nbEntries;
        if index >= array_len as usize {
            panic!("Index out of bounds");
        }
        let elem = unsafe { array.add(index) };
        unsafe { &*(elem as *const DdwafObj) }
    }
}
impl IndexMut<usize> for DdwafObjArray {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let obj: &bindings::ddwaf_object = self.as_ref();
        let array = unsafe { obj.__bindgen_anon_1.array };
        let array_len = obj.nbEntries;
        if index >= array_len as usize {
            panic!("Index out of bounds");
        }
        let elem = unsafe { array.add(index) };
        unsafe { &mut *(elem as *mut DdwafObj) }
    }
}
impl AsRef<[DdwafObj]> for DdwafObjArray {
    fn as_ref(&self) -> &[DdwafObj] {
        if self.is_empty() {
            return &[];
        }
        let array = unsafe { self._obj.__bindgen_anon_1.array };
        unsafe { std::slice::from_raw_parts(array as *const DdwafObj, self.len()) }
    }
}
impl AsMut<[DdwafObj]> for DdwafObjArray {
    fn as_mut(&mut self) -> &mut [DdwafObj] {
        if self.is_empty() {
            return &mut [];
        }
        let array = unsafe { self._obj.__bindgen_anon_1.array };
        unsafe { std::slice::from_raw_parts_mut(array as *mut _, self.len()) }
    }
}
pub struct DdwafArrayIter<T> {
    // we take ownership of this pointer.
    // Don't convert to Box<[T]>; while dropping both the array and its elements
    // would become automatic, we we would lose the ability to call drop on only
    // the elements that were not iterated over
    array: NonNull<T>,
    len: usize,
    pos: usize,
}
impl<T> std::iter::Iterator for DdwafArrayIter<T>
where
    T: Default,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.len {
            return None;
        }

        let obj = unsafe { &mut *self.array.add(self.pos).as_ptr() };
        self.pos += 1;
        Some(std::mem::take(obj))
    }
}
impl<T> Drop for DdwafArrayIter<T> {
    fn drop(&mut self) {
        for i in self.pos..self.len {
            let elem = unsafe { self.array.add(i) };
            unsafe { elem.drop_in_place() };
        }
        if self.len != 0 {
            let layout = Layout::array::<T>(self.len).unwrap();
            unsafe { std::alloc::dealloc(self.array.as_ptr() as *mut _, layout) }
        }
    }
}
impl IntoIterator for DdwafObjArray {
    type Item = DdwafObj;
    type IntoIter = DdwafArrayIter<DdwafObj>;

    fn into_iter(self) -> Self::IntoIter {
        let array_ptr = unsafe { self._obj.__bindgen_anon_1.array };
        let array = if array_ptr.is_null() {
            NonNull::<DdwafObj>::dangling()
        } else {
            unsafe { NonNull::new_unchecked(array_ptr as *mut _) }
        };
        let len = self.len();
        std::mem::forget(self);
        DdwafArrayIter { array, len, pos: 0 }
    }
}
impl std::fmt::Debug for DdwafObjArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DdwafObjArray{")?;
        for elem in self.iter() {
            f.write_fmt(format_args!("{:?}, ", elem))?;
        }
        f.write_char('}')
    }
}
impl TryFrom<DdwafObj> for DdwafObjArray {
    type Error = DdwafObjTypeError;
    fn try_from(value: DdwafObj) -> Result<Self, Self::Error> {
        if value.get_type() != DdwafObjType::Array {
            return Err(Self::Error {
                message: "Invalid DDWAFObjType value (not an array)",
            });
        }
        let res = Ok(Self { _obj: value._obj });
        std::mem::forget(value);
        res
    }
}

/// # Safety
///
/// - The ddwaf_object must be a a valid representation of an array.
/// - The array must be an [`std::alloc::alloc`]ated array of ddwaf_object of the proper size.
/// - The individual elements of the array must be valid `ddwaf_object`s that can be dropped with
///   [`drop_ddwaf_object`].
unsafe fn drop_ddwaf_object_array(obj: &mut bindings::ddwaf_object) {
    debug_assert!(obj.type_ == bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_ARRAY);
    let array = obj.__bindgen_anon_1.array;
    let array_len = obj.nbEntries;
    if array_len == 0 {
        return;
    }
    for i in 0..array_len {
        let elem = array.offset(i as isize);
        drop_ddwaf_object(&mut *elem);
    }
    let layout = Layout::array::<bindings::ddwaf_object>(array_len as usize).unwrap();
    std::alloc::dealloc(array as *mut u8, layout);
}
impl Drop for DdwafObjArray {
    fn drop(&mut self) {
        unsafe { drop_ddwaf_object_array(&mut self._obj) }
    }
}

// map
impl DdwafObjMap {
    pub fn new(size: u64) -> Self {
        let layout = Layout::array::<bindings::ddwaf_object>(size as usize).unwrap();
        let array = unsafe { no_fail_alloc(layout) } as *mut bindings::ddwaf_object;
        if !array.is_null() {
            unsafe {
                std::ptr::write_bytes(array, 0, size as usize);
            }
        }
        Self {
            _obj: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_MAP,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 { array },
                nbEntries: size,
                ..Default::default()
            },
        }
    }

    pub fn len(&self) -> usize {
        let obj: &bindings::ddwaf_object = self.as_ref();
        obj.nbEntries as usize
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, key: &[u8]) -> Option<&Keyed<DdwafObj>> {
        let array = unsafe { self._obj.__bindgen_anon_1.array };
        let array_len = self.len();
        for i in 0..array_len {
            let elem_ptr = unsafe { array.add(i) };
            let elem = unsafe { &*elem_ptr }.as_keyed_ddwaf_obj_ref();
            if elem.key() == key {
                return Some(elem);
            }
        }
        None
    }

    pub fn get_str(&self, key: &str) -> Option<&Keyed<DdwafObj>> {
        self.get(key.as_bytes())
    }

    pub fn get_mut(&mut self, key: &[u8]) -> Option<&mut Keyed<DdwafObj>> {
        let array = unsafe { self._obj.__bindgen_anon_1.array };
        let array_len = self.len();
        for i in 0..array_len {
            let elem_ptr = unsafe { array.add(i) };
            // SAFETY: elem_ptr is a valid pointer to memory with a valid representation
            // of a Keyed<DdwafObj> object.
            let elem = unsafe { (*elem_ptr).as_keyed_ddwaf_obj_ref_mut() };
            if elem.key() == key {
                return Some(elem);
            }
        }
        None
    }

    pub fn get_str_mut(&mut self, key: &str) -> Option<&mut Keyed<DdwafObj>> {
        self.get_mut(key.as_bytes())
    }

    pub fn iter(&self) -> impl Iterator<Item = &Keyed<DdwafObj>> {
        let slice: &[Keyed<DdwafObj>] = self.as_ref();
        slice.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Keyed<DdwafObj>> {
        let slice: &mut [Keyed<DdwafObj>] = AsMut::as_mut(self);
        slice.iter_mut()
    }
}
impl Default for DdwafObjMap {
    fn default() -> Self {
        Self::new(0)
    }
}
impl private::Sealed for DdwafObjMap {}
impl TypedDdwafObj for DdwafObjMap {
    const TYPE: DdwafObjType = DdwafObjType::Map;
}
impl AsRef<[Keyed<DdwafObj>]> for DdwafObjMap {
    fn as_ref(&self) -> &[Keyed<DdwafObj>] {
        if self.is_empty() {
            return &[];
        }
        let array = unsafe { self._obj.__bindgen_anon_1.array };
        unsafe { std::slice::from_raw_parts(array as *const Keyed<DdwafObj>, self.len()) }
    }
}
impl AsMut<[Keyed<DdwafObj>]> for DdwafObjMap {
    fn as_mut(&mut self) -> &mut [Keyed<DdwafObj>] {
        if self.is_empty() {
            return &mut [];
        }
        let array = unsafe { self._obj.__bindgen_anon_1.array };
        unsafe { std::slice::from_raw_parts_mut(array as *mut _, self.len()) }
    }
}
impl Index<usize> for DdwafObjMap {
    type Output = Keyed<DdwafObj>;
    fn index(&self, index: usize) -> &Self::Output {
        let obj: &bindings::ddwaf_object = self.as_ref();
        let array = unsafe { obj.__bindgen_anon_1.array };
        let array_len = obj.nbEntries;
        if index >= array_len as usize {
            panic!("Index out of bounds");
        }
        let elem = unsafe { &*array.add(index) };
        elem.as_keyed_ddwaf_obj_ref()
    }
}
impl IndexMut<usize> for DdwafObjMap {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let obj: &bindings::ddwaf_object = self.as_ref();
        let array = unsafe { obj.__bindgen_anon_1.array };
        let array_len = obj.nbEntries;
        if index >= array_len as usize {
            panic!("Index out of bounds");
        }
        let elem = unsafe { &mut *array.add(index) };
        unsafe { elem.as_keyed_ddwaf_obj_ref_mut() }
    }
}
impl IntoIterator for DdwafObjMap {
    type Item = Keyed<DdwafObj>;
    type IntoIter = DdwafArrayIter<Keyed<DdwafObj>>;

    fn into_iter(self) -> Self::IntoIter {
        let array_ptr = unsafe { self._obj.__bindgen_anon_1.array };
        let array = if array_ptr.is_null() {
            NonNull::<Keyed<DdwafObj>>::dangling()
        } else {
            unsafe { NonNull::new_unchecked(array_ptr as *mut _) }
        };
        let len = self.len();
        std::mem::forget(self);
        DdwafArrayIter { array, len, pos: 0 }
    }
}
impl std::fmt::Debug for DdwafObjMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DdwafObjMap{")?;
        for keyed_obj in self.iter() {
            f.write_fmt(format_args!("{:?}, ", keyed_obj))?;
        }
        f.write_char('}')
    }
}
impl TryFrom<DdwafObj> for DdwafObjMap {
    type Error = DdwafObjTypeError;
    fn try_from(value: DdwafObj) -> Result<Self, Self::Error> {
        if value.get_type() != DdwafObjType::Map {
            return Err(Self::Error {
                message: "Invalid DDWAFObjType value (not a map)",
            });
        }
        let res = Ok(Self { _obj: value._obj });
        std::mem::forget(value);
        res
    }
}

/// # Safety
///
/// - The ddwaf_object must be a a valid representation of a map.
/// - The array must be an [`std::alloc::alloc`]ated array of ddwaf_object of the proper size.
/// - The individual elements of the map must be valid `ddwaf_object`s that can be dropped with
///   [`drop_ddwaf_object`] and [`drop_key`].
unsafe fn drop_ddwaf_object_map(obj: &mut bindings::ddwaf_object) {
    debug_assert!(obj.type_ == bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_MAP);
    let array = obj.__bindgen_anon_1.array;
    let array_len = obj.nbEntries;
    if array_len == 0 {
        return;
    }
    for i in 0..array_len {
        let elem = &mut *array.offset(i as isize);
        drop_key(elem);
        drop_ddwaf_object(elem);
    }
    let layout = Layout::array::<bindings::ddwaf_object>(array_len as usize).unwrap();
    std::alloc::dealloc(array as *mut u8, layout);
}
impl Drop for DdwafObjMap {
    fn drop(&mut self) {
        unsafe { drop_ddwaf_object_map(&mut self._obj) }
    }
}

pub struct WafOwnedDdwafObj {
    _inner: std::mem::ManuallyDrop<DdwafObj>,
}
impl Default for WafOwnedDdwafObj {
    fn default() -> Self {
        Self {
            _inner: std::mem::ManuallyDrop::new(DdwafObj::default()),
        }
    }
}
impl Drop for WafOwnedDdwafObj {
    fn drop(&mut self) {
        unsafe { bindings::ddwaf_object_free(&mut self._inner._obj) }
    }
}
impl Deref for WafOwnedDdwafObj {
    type Target = DdwafObj;

    fn deref(&self) -> &Self::Target {
        &self._inner
    }
}
impl DerefMut for WafOwnedDdwafObj {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self._inner
    }
}

pub fn get_version() -> &'static CStr {
    unsafe { CStr::from_ptr(bindings::ddwaf_get_version()) }
}

pub struct DdwafConfig {
    _cfg: bindings::ddwaf_config,
    obfuscator: Obfuscator,
}
impl DdwafConfig {
    pub fn new(limits: Limits, obfuscator: Obfuscator) -> Self {
        Self {
            _cfg: bindings::ddwaf_config {
                limits,
                obfuscator: obfuscator._raw_obfuscator,
                free_fn: None,
            },
            obfuscator,
        }
    }
}
impl Default for DdwafConfig {
    fn default() -> Self {
        Self::new(Limits::default(), Obfuscator::default())
    }
}
impl Clone for DdwafConfig {
    fn clone(&self) -> Self {
        let limits = self._cfg.limits;
        let obfuscator = self.obfuscator.clone();
        Self::new(limits, obfuscator)
    }
}

pub type Limits = bindings::_ddwaf_config__ddwaf_config_limits;
#[allow(clippy::derivable_impls)]
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_container_size: 0,
            max_container_depth: 0,
            max_string_length: 0,
        }
    }
}

pub struct Obfuscator {
    _raw_obfuscator: bindings::_ddwaf_config__ddwaf_config_obfuscator,
}

impl Default for Obfuscator {
    fn default() -> Self {
        Obfuscator::new::<Vec<u8>, Vec<u8>>(None, None)
    }
}
impl Obfuscator {
    pub fn new<T: Into<Vec<u8>>, U: Into<Vec<u8>>>(
        key_regex: Option<T>,
        value_regex: Option<U>,
    ) -> Self {
        let key_regex = key_regex
            .map(|s| CString::new(s).expect("Invalid key regex").into_raw())
            .unwrap_or(null_mut());
        let value_regex = value_regex
            .map(|s| CString::new(s).expect("Invalid value regex").into_raw())
            .unwrap_or(null_mut());
        Self {
            _raw_obfuscator: bindings::_ddwaf_config__ddwaf_config_obfuscator {
                key_regex,
                value_regex,
            },
        }
    }

    pub fn key_regex(&self) -> Option<&CStr> {
        if self._raw_obfuscator.key_regex.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(self._raw_obfuscator.key_regex) })
        }
    }

    pub fn value_regex(&self) -> Option<&CStr> {
        if self._raw_obfuscator.value_regex.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(self._raw_obfuscator.value_regex) })
        }
    }
}
impl Clone for Obfuscator {
    fn clone(&self) -> Self {
        // &CStr is not directly convertible to Vec<u8>
        let key = self.key_regex().map(|x| x.to_bytes());
        let value = self.value_regex().map(|x| x.to_bytes());
        Self::new(key, value)
    }
}
impl Drop for Obfuscator {
    fn drop(&mut self) {
        unsafe {
            if !self._raw_obfuscator.key_regex.is_null() {
                drop(CString::from_raw(self._raw_obfuscator.key_regex as *mut _));
            }
            if !self._raw_obfuscator.value_regex.is_null() {
                drop(CString::from_raw(
                    self._raw_obfuscator.value_regex as *mut _,
                ));
            }
        }
    }
}

pub struct WafInstance {
    _handle: bindings::ddwaf_handle,
    _config: Option<DdwafConfig>, // for holding memory only
}
impl Drop for WafInstance {
    fn drop(&mut self) {
        unsafe { bindings::ddwaf_destroy(self._handle) }
    }
}

// SAFETY: ddwaf instances are effectively immutable
unsafe impl Send for WafInstance {}
unsafe impl Sync for WafInstance {}

impl WafInstance {
    pub fn new<T: AsRef<bindings::ddwaf_object>>(
        ruleset: &T,
        config: DdwafConfig,
        diagnostics: Option<&mut WafOwnedDdwafObj>,
    ) -> Result<Self, DdwafGenericError> {
        let handle = unsafe {
            bindings::ddwaf_init(
                ruleset.as_ref(),
                &config._cfg,
                diagnostics
                    .map(|d| &mut d._inner._obj as *mut bindings::ddwaf_object)
                    .unwrap_or(null_mut()),
            )
        };

        if handle.is_null() {
            return Err("Failed to initialize handle".into());
        }
        Ok(Self {
            _handle: handle,
            _config: Some(config),
        })
    }

    pub fn create_context(&self) -> WafContext {
        WafContext {
            _ctx: unsafe { bindings::ddwaf_context_init(self._handle) },
            _owned_objs: Vec::new(),
        }
    }

    pub fn known_actions(&mut self) -> Vec<&CStr> {
        self.call_c_str_arr_fun(bindings::ddwaf_known_actions)
    }

    pub fn known_addresses(&mut self) -> Vec<&CStr> {
        self.call_c_str_arr_fun(bindings::ddwaf_known_addresses)
    }

    fn call_c_str_arr_fun(
        &mut self,
        f: unsafe extern "C" fn(
            *mut bindings::_ddwaf_handle,
            *mut u32,
        ) -> *const *const std::os::raw::c_char,
    ) -> Vec<&CStr> {
        // function is not thread-safe, so we need an exclusive reference
        let mut size = std::mem::MaybeUninit::<u32>::uninit();
        let raw = unsafe { f(self._handle, size.as_mut_ptr()) };
        if raw.is_null() {
            return vec![];
        }

        let size = unsafe { size.assume_init() as usize };
        let arr = unsafe { std::slice::from_raw_parts(raw, size) };
        arr.iter().map(|&x| unsafe { CStr::from_ptr(x) }).collect()
    }
}

pub struct WafContext {
    _ctx: bindings::ddwaf_context,
    _owned_objs: Vec<DdwafObjMap>,
}
impl WafContext {
    pub fn run(
        &mut self,
        mut persistent_data: Option<DdwafObjMap>,
        ephemeral_data: Option<&DdwafObjMap>,
        timeout: std::time::Duration,
    ) -> WafRunResult {
        let mut dres = std::mem::MaybeUninit::<DdwafResult>::uninit();
        let persistent_ref = persistent_data
            .as_mut()
            .map_or(null_mut(), |r| r as *mut DdwafObjMap);
        // we can cast away thte constness because ddwaf_run does not modify/free the data;
        // in fact, it should take a const pointer
        let ephemeral_ref =
            ephemeral_data.map_or(null_mut(), |r| r as *const DdwafObjMap as *mut DdwafObjMap);
        let res = unsafe {
            bindings::ddwaf_run(
                self._ctx,
                persistent_ref as *mut bindings::ddwaf_object,
                ephemeral_ref as *mut bindings::ddwaf_object,
                dres.as_mut_ptr() as *mut bindings::ddwaf_result,
                timeout.as_micros().try_into().unwrap_or(u64::MAX),
            )
        };

        match res {
            bindings::DDWAF_RET_CODE_DDWAF_ERR_INTERNAL => {
                // leak input data because according to the docs, it's not clear who owns it
                std::mem::forget(persistent_data);
                WafRunResult::InternalError
            }
            bindings::DDWAF_RET_CODE_DDWAF_ERR_INVALID_OBJECT => WafRunResult::InvalidObject,
            bindings::DDWAF_RET_CODE_DDWAF_ERR_INVALID_ARGUMENT => WafRunResult::InvalidArgument,
            bindings::DDWAF_RET_CODE_DDWAF_OK => {
                if let Some(obj) = persistent_data {
                    self._owned_objs.push(obj);
                }
                WafRunResult::Ok(unsafe { dres.assume_init() })
            }
            bindings::DDWAF_RET_CODE_DDWAF_MATCH => {
                if let Some(obj) = persistent_data {
                    self._owned_objs.push(obj);
                }
                WafRunResult::Match(unsafe { dres.assume_init() })
            }
            _ => unreachable!("Unexpected result from ddwaf_run"),
        }
    }
}
// Safety: WafContext only changes internal state in operations that require
// an exclusive (mutable) reference, and does not expose any internal state.
// WafContext is also trivially Sync because it contains no methods taking
// shared references.
unsafe impl Send for WafContext {}
impl Drop for WafContext {
    fn drop(&mut self) {
        unsafe { bindings::ddwaf_context_destroy(self._ctx) }
    }
}

/// Sets the log callback function.
///
/// # Safety
///
/// This function is unsafe because it writes to a static variable without synchronization.
/// It should only be used during startup.
pub unsafe fn set_log_cb<
    F: Fn(DdwafLogLevel, &'static CStr, &'static CStr, u32, &[std::os::raw::c_char]) + 'static,
>(
    cb: Option<F>,
    min_level: DdwafLogLevel,
) {
    match cb {
        Some(cb) => unsafe {
            LOG_CB = Some(Box::new(cb));
            bindings::ddwaf_set_log_cb(Some(bridge_log_cb), min_level as u32);
        },
        None => unsafe {
            bindings::ddwaf_set_log_cb(None, DdwafLogLevel::Off as u32);
            LOG_CB = None;
        },
    }
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DdwafLogLevel {
    Trace = bindings::DDWAF_LOG_LEVEL_DDWAF_LOG_TRACE,
    Debug = bindings::DDWAF_LOG_LEVEL_DDWAF_LOG_DEBUG,
    Info = bindings::DDWAF_LOG_LEVEL_DDWAF_LOG_INFO,
    Warn = bindings::DDWAF_LOG_LEVEL_DDWAF_LOG_WARN,
    Error = bindings::DDWAF_LOG_LEVEL_DDWAF_LOG_ERROR,
    Off = bindings::DDWAF_LOG_LEVEL_DDWAF_LOG_OFF,
}

impl TryFrom<u32> for DdwafLogLevel {
    type Error = DdwafObjTypeError;

    fn try_from(value: u32) -> Result<Self, DdwafObjTypeError> {
        match value {
            bindings::DDWAF_LOG_LEVEL_DDWAF_LOG_TRACE => Ok(DdwafLogLevel::Trace),
            bindings::DDWAF_LOG_LEVEL_DDWAF_LOG_DEBUG => Ok(DdwafLogLevel::Debug),
            bindings::DDWAF_LOG_LEVEL_DDWAF_LOG_INFO => Ok(DdwafLogLevel::Info),
            bindings::DDWAF_LOG_LEVEL_DDWAF_LOG_WARN => Ok(DdwafLogLevel::Warn),
            bindings::DDWAF_LOG_LEVEL_DDWAF_LOG_ERROR => Ok(DdwafLogLevel::Error),
            bindings::DDWAF_LOG_LEVEL_DDWAF_LOG_OFF => Ok(DdwafLogLevel::Off),
            _ => Err(DdwafObjTypeError {
                message: "Unexpected log level",
            }),
        }
    }
}

type LogCallback =
    Box<dyn Fn(DdwafLogLevel, &'static CStr, &'static CStr, u32, &[std::os::raw::c_char])>;

static mut LOG_CB: Option<LogCallback> = None;

extern "C" fn bridge_log_cb(
    level: bindings::DDWAF_LOG_LEVEL,
    file: *const std::os::raw::c_char,
    function: *const std::os::raw::c_char,
    line: u32,
    message: *const std::os::raw::c_char,
    message_len: u64,
) {
    unsafe {
        let file = CStr::from_ptr(file);
        let function = CStr::from_ptr(function);
        let message = slice::from_raw_parts(message, message_len.try_into().unwrap());
        if let Some(cb) = &*addr_of_mut!(LOG_CB) {
            cb(
                DdwafLogLevel::try_from(level).unwrap_or(DdwafLogLevel::Error),
                file,
                function,
                line,
                message,
            );
        }
    }
}

#[derive(Debug)]
pub enum WafRunResult {
    InternalError,
    InvalidObject,
    InvalidArgument,
    Ok(DdwafResult),
    Match(DdwafResult),
}

#[repr(C)]
pub struct DdwafResult {
    _result: bindings::ddwaf_result,
}
impl DdwafResult {
    pub fn is_timeout(&self) -> bool {
        self._result.timeout
    }
    pub fn events(&self) -> &DdwafObjArray {
        // ddwaf_result does not escape without having been written
        // (no public methods to construct it), so this is guaranteed
        // (to the outside) to be an array
        unsafe { self._result.events.unchecked_as_ref() }
    }

    pub fn actions(&self) -> &DdwafObjMap {
        unsafe { self._result.actions.unchecked_as_ref() }
    }

    pub fn derivatives(&self) -> &DdwafObjMap {
        unsafe { self._result.derivatives.unchecked_as_ref() }
    }

    pub fn runtime(&self) -> std::time::Duration {
        std::time::Duration::from_nanos(self._result.total_runtime)
    }
}
impl Drop for DdwafResult {
    fn drop(&mut self) {
        unsafe { bindings::ddwaf_result_free(&mut self._result) }
    }
}
impl std::fmt::Debug for DdwafResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self._result;
        f.debug_struct("DdwafResult")
            .field("timeout", &inner.timeout)
            .field("events", self.events())
            .field("actions", self.actions())
            .field("derivatives", self.derivatives())
            .field("total_runtime", &self.runtime())
            .finish()
    }
}

pub struct DdwafBuilder {
    _builder: bindings::ddwaf_builder,
    _config: DdwafConfig, // for holding memory only
}

// SAFETY: no thread-local data and no data can be changed under us if we have an owning handle
unsafe impl Send for DdwafBuilder {}
// SAFETY: changes are only made through exclusive references
unsafe impl Sync for DdwafBuilder {}
impl DdwafBuilder {
    pub fn new(config: Option<DdwafConfig>) -> Result<Self, DdwafGenericError> {
        let config = config.unwrap_or_default();
        let builder = DdwafBuilder {
            _builder: unsafe { bindings::ddwaf_builder_init(&config._cfg) },
            _config: config,
        };

        if builder._builder.is_null() {
            Err("Failed to initialize builder (ddwaf_builder_init returned null)".into())
        } else {
            Ok(builder)
        }
    }

    pub fn add_or_update_config(
        &mut self,
        path: &[u8],
        ruleset: &impl AsRef<bindings::ddwaf_object>,
        diagnostics: Option<&mut WafOwnedDdwafObj>,
    ) -> bool {
        unsafe {
            bindings::ddwaf_builder_add_or_update_config(
                self._builder,
                path.as_ptr() as _,
                path.len() as u32,
                // function takes non-const, but doesn't actually change config
                ruleset.as_ref() as *const _ as *mut _,
                diagnostics
                    .map(|d| &mut d._inner._obj as *mut bindings::ddwaf_object)
                    .unwrap_or(null_mut()),
            )
        }
    }

    pub fn build_instance(&mut self) -> Result<WafInstance, DdwafGenericError> {
        let raw_instance = unsafe { bindings::ddwaf_builder_build_instance(self._builder) };
        if raw_instance.is_null() {
            return Err(
                "Failed to build instance (ddwaf_builder_build_instance returned null)".into(),
            );
        }
        Ok(WafInstance {
            _handle: raw_instance,
            _config: None,
        })
    }

    pub fn remove_config(&mut self, path: &[u8]) -> bool {
        unsafe {
            bindings::ddwaf_builder_remove_config(
                self._builder,
                path.as_ptr() as _,
                path.len() as u32,
            )
        }
    }

    pub fn count_config_paths(&mut self, filter: &[u8]) -> u32 {
        unsafe {
            bindings::ddwaf_builder_get_config_paths(
                self._builder,
                null_mut(),
                filter.as_ptr() as _,
                filter.len() as u32,
            )
        }
    }

    pub fn get_config_paths(&mut self, filter: &[u8]) -> WafOwnedDdwafObj {
        let mut res = WafOwnedDdwafObj::default();
        unsafe {
            let _ = bindings::ddwaf_builder_get_config_paths(
                self._builder,
                res.as_mut(),
                filter.as_ptr() as _,
                filter.len() as u32,
            );
        }
        res
    }
}
impl Drop for DdwafBuilder {
    fn drop(&mut self) {
        unsafe { bindings::ddwaf_builder_destroy(self._builder) }
    }
}

// A WAF instance that can be shared (through clone())  and updated by any thread.
// More performant alternatives exist, with better reclamation strategies
pub struct UpdateableWafInstance {
    inner: Arc<UpdateableWafInstanceInner>,
}
struct UpdateableWafInstanceInner {
    builder: Mutex<DdwafBuilder>,
    waf_instance: ArcSwap<WafInstance>,
}
impl UpdateableWafInstance {
    pub const INITIAL_RULESET: &'static [u8] = b"<initial_ruleset>";

    pub fn new(
        ruleset: &impl AsRef<bindings::ddwaf_object>,
        config: Option<DdwafConfig>,
        diagnostics: Option<&mut WafOwnedDdwafObj>,
    ) -> Result<Self, DdwafGenericError> {
        let mut builder = DdwafBuilder::new(config)?;
        if !builder.add_or_update_config(Self::INITIAL_RULESET, ruleset, diagnostics) {
            return Err(
                "Failed to add initial ruleset (add_or_update_config returned false)".into(),
            );
        }
        let waf = builder.build_instance()?;
        Ok(Self {
            inner: Arc::new(UpdateableWafInstanceInner {
                builder: Mutex::new(builder),
                waf_instance: ArcSwap::from_pointee(waf),
            }),
        })
    }
}
impl Clone for UpdateableWafInstance {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(), // clone outer arc
        }
    }
}
impl UpdateableWafInstance {
    pub fn current(&self) -> Arc<WafInstance> {
        self.inner.waf_instance.load().clone()
    }

    pub fn add_or_update_config(
        &self,
        path: &[u8],
        ruleset: &impl AsRef<bindings::ddwaf_object>,
        diagnostics: Option<&mut WafOwnedDdwafObj>,
    ) -> bool {
        let mut guard = self.inner.builder.lock().unwrap();
        guard.add_or_update_config(path, ruleset, diagnostics)
    }

    pub fn remove_config(&self, path: &[u8]) -> bool {
        let mut guard = self.inner.builder.lock().unwrap();
        guard.remove_config(path)
    }

    pub fn count_config_paths(&self, filter: &[u8]) -> u32 {
        let mut guard = self.inner.builder.lock().unwrap();
        guard.count_config_paths(filter)
    }

    pub fn get_config_paths(&self, filter: &[u8]) -> WafOwnedDdwafObj {
        let mut guard = self.inner.builder.lock().unwrap();
        guard.get_config_paths(filter)
    }

    pub fn update(&self) -> Result<Arc<WafInstance>, DdwafGenericError> {
        let mut guard = self.inner.builder.lock().unwrap();
        let new_instance = Arc::new(guard.build_instance()?);
        let old = self.inner.waf_instance.swap(new_instance.clone());
        drop(old);
        Ok(new_instance)
    }
}

#[macro_export]
macro_rules! ddwaf_obj {
    (null) => {
        $crate::DdwafObj::from(())
    };
    ($l:expr) => {
        $crate::DdwafObj::from($l)
    };
}
#[macro_export]
macro_rules! ddwaf_obj_array {
    () => { DdwafObjArray::new(0) };
    ($($e:expr),* $(,)?) => {
        {
            let size : usize = [$($crate::__repl_expr_with_unit!($e)),*].len();
            let mut res = $crate::DdwafObjArray::new(size as u64);
            let mut i = usize::MAX;
            $(
                i = i.wrapping_add(1);
                res[i] = $crate::ddwaf_obj!($e);
            )*
            res
        }
    };
}
#[macro_export]
macro_rules! ddwaf_obj_map {
    () => { $crate::DdwafObjMap::new(0) };
    ($(($k:literal, $v:expr)),* $(,)?) => {
        {
            let size : usize = [$($crate::__repl_expr_with_unit!($v)),*].len();
            let mut res = $crate::DdwafObjMap::new(size as u64);
            let mut i = usize::MAX;
            $(
                i = i.wrapping_add(1);
                let k: &str = $k.into();
                let obj = $crate::Keyed::<DdwafObj>::from((k, $v));
                res[i] = obj.into();
            )*
            res
        }
    };
}
#[macro_export]
macro_rules! __repl_expr_with_unit {
    ($e:expr) => {
        ()
    };
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::*;

    #[cfg(not(miri))]
    #[test]
    fn test_get_version() {
        println!("libddwaf version: {:?}", get_version());
    }

    #[test]
    fn sample_mixed_object() {
        let mut root = DdwafObjArray::new(4);
        root[0] = 42_u64.into();
        root[1] = "Hello, world!".into();
        root[2] = DdwafObjArray::new(1).into();
        root[2].as_type_mut::<DdwafObjArray>().unwrap()[0] = 123_u64.into();

        let mut map = DdwafObjMap::new(7);
        map[0] = ("key 1", "value 1").into();
        map[1] = ("key 2", -2_i64).into();
        map[2] = ("key 3", 2_u64).into();
        map[3] = ("key 4", 5.2).into();
        map[4] = ("key 5", ()).into();
        map[5] = ("key 6", true).into();
        root[3] = map.into();

        let res = format!("{:?}", root);
        assert_eq!(
            res,
            "DdwafObjArray{DdwafObjUnsignedInt(42), DdwafObjString(\"Hello, \
            world!\"), DdwafObjArray{DdwafObjUnsignedInt(123), }, DdwafObjMap{\
            \"key 1\": DdwafObjString(\"value 1\"), \"key 2\": \
            DdwafObjSignedInt(-2), \"key 3\": DdwafObjUnsignedInt(2), \
            \"key 4\": DdwafObjFloat(5.2), \"key 5\": DdwafObjNull, \
            \"key 6\": DdwafObjBool(true), \"\": DdwafObj(Invalid), }, }"
        );
    }

    #[test]
    fn sample_mixed_object_macro() {
        let root = ddwaf_obj_array!(
            42_u64,
            "Hello, world!",
            123_u64,
            ddwaf_obj_map!(
                ("key 1", "value 1"),
                ("key 2", -2_i64),
                ("key 3", 2_u64),
                ("key 4", 5.2),
                ("key 5", ddwaf_obj!(null)),
                ("key 6", ddwaf_obj_array!()),
                ("key 7", ddwaf_obj_array!(true, false)),
            ),
            ddwaf_obj_array!(),
            ddwaf_obj_map!(),
        );

        let expected = r#"
- 42
- "Hello, world!"
- 123
- 
  key 1: "value 1"
  key 2: -2
  key 3: 2
  key 4: 5.2
  key 5: null
  key 6: []
  key 7: 
    - true
    - false
- []
- {}
"#;

        assert_eq!(root.debug_str(0), expected);
    }

    #[test]
    fn ddwaf_obj_from_conversions() {
        let obj: DdwafObj = 42u64.into();
        assert_eq!(obj.to_u64().unwrap(), 42u64);
        assert_eq!(obj.to_i64().unwrap(), 42i64);

        let obj: DdwafObj = (-42i64).into();
        assert_eq!(obj.to_i64().unwrap(), -42i64);

        let obj: DdwafObj = 3.0.into();
        assert_eq!(obj.to_f64().unwrap(), 3.0);

        let obj: DdwafObj = true.into();
        assert!(obj.to_bool().unwrap());

        let obj: DdwafObj = ().into();
        assert_eq!(obj.get_type(), DdwafObjType::Null);

        let obj: DdwafObj = "Hello, world!".into();
        assert_eq!(obj.to_str().unwrap(), "Hello, world!");

        let obj: DdwafObj = b"Hello, world!"[..].into();
        assert_eq!(obj.to_str().unwrap(), "Hello, world!");
    }

    #[test]
    fn ddwaf_obj_failed_conversions() {
        let mut obj: DdwafObj = ().into();
        assert!(obj.as_type::<DdwafObjBool>().is_none());
        assert!(obj.as_type_mut::<DdwafObjBool>().is_none());

        assert!(obj.to_bool().is_none());
        assert!(obj.to_u64().is_none());
        assert!(obj.to_i64().is_none());
        assert!(obj.to_f64().is_none());
        assert!(obj.to_str().is_none());
    }

    #[test]
    fn invalid_utf8() {
        let non_utf8_str: &[u8] = &[0x80];
        let obj: Keyed<DdwafObjString> = (non_utf8_str, non_utf8_str).into();
        assert_eq!(obj.debug_str(0), "\\x80: \"\\x80\"\n");
        assert_eq!(format!("{:?}", obj), r#""\x80": DdwafObjString("\x80")"#);

        assert!(obj.key_str().is_err());
        assert!(obj.as_str().is_err());
    }

    #[test]
    fn empty_key() {
        let map = ddwaf_obj_map!(("", 42_u64));
        let empty_slice: &[u8] = &[];
        assert_eq!(map[0].key(), empty_slice);
    }

    #[test]
    fn keyed_obj_methods() {
        let mut map = ddwaf_obj_map!(("key", 42_u64));
        let elem = &mut map[0];
        assert!(elem.as_keyed_type::<DdwafObjBool>().is_none());
        let elem_cast = elem.as_keyed_type::<DdwafObjUnsignedInt>().unwrap();
        assert_eq!(elem_cast.value(), 42u64);

        assert!(elem.as_keyed_type_mut::<DdwafObjBool>().is_none());
        let elem_cast = elem.as_keyed_type_mut::<DdwafObjUnsignedInt>().unwrap();
        elem_cast.set_key_str("key 2");
        assert_eq!(elem_cast.key_str().unwrap(), "key 2");
    }

    #[test]
    fn map_fetching_methods() {
        let mut map = ddwaf_obj_map!(("key1", 1u64), ("key2", 2u64),);

        // index
        assert_eq!(map[0].key(), b"key1");
        // index mut
        map[0].set_key(b"new key");
        assert_eq!(map[0].key(), b"new key");

        // get
        assert_eq!(map.get(b"key2").unwrap().to_u64().unwrap(), 2);
        assert!(map.get(b"bad key").is_none());
        // get_str
        assert_eq!(map.get_str("key2").unwrap().to_u64().unwrap(), 2);
        assert!(map.get_str("bad key").is_none());

        // get_mut
        map.get_mut(b"key2").unwrap().set_key_str("key3");
        let entry_k3 = map.get_str_mut("key3").unwrap();
        let new_entry: Keyed<DdwafObjUnsignedInt> = ("key3", 3u64).into();
        let _ = std::mem::replace(entry_k3, new_entry.into());
        assert_eq!(map.get_str("key3").unwrap().to_u64().unwrap(), 3);

        assert!(map.get_mut(b"bad key").is_none());

        // get_str_mut
        map.get_str_mut("key3").unwrap().set_key(b"key4");
        assert_eq!(map.get_str("key4").unwrap().to_u64().unwrap(), 3);

        assert!(map.get_str_mut("bad key").is_none());
    }

    #[test]
    fn array_iteration() {
        let mut arr = ddwaf_obj_array!(1u64, "foo", ddwaf_obj_array!("xyz"), ddwaf_obj!(null));

        for (i, elem) in arr.iter().enumerate() {
            match i {
                0 => assert_eq!(elem.to_u64().unwrap(), 1),
                1 => assert_eq!(elem.to_str().unwrap(), "foo"),
                2 => assert_eq!(elem.as_type::<DdwafObjArray>().unwrap().len(), 1),
                3 => assert_eq!(elem.get_type(), DdwafObjType::Null),
                _ => unreachable!(),
            }
        }

        for (i, elem) in arr.iter_mut().enumerate() {
            match i {
                0 => assert_eq!(elem.to_u64().unwrap(), 1),
                1 => {
                    assert_eq!(elem.to_str().unwrap(), "foo");
                    let new_str: DdwafObjString = "bar".into();
                    let _ = std::mem::replace(elem, new_str.into());
                }
                2 => assert_eq!(elem.as_type::<DdwafObjArray>().unwrap().len(), 1),
                3 => assert_eq!(elem.get_type(), DdwafObjType::Null),
                _ => unreachable!(),
            }
        }
        assert_eq!(arr[1].to_str().unwrap(), "bar");

        for (i, elem) in arr.into_iter().enumerate() {
            match i {
                0 => assert_eq!(elem.to_u64().unwrap(), 1),
                1 => assert_eq!(elem.to_str().unwrap(), "bar"),
                2 => assert_eq!(elem.as_type::<DdwafObjArray>().unwrap().len(), 1),
                3 => assert_eq!(elem.get_type(), DdwafObjType::Null),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn map_iteration() {
        let mut map = ddwaf_obj_map!(
            ("key1", 1u64),
            ("key2", "foo"),
            ("key3", ddwaf_obj_array!("xyz")),
            ("key4", ddwaf_obj!(null))
        );

        for (i, elem) in map.iter().enumerate() {
            match i {
                0 => {
                    assert_eq!(elem.key_str().unwrap(), "key1");
                    assert_eq!(elem.to_u64().unwrap(), 1);
                }
                1 => {
                    assert_eq!(elem.key_str().unwrap(), "key2");
                    assert_eq!(elem.to_str().unwrap(), "foo");
                }
                2 => {
                    assert_eq!(elem.key_str().unwrap(), "key3");
                    assert_eq!(elem.as_type::<DdwafObjArray>().unwrap().len(), 1);
                }
                3 => {
                    assert_eq!(elem.key_str().unwrap(), "key4");
                    assert_eq!(elem.get_type(), DdwafObjType::Null);
                }
                _ => unreachable!(),
            }
        }

        for (i, elem) in map.iter_mut().enumerate() {
            match i {
                0 => assert_eq!(elem.to_u64().unwrap(), 1),
                1 => {
                    assert_eq!(elem.key_str().unwrap(), "key2");
                    assert_eq!(elem.to_str().unwrap(), "foo");
                    let new_val: Keyed<DdwafObjString> = ("new_key", "bar").into();
                    let _ = std::mem::replace(elem, new_val.into());
                }
                2 => assert_eq!(elem.key_str().unwrap(), "key3"),
                3 => assert_eq!(elem.key_str().unwrap(), "key4"),
                _ => unreachable!(),
            }
        }

        assert_eq!(map[1].key_str().unwrap(), "new_key");
        assert_eq!(map[1].to_str().unwrap(), "bar");

        for (i, elem) in map.into_iter().enumerate() {
            match i {
                0 => assert_eq!(elem.key_str().unwrap(), "key1"),
                1 => assert_eq!(elem.key_str().unwrap(), "new_key"),
                2 => assert_eq!(elem.key_str().unwrap(), "key3"),
                3 => assert_eq!(elem.key_str().unwrap(), "key4"),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn partial_iteration() {
        let arr = ddwaf_obj_array!(1u64, "foo");
        for elem in arr.into_iter() {
            if elem.get_type() == DdwafObjType::Unsigned {
                break;
            }
        }

        let map = ddwaf_obj_map!(("key1", 1u64), ("key2", "foo"));
        for elem in map.into_iter() {
            if elem.get_type() == DdwafObjType::Unsigned {
                break;
            }
        }
    }

    #[test]
    fn iteration_of_empty_containers() {
        let mut arr: DdwafObjArray = ddwaf_obj_array!();
        assert!(arr.iter().next().is_none());
        assert!(arr.iter_mut().next().is_none());
        assert!(arr.into_iter().next().is_none());

        let mut map = ddwaf_obj_map!();
        assert!(map.iter().next().is_none());
        assert!(map.iter_mut().next().is_none());
        assert!(map.into_iter().next().is_none());
    }

    #[test]
    fn iteration_of_keyed_array() {
        let mut map = ddwaf_obj_map!(("key1", ddwaf_obj_array!(1u64, "foo")));
        let keyed_array: &mut Keyed<DdwafObjArray> = map[0].as_keyed_type_mut().unwrap();

        for (i, elem) in keyed_array.iter().enumerate() {
            match i {
                0 => assert_eq!(elem.to_u64().unwrap(), 1),
                1 => assert_eq!(elem.to_str().unwrap(), "foo"),
                _ => unreachable!(),
            }
        }

        for (i, elem) in keyed_array.iter_mut().enumerate() {
            match i {
                0 => assert_eq!(elem.to_u64().unwrap(), 1),
                1 => {
                    assert_eq!(elem.to_str().unwrap(), "foo");
                    let new_str: DdwafObjString = "bar".into();
                    let _ = std::mem::replace(elem, new_str.into());
                }
                _ => unreachable!(),
            }
        }

        assert_eq!(keyed_array[1].to_str().unwrap(), "bar");

        for (i, elem) in std::mem::take(keyed_array).into_iter().enumerate() {
            match i {
                0 => assert_eq!(elem.to_u64().unwrap(), 1),
                1 => assert_eq!(elem.to_str().unwrap(), "bar"),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn iteration_of_keyed_map() {
        let mut map = ddwaf_obj_map!(("key1", ddwaf_obj_map!(("key2", 1u64))));
        let keyed_map: &mut Keyed<DdwafObjMap> = map[0].as_keyed_type_mut().unwrap();

        for (i, elem) in keyed_map.iter().enumerate() {
            match i {
                0 => {
                    assert_eq!(elem.key_str().unwrap(), "key2");
                    assert_eq!(elem.to_u64().unwrap(), 1);
                }
                _ => unreachable!(),
            }
        }

        for (i, elem) in keyed_map.iter_mut().enumerate() {
            match i {
                0 => {
                    assert_eq!(elem.key_str().unwrap(), "key2");
                    assert_eq!(elem.to_u64().unwrap(), 1);
                    let new_val: Keyed<DdwafObjString> = ("new_key", "bar").into();
                    let _ = std::mem::replace(elem, new_val.into());
                }
                _ => unreachable!(),
            }
        }
        assert_eq!(keyed_map[0].key_str().unwrap(), "new_key");
        assert_eq!(keyed_map[0].to_str().unwrap(), "bar");

        for (i, elem) in std::mem::take(keyed_map).into_iter().enumerate() {
            match i {
                0 => {
                    assert_eq!(elem.key_str().unwrap(), "new_key");
                    assert_eq!(elem.to_str().unwrap(), "bar");
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn try_from_implementations() {
        let obj = ddwaf_obj!(42u64);
        assert!(DdwafObjArray::try_from(obj).is_err());
        let obj = ddwaf_obj!(42u64);
        assert!(DdwafObjUnsignedInt::try_from(obj).is_ok());

        let obj = ddwaf_obj!(42);
        assert!(DdwafObjUnsignedInt::try_from(obj).is_err());
        let obj = ddwaf_obj!(42);
        assert!(DdwafObjSignedInt::try_from(obj).is_ok());

        let obj = ddwaf_obj!(42.0);
        assert!(DdwafObjSignedInt::try_from(obj).is_err());
        let obj = ddwaf_obj!(42.0);
        assert!(DdwafObjFloat::try_from(obj).is_ok());

        let obj = ddwaf_obj!(true);
        assert!(DdwafObjFloat::try_from(obj).is_err());
        let obj = ddwaf_obj!(true);
        assert!(DdwafObjBool::try_from(obj).is_ok());

        let obj = ddwaf_obj!(null);
        assert!(DdwafObjBool::try_from(obj).is_err());
        let obj = ddwaf_obj!(null);
        assert!(DdwafObjNull::try_from(obj).is_ok());

        let obj = ddwaf_obj!("foobar");
        assert!(DdwafObjNull::try_from(obj).is_err());
        let obj = ddwaf_obj!("foobar");
        assert!(DdwafObjString::try_from(obj).is_ok());

        let obj: DdwafObj = ddwaf_obj_map!().into();
        assert!(DdwafObjString::try_from(obj).is_err());
        let obj: DdwafObj = ddwaf_obj_map!().into();
        assert!(DdwafObjMap::try_from(obj).is_ok());

        let obj: DdwafObj = ddwaf_obj_array!().into();
        assert!(DdwafObjMap::try_from(obj).is_err());
        let obj: DdwafObj = ddwaf_obj_array!().into();
        assert!(DdwafObjArray::try_from(obj).is_ok());
    }

    #[test]
    fn unsafe_changes_to_default_objects() {
        unsafe {
            let mut unsigned = DdwafObjUnsignedInt::default();
            unsigned.as_mut().__bindgen_anon_1.uintValue += 1;
            assert_eq!(unsigned.value(), 1);

            let mut signed = DdwafObjSignedInt::default();
            signed.as_mut().__bindgen_anon_1.intValue -= 1;
            assert_eq!(signed.value(), -1);

            let mut float = DdwafObjFloat::default();
            float.as_mut().__bindgen_anon_1.f64_ += 1.0;
            assert_eq!(float.value(), 1.0);

            let mut boolean = DdwafObjBool::default();
            boolean.as_mut().__bindgen_anon_1.boolean = true;
            assert!(boolean.value());

            let mut null = DdwafObjNull::default();
            // nothing interesting to do for null; let's try manually setting
            // the parameter name
            let s = String::from_str("foobar").unwrap();
            let b: Box<[u8]> = s.as_bytes().into();
            let p = Box::<[u8]>::into_raw(b);
            let null_mut = null.as_mut();
            null_mut.parameterName = p as *mut _;
            null_mut.parameterNameLength = s.len() as u64;
            drop(std::mem::take(null_mut.as_keyed_ddwaf_obj_ref_mut()));

            let mut string = DdwafObjString::default();
            let str_mut = string.as_mut();
            let b: Box<[u8]> = s.as_bytes().into();
            let p = Box::<[u8]>::into_raw(b);
            drop_ddwaf_object_string(str_mut);
            str_mut.__bindgen_anon_1.stringValue = p as *const _;
            str_mut.nbEntries = s.len() as u64;
            assert_eq!(string.as_str().unwrap(), "foobar");
            assert_eq!(string.len(), s.len());
            assert!(!string.is_empty());
        }
    }
}
