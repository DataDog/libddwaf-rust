use core::slice;
use std::{
    alloc::Layout,
    ffi::{CStr, CString},
    fmt::Write,
    mem::MaybeUninit,
    ops::{Deref, Fn, Index, IndexMut},
    ptr::{addr_of_mut, null, null_mut, NonNull},
    sync::{Arc, RwLock},
};

#[allow(unused)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[allow(unused)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

#[cfg(feature = "serde")]
pub mod serde;

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

pub trait TypedDdwafObj {
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

// implement Send + Sync on ddwaf_object to avoid haing to implement it on each struct
unsafe impl Send for bindings::ddwaf_object {}
unsafe impl Sync for bindings::ddwaf_object {}

impl AsRef<bindings::ddwaf_object> for DdwafObj {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        &self._obj
    }
}
impl AsMut<bindings::ddwaf_object> for DdwafObj {
    fn as_mut(&mut self) -> &mut bindings::ddwaf_object {
        &mut self._obj
    }
}
impl AsRef<bindings::ddwaf_object> for DdwafObjUnsignedInt {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        &self._obj
    }
}
impl AsMut<bindings::ddwaf_object> for DdwafObjUnsignedInt {
    fn as_mut(&mut self) -> &mut bindings::ddwaf_object {
        &mut self._obj
    }
}
impl AsRef<bindings::ddwaf_object> for DdwafObjSignedInt {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        &self._obj
    }
}
impl AsMut<bindings::ddwaf_object> for DdwafObjSignedInt {
    fn as_mut(&mut self) -> &mut bindings::ddwaf_object {
        &mut self._obj
    }
}
impl AsRef<bindings::ddwaf_object> for DdwafObjFloat {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        &self._obj
    }
}
impl AsMut<bindings::ddwaf_object> for DdwafObjFloat {
    fn as_mut(&mut self) -> &mut bindings::ddwaf_object {
        &mut self._obj
    }
}
impl AsRef<bindings::ddwaf_object> for DdwafObjBool {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        &self._obj
    }
}
impl AsMut<bindings::ddwaf_object> for DdwafObjBool {
    fn as_mut(&mut self) -> &mut bindings::ddwaf_object {
        &mut self._obj
    }
}
impl AsRef<bindings::ddwaf_object> for DdwafObjNull {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        &self._obj
    }
}
impl AsMut<bindings::ddwaf_object> for DdwafObjNull {
    fn as_mut(&mut self) -> &mut bindings::ddwaf_object {
        &mut self._obj
    }
}
impl AsRef<bindings::ddwaf_object> for DdwafObjString {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        &self._obj
    }
}
impl AsMut<bindings::ddwaf_object> for DdwafObjString {
    fn as_mut(&mut self) -> &mut bindings::ddwaf_object {
        &mut self._obj
    }
}
impl AsRef<bindings::ddwaf_object> for DdwafObjArray {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        &self._obj
    }
}
impl AsMut<bindings::ddwaf_object> for DdwafObjArray {
    fn as_mut(&mut self) -> &mut bindings::ddwaf_object {
        &mut self._obj
    }
}
impl AsRef<bindings::ddwaf_object> for DdwafObjMap {
    fn as_ref(&self) -> &bindings::ddwaf_object {
        &self._obj
    }
}
impl AsMut<bindings::ddwaf_object> for DdwafObjMap {
    fn as_mut(&mut self) -> &mut bindings::ddwaf_object {
        &mut self._obj
    }
}

// generic
impl bindings::ddwaf_object {
    // these two are actually unsafe (they don't check the type)
    fn unchecked_as_ref<T>(&self) -> &T {
        unsafe { &*(self as *const bindings::ddwaf_object as *const T) }
    }
    fn unchecked_as_ref_mut<T>(&mut self) -> &mut T {
        unsafe { &mut *(self as *mut bindings::ddwaf_object as *mut T) }
    }

