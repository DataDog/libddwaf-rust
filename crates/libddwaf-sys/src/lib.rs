#![crate_type = "dylib"]
#![deny(clippy::correctness, clippy::perf, clippy::style, clippy::suspicious)]
#![allow(unused)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use std::alloc::Layout;
use std::ptr::null;
use std::slice;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// Implement [Send] and [Sync] for [ddwaf_object]. There is nothing thread unsafe about these unless
// its pointers are dereferences, which is inherently unsafe anyway.
unsafe impl Send for ddwaf_object {}
unsafe impl Sync for ddwaf_object {}

#[warn(clippy::pedantic)]
impl ddwaf_object {
    /// Drops the key associated with the receiving [`ddwaf_object`].
    ///
    /// # Safety
    /// The key, if present, must be a raw-converted [`Box<[u8]>`]. After this method returns, the
    /// values of [`ddwaf_object::parameterName`] and [`ddwaf_object::parameterNameLength`] must be
    /// replaced as they will no longer be valid.
    ///
    /// # Panics
    /// If the key is too large for this platform (can only happens on 32-bit platforms).
    pub unsafe fn drop_key(&mut self) {
        if self.parameterName.is_null() {
            return;
        }
        let len =
            usize::try_from(self.parameterNameLength).expect("key is too large for this platform");
        let slice: &mut [u8] = unsafe {
            std::slice::from_raw_parts_mut(self.parameterName.cast::<u8>().cast_mut(), len)
        };
        drop(unsafe { Box::from_raw(std::ptr::from_mut(slice)) });
    }

    /// Drops the array data associated with the receiving [`ddwaf_object`].
    ///
    /// # Safety
    /// - The [`ddwaf_object`] must be a valid representation of an array.
    /// - The array must be an [`std::alloc::alloc`]ated array of [`ddwaf_object`] of the proper size.
    /// - The individual elements of the array must be valid [`ddwaf_object`]s that can be dropped
    ///   with [`ddwaf_object::drop_object`].
    ///
    /// # Panics
    /// If the array is too large for this platform (can only happens on 32-bit platforms).
    pub unsafe fn drop_array(&mut self) {
        debug_assert_eq!(self.type_, DDWAF_OBJ_ARRAY);
        if self.nbEntries == 0 {
            return;
        }
        let array = unsafe { self.__bindgen_anon_1.array };
        let len = isize::try_from(self.nbEntries).expect("array is too large for this platform");
        for i in 0..len {
            let elem = unsafe { &mut *array.offset(i) };
            unsafe { elem.drop_object() };
        }
        #[allow(clippy::cast_possible_truncation)] // We could cast to isize, and usize is wider.
        let layout = Layout::array::<ddwaf_object>(self.nbEntries as usize).unwrap();
        unsafe { std::alloc::dealloc(array.cast(), layout) };
    }

    /// Drops the map data associated with the receiving [`ddwaf_object`].
    ///
    /// # Safety
    /// - The [`ddwaf_object`] must be a valid representation of a map.
    /// - The map must be an [`std::alloc::alloc`]ated array of [`ddwaf_object`] of the proper size.
    /// - The individual elements of the map must be valid [`ddwaf_object`]s that can be dropped with
    ///   both [`ddwaf_object::drop_object`] and [`ddwaf_object::drop_key`].
    ///
    /// # Panics
    /// If the map is too large for this platform (can only happens on 32-bit platforms).
    pub unsafe fn drop_map(&mut self) {
        debug_assert_eq!(self.type_, DDWAF_OBJ_MAP);
        if self.nbEntries == 0 {
            return;
        }
        let array = unsafe { self.__bindgen_anon_1.array };
        let len = isize::try_from(self.nbEntries).expect("map is too large for this platform");
        for i in 0..len {
            let elem = unsafe { &mut *array.offset(i) };
            unsafe { elem.drop_key() };
            unsafe { elem.drop_object() };
        }
        #[allow(clippy::cast_possible_truncation)] // We could cast to isize, and usize is wider.
        let layout = Layout::array::<ddwaf_object>(self.nbEntries as usize).unwrap();
        unsafe { std::alloc::dealloc(array.cast(), layout) };
    }

