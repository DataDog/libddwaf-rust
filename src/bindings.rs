#![allow(unused)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::pedantic)]

use std::alloc::Layout;
use std::ptr::null;

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
    /// The key, if present, must be a raw-converted Box<[u8]>. After this method returns, the
    /// values of [`ddwaf_object::parameterName`] and [`ddwaf_object::parameterNameLength`] must be
    /// replaced as they will no longer be valid.
    pub(crate) unsafe fn drop_key(&mut self) {
        if self.parameterName.is_null() {
            return;
        }
        let len =
            usize::try_from(self.parameterNameLength).expect("key is too large for this platform");
        let slice: &mut [u8] =
            std::slice::from_raw_parts_mut(self.parameterName.cast::<u8>().cast_mut(), len);
        drop(Box::from_raw(std::ptr::from_mut(slice)));
    }

    /// Drops the array data associated with the receiving [`ddwaf_object`].
    ///
    /// # Safety
    /// - The [`ddwaf_object`] must be a valid representation of an array.
    /// - The array must be an [`std::alloc::alloc`]ated array of [`ddwaf_object`] of the proper size.
    /// - The individual elements of the array must be valid [`ddwaf_object`]s that can be dropped
    ///   with [`ddwaf_object::drop_object`].
    pub(crate) unsafe fn drop_array(&mut self) {
        debug_assert_eq!(self.type_, DDWAF_OBJ_TYPE_DDWAF_OBJ_ARRAY);
        if self.nbEntries == 0 {
            return;
        }
        let array = self.__bindgen_anon_1.array;
        let len = isize::try_from(self.nbEntries).expect("array is too large for this platform");
        for i in 0..len {
            let elem = &mut *array.offset(i);
            elem.drop_object();
        }
        #[allow(clippy::cast_possible_truncation)] // We could cast to isize, and usize is wider.
        let layout = Layout::array::<ddwaf_object>(self.nbEntries as usize).unwrap();
        std::alloc::dealloc(array.cast(), layout);
    }

    /// Drops the map data associated with the receiving [`ddwaf_object`].
    ///
    /// # Safety
    /// - The [`ddwaf_object`] must be a valid representation of a map.
    /// - The map must be an [`std::alloc::alloc`]ated array of [`ddwaf_object`] of the proper size.
    /// - The individual elements of the map must be valid [`ddwaf_object`]s that can be dropped with
    ///   both [`ddwaf_object::drop_object`] and [`ddwaf_object::drop_key`].
    pub(crate) unsafe fn drop_map(&mut self) {
        debug_assert_eq!(self.type_, DDWAF_OBJ_TYPE_DDWAF_OBJ_MAP);
        if self.nbEntries == 0 {
            return;
        }
        let array = self.__bindgen_anon_1.array;
        let len = isize::try_from(self.nbEntries).expect("map is too large for this platform");
        for i in 0..len {
            let elem = &mut *array.offset(i);
            elem.drop_key();
            elem.drop_object();
        }
        #[allow(clippy::cast_possible_truncation)] // We could cast to isize, and usize is wider.
        let layout = Layout::array::<ddwaf_object>(self.nbEntries as usize).unwrap();
        std::alloc::dealloc(array.cast(), layout);
    }

    /// Drops the value associated with the receiving [`ddwaf_object`].
    ///
    /// # Safety
    /// If the [`ddwaf_object`] is a string, array, or map, the respective requirements of the
    /// [`ddwaf_object::drop_string`], [`ddwaf_object::drop_array`], or [`ddwaf_object::drop_map`]
    /// methods apply.
    pub(crate) unsafe fn drop_object(&mut self) {
        match self.type_ {
            DDWAF_OBJ_TYPE_DDWAF_OBJ_STRING => self.drop_string(),
            DDWAF_OBJ_TYPE_DDWAF_OBJ_ARRAY => self.drop_array(),
            DDWAF_OBJ_TYPE_DDWAF_OBJ_MAP => self.drop_map(),
            _ => { /* nothing to do */ }
        }
    }

    /// Drops the string associated with the receiving [`ddwaf_object`].
    ///
    /// # Safety
    /// - The [`ddwaf_object`] must be a valid representation of a string
    /// - The [`ddwaf_object::__bindgen_anon_1`] field must have a
    ///   [`_ddwaf_object__bindgen_ty_1::stringValue`] set from a raw-converted [`Box<[u8]>`]
    pub(crate) unsafe fn drop_string(&mut self) {
        debug_assert_eq!(self.type_, DDWAF_OBJ_TYPE_DDWAF_OBJ_STRING);
        let sval = self.__bindgen_anon_1.stringValue;
        if sval.is_null() {
            return;
        }
        let len = usize::try_from(self.nbEntries).expect("string is too large for this platform");
        let slice: &mut [u8] = std::slice::from_raw_parts_mut(sval as *mut _, len);
        drop(Box::from_raw(slice));
    }
}