    // are these safe or (esp. the mut one) can the violate aliasing rules?
    fn as_ddwaf_obj_ref(&self) -> &DdwafObj {
        self.unchecked_as_ref::<DdwafObj>()
    }
    fn as_ddwaf_obj_ref_mut(&mut self) -> &mut DdwafObj {
        self.unchecked_as_ref_mut::<DdwafObj>()
    }
}

pub trait CommonDdwafObj {
    fn get_type(&self) -> DdwafObjType;
    fn key(&self) -> &[u8];
    fn debug_str(&self, indent: i32) -> String;
}

pub trait CommonDdwafObjMut {
    fn set_key_str(&mut self, key: &str) -> &mut Self;
}

impl<T> CommonDdwafObj for T
where
    T: AsRef<bindings::ddwaf_object>,
{
    fn get_type(&self) -> DdwafObjType {
        (self.as_ref().type_).try_into().unwrap()
    }

    fn key(&self) -> &[u8] {
        let obj = self.as_ref();
        let data = if obj.parameterName.is_null() {
            debug_assert!(obj.parameterNameLength == 0);
            NonNull::dangling().as_ptr()
        } else {
            obj.parameterName as *const u8
        };
        unsafe { std::slice::from_raw_parts(data, obj.parameterNameLength as usize) }
    }

    fn debug_str(&self, indent: i32) -> String {
        let mut s: String = String::default();
        let key = self.key();
        if !key.is_empty() {
            s += std::str::from_utf8(key).unwrap();
            s += ": ";
        }
        match self.get_type() {
            DdwafObjType::String => {
                s += "\"";
                s += self
                    .as_ref()
                    .unchecked_as_ref::<DdwafObjString>()
                    .as_str()
                    .unwrap();
                s += "\"\n";
            }
            DdwafObjType::Unsigned => {
                let obj = self.as_ref().unchecked_as_ref::<DdwafObjUnsignedInt>();
                s += &format!("{}", obj.value());
                s += "\n";
            }
            DdwafObjType::Signed => {
                let obj = self.as_ref().unchecked_as_ref::<DdwafObjSignedInt>();
                s += &format!("{}", obj.value());
                s += "\n";
            }
            DdwafObjType::Float => {
                let obj = self.as_ref().unchecked_as_ref::<DdwafObjFloat>();
                s += &format!("{}", obj.value());
                s += "\n";
            }
            DdwafObjType::Bool => {
                let obj = self.as_ref().unchecked_as_ref::<DdwafObjBool>();
                s += if obj.value() { "true\n" } else { "false\n" };
            }
            DdwafObjType::Null => {
                s += "null\n";
            }
            DdwafObjType::Array => {
                let obj = self.as_ref().unchecked_as_ref::<DdwafObjArray>();
                if obj.len() == 0 {
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
                let obj = self.as_ref().unchecked_as_ref::<DdwafObjMap>();
                if obj.len() == 0 {
                    s += "{}\n";
                } else {
                    s += "\n";
                    for i in 0..obj.len() {
                        s.extend(std::iter::repeat(" ").take(indent as usize));
                        s += &obj[i].debug_str(indent + 2);
                    }
                }
            }
            _ => {
                s += "TODO";
            }
        }
        s
    }
}

impl<T> CommonDdwafObjMut for T
where
    T: AsMut<bindings::ddwaf_object>,
{
    fn set_key_str(&mut self, key: &str) -> &mut Self {
        let obj = self.as_mut();
        drop_key(obj);
        let key_len = key.len();
        let layout = Layout::array::<std::os::raw::c_char>(key_len).unwrap();
        let mem = unsafe { no_fail_alloc(layout) };
        if !mem.is_null() {
            // 0 size
            unsafe {
                std::ptr::copy_nonoverlapping(key.as_ptr(), mem as *mut u8, key_len);
            }
        }
        obj.parameterName = mem as *const std::os::raw::c_char;
        obj.parameterNameLength = key_len as u64;
        self
    }
}
fn drop_key(obj: &mut bindings::ddwaf_object) {
    if !obj.parameterName.is_null() {
        let layout =
            Layout::array::<std::os::raw::c_char>(obj.parameterNameLength as usize).unwrap();
        unsafe { std::alloc::dealloc(obj.parameterName as *mut u8, layout) };
    }
}

// ddwaf_obj
impl DdwafObj {
    pub fn as_type<T: TypedDdwafObj>(&self) -> Option<&T> {
        if self.get_type() == T::TYPE {
            Some(self.as_ref().unchecked_as_ref::<T>())
        } else {
            None
        }
    }
    pub fn as_type_mut<T: TypedDdwafObj>(&mut self) -> Option<&mut T> {
        if self.get_type() == T::TYPE {
            Some(self.as_mut().unchecked_as_ref_mut::<T>())
        } else {
            None
        }
    }
    pub fn to_u64(&self) -> Option<u64> {
        self.as_type::<DdwafObjUnsignedInt>().map(|x| x.value())
    }

    pub fn to_str(&self) -> Option<&str> {
        self.as_type::<DdwafObjString>()
            .and_then(|x| x.as_str().ok())
    }

    pub fn to_i64(&self) -> Option<i64> {
        match self.get_type() {
            DdwafObjType::Unsigned => {
                let obj = self.as_ref().unchecked_as_ref::<DdwafObjUnsignedInt>();
                obj.value().try_into().ok()
            }
            DdwafObjType::Signed => {
                let obj = self.as_ref().unchecked_as_ref::<DdwafObjSignedInt>();
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
                let obj = self.as_ref().unchecked_as_ref::<DdwafObjUnsignedInt>();
                obj.fmt(f)
            }
            DdwafObjType::Signed => {
                let obj = self.as_ref().unchecked_as_ref::<DdwafObjSignedInt>();
                obj.fmt(f)
            }
            DdwafObjType::Float => {
                let obj = self.as_ref().unchecked_as_ref::<DdwafObjFloat>();
                obj.fmt(f)
            }
            DdwafObjType::Bool => {
                let obj = self.as_ref().unchecked_as_ref::<DdwafObjBool>();
                obj.fmt(f)
            }
            DdwafObjType::Null => {
                let obj = self.as_ref().unchecked_as_ref::<DdwafObjNull>();
                obj.fmt(f)
            }
            DdwafObjType::String => {
                let obj = self.as_ref().unchecked_as_ref::<DdwafObjString>();
                obj.fmt(f)
            }
            DdwafObjType::Array => {
                let obj = self.as_ref().unchecked_as_ref::<DdwafObjArray>();
                obj.fmt(f)
            }
            DdwafObjType::Map => {
                let obj = self.as_ref().unchecked_as_ref::<DdwafObjMap>();
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
// TODO: use crate for numer traits
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
impl From<()> for DdwafObj {
    fn from(_: ()) -> Self {
        let ret: DdwafObjNull = ().into();
        ret.into()
    }
}
impl<T> From<(&str, T)> for DdwafObj
where
    T: Into<DdwafObj>,
{
    fn from(value: (&str, T)) -> Self {
        let mut ret = value.1.into();
        ret.set_key_str(value.0);
        ret
    }
}
impl<T> From<T> for DdwafObj
where
    T: AsRef<bindings::ddwaf_object> + TypedDdwafObj,
{
    fn from(value: T) -> Self {
        let res = Self {
            _obj: value.as_ref().clone(),
        };
        std::mem::forget(value);
        res
    }
}

fn drop_ddwaf_object(obj: &mut bindings::ddwaf_object) {
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
        drop_key(&mut self._obj);
        drop_ddwaf_object(&mut self._obj)
    }
}

// unsigned int
impl DdwafObjUnsignedInt {
    pub fn value(&self) -> u64 {
        unsafe { self._obj.__bindgen_anon_1.uintValue }
    }
}
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
        Self {
            _obj: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_UNSIGNED,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 { uintValue: value },
                parameterName: null_mut(),
                parameterNameLength: 0,
                nbEntries: 0,
            },
        }
    }
}
impl Drop for DdwafObjUnsignedInt {
    fn drop(&mut self) {
        drop_key(&mut self._obj);
    }
}

// signed int
impl DdwafObjSignedInt {
    pub fn value(&self) -> i64 {
        unsafe { self._obj.__bindgen_anon_1.intValue }
    }
}
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
        Self {
            _obj: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_SIGNED,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 { intValue: value },
                parameterName: null_mut(),
                parameterNameLength: 0,
                nbEntries: 0,
            },
        }
    }
}
impl TryFrom<DdwafObj> for DdwafObjSignedInt {
    type Error = DdwafObjTypeError;
    fn try_from(value: DdwafObj) -> Result<Self, Self::Error> {
        if value.get_type() != DdwafObjType::Signed {
            return Err(Self::Error {
                message: "Invalid DDWAFObjType value (not an unsigned int)",
            });
        }
        let res = Ok(Self { _obj: value._obj });
        std::mem::forget(value);
        res
    }
}
impl Drop for DdwafObjSignedInt {
    fn drop(&mut self) {
        drop_key(&mut self._obj);
    }
}