    /// Drops the value associated with the receiving [`ddwaf_object`].
    ///
    /// # Safety
    /// If the [`ddwaf_object`] is a string, array, or map, the respective requirements of the
    /// [`ddwaf_object::drop_string`], [`ddwaf_object::drop_array`], or [`ddwaf_object::drop_map`]
    /// methods apply.
    pub unsafe fn drop_object(&mut self) {
        match self.type_ {
            DDWAF_OBJ_STRING => unsafe { self.drop_string() },
            DDWAF_OBJ_ARRAY => unsafe { self.drop_array() },
            DDWAF_OBJ_MAP => unsafe { self.drop_map() },
            _ => { /* nothing to do */ }
        }
    }

    /// Drops the string associated with the receiving [`ddwaf_object`].
    ///
    /// # Safety
    /// - The [`ddwaf_object`] must be a valid representation of a string
    /// - The [`ddwaf_object::__bindgen_anon_1`] field must have a
    ///   [`_ddwaf_object__bindgen_ty_1::stringValue`] set from a raw-converted [`Box<[u8]>`]
    ///
    /// # Panics
    /// If the string is too large for this platform (can only happens on 32-bit platforms).
    pub unsafe fn drop_string(&mut self) {
        debug_assert_eq!(self.type_, DDWAF_OBJ_STRING);
        let sval = unsafe { self.__bindgen_anon_1.stringValue };
        if sval.is_null() {
            return;
        }
        let len = usize::try_from(self.nbEntries).expect("string is too large for this platform");
        let slice: &mut [u8] = unsafe { std::slice::from_raw_parts_mut(sval as *mut _, len) };
        drop(unsafe { Box::from_raw(slice) });
    }
}
impl std::cmp::PartialEq<ddwaf_object> for ddwaf_object {
    fn eq(&self, other: &ddwaf_object) -> bool {
        if self.type_ != other.type_ {
            return false;
        }
        match self.type_ {
            DDWAF_OBJ_INVALID | DDWAF_OBJ_NULL => true,
            DDWAF_OBJ_SIGNED => unsafe {
                self.__bindgen_anon_1.intValue == other.__bindgen_anon_1.intValue
            },
            DDWAF_OBJ_UNSIGNED => unsafe {
                self.__bindgen_anon_1.uintValue == other.__bindgen_anon_1.uintValue
            },
            DDWAF_OBJ_BOOL => unsafe {
                self.__bindgen_anon_1.boolean == other.__bindgen_anon_1.boolean
            },
            DDWAF_OBJ_FLOAT => unsafe {
                // We do an exact comparison here, which ought to be okay as we normally don't do math here...
                self.__bindgen_anon_1.f64_ == other.__bindgen_anon_1.f64_
            },

            // Strings are a pointer, we need to compare the data they point to...
            DDWAF_OBJ_STRING => unsafe {
                if self.nbEntries != other.nbEntries {
                    return false;
                }
                if self.nbEntries == 0 {
                    return true;
                }
                let len =
                    usize::try_from(self.nbEntries).expect("string is too large for this platform");
                let left = slice::from_raw_parts(self.__bindgen_anon_1.stringValue, len);
                let right = slice::from_raw_parts(other.__bindgen_anon_1.stringValue, len);
                left == right
            },

            // Arrays and maps are pointers to collections, we need to compare the data they point to...
            DDWAF_OBJ_ARRAY | DDWAF_OBJ_MAP => unsafe {
                if self.nbEntries != other.nbEntries {
                    return false;
                }
                if self.nbEntries == 0 {
                    return true;
                }

                let left = slice::from_raw_parts(
                    self.__bindgen_anon_1.array,
                    usize::try_from(self.nbEntries)
                        .expect("array/map is too large for this platform"),
                );
                let right = slice::from_raw_parts(
                    other.__bindgen_anon_1.array,
                    usize::try_from(other.nbEntries)
                        .expect("array/map is too large for this platform"),
                );
                for (left, right) in left.iter().zip(right.iter()) {
                    if self.type_ == DDWAF_OBJ_MAP {
                        if left.parameterNameLength != right.parameterNameLength {
                            return false;
                        }
                        let len = usize::try_from(left.parameterNameLength)
                            .expect("key is too large for this platform");
                        if len > 0 {
                            let left_key = slice::from_raw_parts(left.parameterName, len);
                            let right_key = slice::from_raw_parts(right.parameterName, len);
                            if left_key != right_key {
                                return false;
                            }
                        }
                    }
                    if left != right {
                        return false;
                    }
                }
                true
            },

            _ => false,
        }
    }
}
impl std::fmt::Debug for ddwaf_object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("ddwaf_object");
        let dbg = match usize::try_from(self.parameterNameLength) {
            Ok(0) => &mut dbg,
            Ok(len) => {
                let key = unsafe { slice::from_raw_parts(self.parameterName.cast(), len) };
                let key = String::from_utf8_lossy(key);
                dbg.field("parameterName", &key)
            }
            Err(_) => {
                let key = unsafe { slice::from_raw_parts(self.parameterName.cast(), usize::MAX) };
                let key = String::from_utf8_lossy(key);
                dbg.field("parameterName(trunc)", &key)
            }
        };
        let dbg = match self.type_ {
            DDWAF_OBJ_BOOL => dbg
                .field("type", &stringify!(DDWAF_OBJ_BOOL))
                .field("boolean", unsafe { &self.__bindgen_anon_1.boolean }),
            DDWAF_OBJ_FLOAT => dbg
                .field("type", &stringify!(DDWAF_OBJ_FLOAT))
                .field("f64", unsafe { &self.__bindgen_anon_1.f64_ }),
            DDWAF_OBJ_SIGNED => dbg
                .field("type", &stringify!(DDWAF_OBJ_SIGNED))
                .field("int", unsafe { &self.__bindgen_anon_1.intValue }),
            DDWAF_OBJ_UNSIGNED => dbg
                .field("type", &stringify!(DDWAF_OBJ_UNSIGNED))
                .field("uint", unsafe { &self.__bindgen_anon_1.uintValue }),
            DDWAF_OBJ_STRING => {
                let (field, len) = match usize::try_from(self.nbEntries) {
                    Ok(len) => ("string", len),
                    Err(_) => ("string(trunc)", usize::MAX),
                };
                let sval =
                    unsafe { slice::from_raw_parts(self.__bindgen_anon_1.stringValue.cast(), len) };
                let sval = String::from_utf8_lossy(sval);
                dbg.field("type", &stringify!(DDWAF_OBJ_STRING))
                    .field(field, &sval)
            }
            DDWAF_OBJ_ARRAY => {
                let (field, len) = match usize::try_from(self.nbEntries) {
                    Ok(len) => ("array", len),
                    Err(_) => ("array(trunc)", usize::MAX),
                };
                let array: &[ddwaf_object] =
                    unsafe { slice::from_raw_parts(self.__bindgen_anon_1.array.cast(), len) };
                dbg.field("type", &stringify!(DDWAF_OBJ_ARRAY))
                    .field(field, &array)
            }
            DDWAF_OBJ_MAP => {
                let (field, len) = match usize::try_from(self.nbEntries) {
                    Ok(len) => ("map", len),
                    Err(_) => ("map(trunc)", usize::MAX),
                };
                let array: &[ddwaf_object] =
                    unsafe { slice::from_raw_parts(self.__bindgen_anon_1.array.cast(), len) };
                dbg.field("type", &stringify!(DDWAF_OBJ_MAP))
                    .field(field, &array)
            }
            DDWAF_OBJ_NULL => dbg.field("type", &stringify!(DDWAF_OBJ_NULL)),
            DDWAF_OBJ_INVALID => dbg.field("type", &stringify!(DDWAF_OBJ_INVALID)),
            unknown => dbg.field("type", &unknown),
        };

        dbg.finish_non_exhaustive()
    }
}