// float
impl DdwafObjFloat {
    pub fn value(&self) -> f64 {
        unsafe { self._obj.__bindgen_anon_1.f64_ }
    }
}
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
        Self {
            _obj: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_FLOAT,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 { f64_: value },
                parameterName: null_mut(),
                parameterNameLength: 0,
                nbEntries: 0,
            },
        }
    }
}
impl TryFrom<DdwafObj> for DdwafObjFloat {
    type Error = DdwafObjTypeError;
    fn try_from(value: DdwafObj) -> Result<Self, Self::Error> {
        if value.get_type() != DdwafObjType::Signed {
            return Err(Self::Error {
                message: "Invalid DDWAFObjType value (not a floating point number)",
            });
        }
        let res = Ok(Self { _obj: value._obj });
        std::mem::forget(value);
        res
    }
}
impl Drop for DdwafObjFloat {
    fn drop(&mut self) {
        drop_key(&mut self._obj);
    }
}

// bool
impl DdwafObjBool {
    pub fn value(&self) -> bool {
        unsafe { self._obj.__bindgen_anon_1.boolean }
    }
}
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
        Self {
            _obj: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_BOOL,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 { boolean: value },
                parameterName: null_mut(),
                parameterNameLength: 0,
                nbEntries: 0,
            },
        }
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
impl Drop for DdwafObjBool {
    fn drop(&mut self) {
        drop_key(&mut self._obj);
    }
}

// null
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
                parameterName: null_mut(),
                parameterNameLength: 0,
                nbEntries: 0,
            },
        }
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
impl Drop for DdwafObjNull {
    fn drop(&mut self) {
        drop_key(&mut self._obj);
    }
}

// string
impl DdwafObjString {
    pub fn new(value: &[u8]) -> Self {
        let layout = Layout::array::<std::os::raw::c_char>(value.len()).unwrap();
        let mem = unsafe { no_fail_alloc(layout) };
        if !mem.is_null() {
            unsafe {
                std::ptr::copy_nonoverlapping(value.as_ptr(), mem as *mut u8, value.len());
            }
        }
        Self {
            _obj: bindings::ddwaf_object {
                type_: bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_STRING,
                __bindgen_anon_1: bindings::_ddwaf_object__bindgen_ty_1 {
                    stringValue: mem as *const std::os::raw::c_char,
                },
                parameterName: null_mut(),
                parameterNameLength: 0,
                nbEntries: value.len() as u64,
            },
        }
    }

    pub fn len(&self) -> usize {
        self._obj.nbEntries as usize
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
impl TypedDdwafObj for DdwafObjString {
    const TYPE: DdwafObjType = DdwafObjType::String;
}
impl std::fmt::Debug for DdwafObjString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "DdwafObjString({:?})",
            self.as_str().unwrap_or("<invalid utf8>")
        ))
    }
}
impl From<&str> for DdwafObjString {
    fn from(value: &str) -> Self {
        Self::new(value.as_bytes())
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
fn drop_ddwaf_object_string(obj: &mut bindings::ddwaf_object) {
    debug_assert!(obj.type_ == bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_STRING);
    let sval = unsafe { obj.__bindgen_anon_1.stringValue };
    let sval_len = obj.nbEntries;
    if !sval.is_null() {
        let layout = Layout::array::<std::os::raw::c_char>(sval_len as usize).unwrap();
        unsafe { std::alloc::dealloc(sval as *mut u8, layout) };
    }
}
impl Drop for DdwafObjString {
    fn drop(&mut self) {
        drop_key(&mut self._obj);
        drop_ddwaf_object_string(&mut self._obj)
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
                parameterName: null_mut(),
                parameterNameLength: 0,
                nbEntries: size,
            },
        }
    }

    pub fn len(&self) -> usize {
        self.as_ref().nbEntries as usize
    }
}
impl TypedDdwafObj for DdwafObjArray {
    const TYPE: DdwafObjType = DdwafObjType::Array;
}
impl Index<usize> for DdwafObjArray {
    type Output = DdwafObj;
    fn index(&self, index: usize) -> &Self::Output {
        let obj = self.as_ref();
        let array = unsafe { obj.__bindgen_anon_1.array };
        let array_len = obj.nbEntries;
        if index >= array_len as usize {
            panic!("Index out of bounds");
        }
        let elem = unsafe { array.offset(index as isize) };
        unsafe { &*(elem as *const DdwafObj) }
    }
}
impl IndexMut<usize> for DdwafObjArray {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let obj = self.as_ref();
        let array = unsafe { obj.__bindgen_anon_1.array };
        let array_len = obj.nbEntries;
        if index >= array_len as usize {
            panic!("Index out of bounds");
        }
        let elem = unsafe { array.offset(index as isize) };
        unsafe { &mut *(elem as *mut DdwafObj) }
    }
}
pub struct DdwafArrayIter<'a> {
    array: &'a DdwafObjArray,
    index: usize,
}
impl<'a> std::iter::Iterator for DdwafArrayIter<'a> {
    type Item = &'a DdwafObj;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.array.len() {
            return None;
        }

        let res = &self.array[self.index];
        self.index += 1;
        Some(res)
    }
}
impl<'a> IntoIterator for &'a DdwafObjArray {
    type Item = &'a DdwafObj;
    type IntoIter = DdwafArrayIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        DdwafArrayIter {
            array: self,
            index: 0,
        }
    }
}
impl std::fmt::Debug for DdwafObjArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DdwafObjArray{")?;
        for elem in self.into_iter() {
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
fn drop_ddwaf_object_array(obj: &mut bindings::ddwaf_object) {
    debug_assert!(obj.type_ == bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_ARRAY);
    let array = unsafe { obj.__bindgen_anon_1.array };
    let array_len = obj.nbEntries;
    for i in 0..array_len {
        let elem = unsafe { array.offset(i as isize) };
        drop_ddwaf_object(unsafe { &mut *elem });
    }
    let layout = Layout::array::<bindings::ddwaf_object>(array_len as usize).unwrap();
    unsafe { std::alloc::dealloc(array as *mut u8, layout) };
}
impl Drop for DdwafObjArray {
    fn drop(&mut self) {
        drop_key(&mut self._obj);
        drop_ddwaf_object_array(&mut self._obj)
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
                parameterName: null_mut(),
                parameterNameLength: 0,
                nbEntries: size,
            },
        }
    }

    pub fn len(&self) -> usize {
        self.as_ref().nbEntries as usize
    }

    pub fn get(&self, key: &[u8]) -> Option<&DdwafObj> {
        let array = unsafe { self._obj.__bindgen_anon_1.array };
        let array_len = self.len();
        for i in 0..array_len {
            let elem_ptr = unsafe { array.offset(i as isize) };
            let elem = unsafe { &*elem_ptr }.as_ddwaf_obj_ref();
            if elem.key() == key {
                return Some(elem);
            }
        }
        None
    }

    pub fn gets(&self, key: &str) -> Option<&DdwafObj> {
        self.get(key.as_bytes())
    }

    pub fn get_mut(&mut self, key: &[u8]) -> Option<&mut DdwafObj> {
        let array = unsafe { self._obj.__bindgen_anon_1.array };
        let array_len = self.len();
        for i in 0..array_len {
            let elem_ptr = unsafe { array.offset(i as isize) };
            let elem = unsafe { &mut *elem_ptr }.as_ddwaf_obj_ref_mut();
            if elem.key() == key {
                return Some(elem);
            }
        }
        None
    }

    pub fn get_str_mut(&mut self, key: &str) -> Option<&mut DdwafObj> {
        self.get_mut(key.as_bytes())
    }
}
impl TypedDdwafObj for DdwafObjMap {
    const TYPE: DdwafObjType = DdwafObjType::Map;
}
impl Index<usize> for DdwafObjMap {
    type Output = DdwafObj;
    fn index(&self, index: usize) -> &Self::Output {
        let obj = self.as_ref();
        let array = unsafe { obj.__bindgen_anon_1.array };
        let array_len = obj.nbEntries;
        if index >= array_len as usize {
            panic!("Index out of bounds");
        }
        let elem = unsafe { array.offset(index as isize) };
        unsafe { &*(elem as *const DdwafObj) }
    }
}
impl IndexMut<usize> for DdwafObjMap {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let obj = self.as_ref();
        let array = unsafe { obj.__bindgen_anon_1.array };
        let array_len = obj.nbEntries;
        if index >= array_len as usize {
            panic!("Index out of bounds");
        }
        let elem = unsafe { array.offset(index as isize) };
        unsafe { &mut *(elem as *mut DdwafObj) }
    }
}
pub struct DdwafMapIter<'a> {
    array: &'a DdwafObjMap,
    index: usize,
}
impl<'a> std::iter::Iterator for DdwafMapIter<'a> {
    type Item = (&'a [u8], &'a DdwafObj);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.array.len() {
            return None;
        }

        let res = &self.array[self.index];
        self.index += 1;
        Some((res.key(), res))
    }
}
impl<'a> IntoIterator for &'a DdwafObjMap {
    type Item = (&'a [u8], &'a DdwafObj);
    type IntoIter = DdwafMapIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        DdwafMapIter {
            array: self,
            index: 0,
        }
    }
}
impl std::fmt::Debug for DdwafObjMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DdwafObjMap{")?;
        for (key, elem) in self.into_iter() {
            if let Ok(key_str) = std::str::from_utf8(key) {
                f.write_fmt(format_args!("\"{}\": {:?}, ", key_str, elem))?;
            } else {
                f.write_fmt(format_args!("{:?}: {:?}, ", key, elem))?;
            }
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
fn drop_ddwaf_object_map(obj: &mut bindings::ddwaf_object) {
    debug_assert!(obj.type_ == bindings::DDWAF_OBJ_TYPE_DDWAF_OBJ_MAP);
    let array = unsafe { obj.__bindgen_anon_1.array };
    let array_len = obj.nbEntries;
    for i in 0..array_len {
        let elem = unsafe { &mut *array.offset(i as isize) };
        drop_key(elem);
        drop_ddwaf_object(elem);
    }
    let layout = Layout::array::<bindings::ddwaf_object>(array_len as usize).unwrap();
    unsafe { std::alloc::dealloc(array as *mut u8, layout) };
}
impl Drop for DdwafObjMap {
    fn drop(&mut self) {
        drop_key(&mut self._obj);
        drop_ddwaf_object_map(&mut self._obj)
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

pub fn get_version() -> &'static CStr {
    unsafe { CStr::from_ptr(bindings::ddwaf_get_version()) }
}

pub struct Config {
    _cfg: bindings::ddwaf_config,
    _obfuscator: Obfuscator,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            _cfg: bindings::ddwaf_config {
                limits: Limits::default(),
                obfuscator: Obfuscator::default()._raw_obfuscator,
                free_fn: None,
            },
            _obfuscator: Obfuscator::default(),
        }
    }
}
impl Config {
    pub fn new(limits: Limits, obfuscator: Obfuscator) -> Self {
        Self {
            _cfg: bindings::ddwaf_config {
                limits,
                obfuscator: obfuscator._raw_obfuscator,
                free_fn: None,
            },
            _obfuscator: obfuscator,
        }
    }

    fn clone(&self) -> Config {
        Self {
            _cfg: bindings::ddwaf_config {
                limits: self._cfg.limits,
                obfuscator: self._obfuscator._raw_obfuscator,
                free_fn: None,
            },
            _obfuscator: self._obfuscator.clone(),
        }
    }
}

pub type Limits = bindings::_ddwaf_config__ddwaf_config_limits;
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
        Self {
            _raw_obfuscator: bindings::_ddwaf_config__ddwaf_config_obfuscator {
                key_regex: null(),
                value_regex: null(),
            },
        }
    }
}
impl Obfuscator {
    pub fn new(key_regex: &str, value_regex: &str) -> Self {
        let key_regex = CString::new(key_regex).expect("Invalid key regex");
        let value_regex = CString::new(value_regex).expect("Invalid value regex");
        Self {
            _raw_obfuscator: bindings::_ddwaf_config__ddwaf_config_obfuscator {
                key_regex: key_regex.into_raw(),
                value_regex: value_regex.into_raw(),
            },
        }
    }
}
impl Clone for Obfuscator {
    fn clone(&self) -> Self {
        let key_regex = unsafe { CStr::from_ptr(self._raw_obfuscator.key_regex) };
        let value_regex = unsafe { CStr::from_ptr(self._raw_obfuscator.value_regex) };
        Self::new(key_regex.to_str().unwrap(), value_regex.to_str().unwrap())
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
    _config: Config,
}
impl Drop for WafInstance {
    fn drop(&mut self) {
        unsafe { bindings::ddwaf_destroy(self._handle) }
    }
}

// Safety: ddwaf instances are effectively immutable once created and until
// their destruction. This is despite ddwaf_update not taking a pointer to
// const data.
unsafe impl Send for WafInstance {}
unsafe impl Sync for WafInstance {}

impl WafInstance {
    pub fn new<T: AsRef<bindings::ddwaf_object>>(
        ruleset: &T,
        config: Config,
        diagnostics: Option<&mut WafOwnedDdwafObj>,
    ) -> Result<Self, &'static str> {
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
            return Err("Failed to initialize handle");
        }
        Ok(Self {
            _handle: handle,
            _config: config,
        })
    }

    pub fn update(
        &self,
        ruleset: &DdwafObj,
        diagnostics: Option<&mut WafOwnedDdwafObj>,
    ) -> Result<Self, &'static str> {
        let handle = unsafe {
            bindings::ddwaf_update(
                self._handle,
                ruleset.as_ref(),
                diagnostics
                    .map(|d| &mut d._inner._obj as *mut bindings::ddwaf_object)
                    .unwrap_or(null_mut()),
            )
        };

        if handle.is_null() {
            return Err("Failed to update handle");
        }
        Ok(Self {
            _handle: handle,
            _config: self._config.clone(),
        })
    }

    pub fn create_context(&self) -> WafContext {
        WafContext {
            _ctx: unsafe { bindings::ddwaf_context_init(self._handle) },
            _owned_objs: Vec::new(),
        }
    }

    pub fn known_actions<'a>(&mut self) -> Option<Vec<&'a CStr>> {
        // function is not thread-safe, so we need an exclusive reference
        let mut size = std::mem::MaybeUninit::<u32>::uninit();
        let actions_raw = unsafe { bindings::ddwaf_known_actions(self._handle, size.as_mut_ptr()) };
        if actions_raw.is_null() {
            return None;
        }

        let size = unsafe { size.assume_init() as usize };
        let actions = unsafe { std::slice::from_raw_parts(actions_raw, size) };
        // the char pointers cannot be directly converted to &CStr because of &CStr's fatness
        Some(
            actions
                .iter()
                .map(|&x| unsafe { CStr::from_ptr(x) })
                .collect(),
        )
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

// Safety: writes to a static variable without synchronization. Meant to be used
// on startup only
pub unsafe fn set_log_cb<
    F: Fn(DdwafLogLevel, &'static CStr, &'static CStr, u32, &[std::os::raw::c_char]) -> () + 'static,
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

static mut LOG_CB: Option<
    Box<dyn Fn(DdwafLogLevel, &'static CStr, &'static CStr, u32, &[std::os::raw::c_char]) -> ()>,
> = None;

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
        self._result.events.unchecked_as_ref()
    }

    pub fn actions(&self) -> &DdwafObjMap {
        self._result.actions.unchecked_as_ref()
    }

    pub fn derivatives(&self) -> &DdwafObjMap {
        self._result.derivatives.unchecked_as_ref()
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

// A WAF instance that can be shared (through clone())  and updated by any thread.
// More performant alternatives exist, with better reclamation strategies
pub struct UpdateableWafInstance {
    inner: Arc<RwLock<Arc<WafInstance>>>,
}
impl UpdateableWafInstance {
    pub fn new(waf: WafInstance) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Arc::new(waf))),
        }
    }
}
impl Clone for UpdateableWafInstance {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(), // clone outer arc
        }
    }
}

#[macro_export]
macro_rules! ddwaf_obj {
    (null) => {
        DdwafObj::from(())
    };
    ($l:expr) => {
        DdwafObj::from($l)
    };
}
#[macro_export]
macro_rules! ddwaf_obj_array {
    () => { DdwafObj::from(DdwafObjArray::new(0)) };
    ($($e:expr),* $(,)?) => {
        {
            let size : usize = [$($crate::__repl_expr_with_unit!($e)),*].len();
            let mut res = DdwafObjArray::new(size as u64);
            let mut i = usize::MAX;
            $(
                i = i.wrapping_add(1);
                res[i] = $crate::ddwaf_obj!($e);
            )*
            DdwafObj::from(res)
        }
    };
}
#[macro_export]
macro_rules! ddwaf_obj_map {
    () => { DdwafObj::from(DdwafObjMap::new(0)) };
    ($(($k:literal, $v:expr)),* $(,)?) => {
        {
            let size : usize = [$($crate::__repl_expr_with_unit!($v)),*].len();
            let mut res = DdwafObjMap::new(size as u64);
            let mut i = usize::MAX;
            $(
                i = i.wrapping_add(1);
                let k: &str = $k.into();
                let mut obj : DdwafObj = DdwafObj::from($v);
                obj.set_key_str(k);
                res[i] = obj.into();
            )*
            DdwafObj::from(res)
        }
    };
}
#[macro_export]
macro_rules! __repl_expr_with_unit {
    ($e:expr) => {
        ()
    };
}

impl UpdateableWafInstance {
    pub fn current(&self) -> Arc<WafInstance> {
        self.inner.read().unwrap().clone() // clone inner arc holding read lock
    }

    pub fn update(
        &self,
        ruleset: &DdwafObj,
        diagnostics: Option<&mut WafOwnedDdwafObj>,
    ) -> Result<Arc<WafInstance>, &'static str> {
        let mut waf = self.inner.write().unwrap();
        let new_waf = waf.update(ruleset, diagnostics)?;
        *waf = Arc::new(new_waf);
        Ok(waf.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

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

        let mut map = DdwafObjMap::new(5);
        map[0] = ("key 1", "value 1").into();
        map[1] = ("key 2", -2_i64).into();
        map[2] = ("key 3", 2_u64).into();
        map[3] = ("key 4", 5.2).into();
        map[4] = ("key 5", ()).into();
        root[3] = map.into();

        println!("{}", root.debug_str(0));

        for elem in &root {
            println!("elem: {}", elem.debug_str(0));
        }
        for (key, elem) in root[3].as_type::<DdwafObjMap>().unwrap().into_iter() {
            println!(
                "key: {:?}, elem: {}",
                std::str::from_utf8(key).unwrap(),
                elem.debug_str(0)
            );
        }
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
            ),
            ddwaf_obj_array!(),
            ddwaf_obj_map!(),
        );
        println!("{}", root.debug_str(0));
    }
}
