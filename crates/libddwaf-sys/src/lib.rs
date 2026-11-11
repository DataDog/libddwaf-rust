#![crate_type = "dylib"]
#![deny(clippy::correctness, clippy::perf, clippy::style, clippy::suspicious)]
#![allow(unused)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unsafe_op_in_unsafe_fn)] // Bindgen generates some offending code...
#![allow(clippy::missing_safety_doc)] // Bindgen generates undocumented unsafe bitfield accessors.
#![allow(clippy::ptr_offset_with_cast)] // Bindgen uses offset when accessing bitfield storage.
#![allow(clippy::unnecessary_cast)] // Bindgen casts bitfield values to their existing type.
#![allow(clippy::useless_transmute)] // Bindgen emits identity transmutes for unsigned bitfields.

#[cfg(any(feature = "source-static", feature = "source-shared"))]
extern crate libddwaf_src;

use std::alloc::Layout;
use std::ptr::null;
use std::slice;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(feature = "dynamic")]
mod dylib;
#[cfg(feature = "dynamic")]
pub use dylib::*;

// Implement [Send] and [Sync] for [ddwaf_object]. There is nothing thread unsafe about these unless
// its pointers are dereferences, which is inherently unsafe anyway.
unsafe impl Send for ddwaf_object {}
unsafe impl Sync for ddwaf_object {}

#[warn(clippy::pedantic)]
impl ddwaf_object {
    /// Drops the array data associated with the receiving [`ddwaf_object`].
    ///
    /// # Safety
    /// - The [`ddwaf_object`] must be a valid representation of an array.
    /// - The array must be an [`std::alloc::alloc`]ated array of [`ddwaf_object`] of the proper size.
    /// - The individual elements of the array must be valid [`ddwaf_object`]s that can be dropped
    ///   with [`ddwaf_object::drop_object`].
    ///
    /// # Panics
    /// Panics if the capacity is too large to construct a valid allocation layout. This is only
    /// possible on 32-bit targets because the capacity is limited to 28 bits.
    pub unsafe fn drop_array(&mut self) {
        debug_assert!(self.is_array());
        let size = self.array_len();
        let capacity = self.array_capacity();
        if capacity == 0 {
            return;
        }
        let ptr = self.array_ptr();
        for i in 0..size {
            let elem = unsafe { &mut *ptr.add(i) };
            unsafe { elem.drop_object() };
        }
        let layout = Layout::array::<ddwaf_object>(capacity).unwrap();
        unsafe { std::alloc::dealloc(ptr.cast(), layout) };
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
    /// Panics if the capacity is too large to construct a valid allocation layout. This is only
    /// possible on 32-bit targets because the capacity is limited to 28 bits.
    pub unsafe fn drop_map(&mut self) {
        debug_assert!(self.is_map());
        let size = self.map_len();
        let capacity = self.map_capacity();
        if capacity == 0 {
            return;
        }
        let ptr = self.map_ptr();
        for i in 0..size {
            let elem = unsafe { &mut *ptr.add(i) };
            unsafe { elem.key.drop_object() };
            unsafe { elem.val.drop_object() };
        }
        let layout = Layout::array::<_ddwaf_object_kv>(capacity).unwrap();
        unsafe { std::alloc::dealloc(ptr.cast(), layout) };
    }

    /// Drops the value associated with the receiving [`ddwaf_object`].
    ///
    /// # Safety
    /// If the [`ddwaf_object`] is a string, array, or map, the respective requirements of the
    /// [`ddwaf_object::drop_string`], [`ddwaf_object::drop_array`], or [`ddwaf_object::drop_map`]
    /// methods apply.
    /// The method can't be called more than once.
    pub unsafe fn drop_object(&mut self) {
        match self.obj_type() {
            DDWAF_OBJ_STRING => unsafe { self.drop_string() },
            DDWAF_OBJ_ARRAY | DDWAF_OBJ_LARGE_ARRAY => unsafe { self.drop_array() },
            DDWAF_OBJ_MAP | DDWAF_OBJ_LARGE_MAP => unsafe { self.drop_map() },
            _ => { /* nothing to do */ }
        }
    }

    /// Drops the regular string associated with the receiving [`ddwaf_object`].
    ///
    /// # Safety
    /// - The [`ddwaf_object`] must be a valid representation of a string
    /// - The [`_ddwaf_object__bindgen_ty_1::str_`] field must have a
    ///   [`_ddwaf_object_string::ptr`] set from an allocation of `c_char` of the
    ///   size indicated by the [`_ddwaf_object_string::size`] field done with [`std::alloc::alloc`].
    #[allow(clippy::missing_panics_doc)]
    pub unsafe fn drop_string(&mut self) {
        debug_assert_eq!(self.obj_type(), DDWAF_OBJ_STRING);
        let sval = unsafe { self.via.str_.ptr };
        if sval.is_null() {
            return;
        }
        unsafe {
            std::alloc::dealloc(
                sval.cast(),
                Layout::array::<::std::os::raw::c_char>(self.via.str_.size as usize).unwrap(),
            );
        }
    }

    /// Returns the type of the [`ddwaf_object`]
    #[must_use]
    pub fn obj_type(&self) -> DDWAF_OBJ_TYPE {
        DDWAF_OBJ_TYPE::from(unsafe { self.type_ })
    }

    /// Returns true if the [`ddwaf_object`] is a string.
    #[must_use]
    pub fn is_string(&self) -> bool {
        (self.obj_type() & DDWAF_OBJ_STRING) != 0
    }

    /// Returns true if the [`ddwaf_object`] is either array representation.
    #[must_use]
    pub fn is_array(&self) -> bool {
        matches!(self.obj_type(), DDWAF_OBJ_ARRAY | DDWAF_OBJ_LARGE_ARRAY)
    }

    /// Returns true if the [`ddwaf_object`] is either map representation.
    #[must_use]
    pub fn is_map(&self) -> bool {
        matches!(self.obj_type(), DDWAF_OBJ_MAP | DDWAF_OBJ_LARGE_MAP)
    }

    /// Returns the length of the array associated with the receiving [`ddwaf_object`].
    ///
    /// # Panics
    /// Panics if the object is not an array.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn array_len(&self) -> usize {
        match self.obj_type() {
            DDWAF_OBJ_ARRAY => usize::from(unsafe { self.via.array.size }),
            DDWAF_OBJ_LARGE_ARRAY => unsafe { self.via.large_array.size() as usize },
            object_type => panic!("object of type {object_type} is not an array"),
        }
    }

    /// Returns the capacity of the array associated with the receiving [`ddwaf_object`].
    ///
    /// # Panics
    /// Panics if the object is not an array.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn array_capacity(&self) -> usize {
        match self.obj_type() {
            DDWAF_OBJ_ARRAY => usize::from(unsafe { self.via.array.capacity }),
            DDWAF_OBJ_LARGE_ARRAY => unsafe { self.via.large_array.capacity() as usize },
            object_type => panic!("object of type {object_type} is not an array"),
        }
    }

    /// Returns the element pointer of the array associated with the receiving [`ddwaf_object`].
    ///
    /// # Panics
    /// Panics if the object is not an array.
    #[must_use]
    pub fn array_ptr(&self) -> *mut ddwaf_object {
        match self.obj_type() {
            DDWAF_OBJ_ARRAY => unsafe { self.via.array.ptr },
            DDWAF_OBJ_LARGE_ARRAY => unsafe { self.via.large_array.ptr },
            object_type => panic!("object of type {object_type} is not an array"),
        }
    }

    /// Changes the length of the array associated with the receiving [`ddwaf_object`].
    ///
    /// # Safety
    /// Every element before `new_len` must be initialized and valid to drop.
    ///
    /// # Panics
    /// Panics if `new_len` exceeds the array's capacity.
    pub unsafe fn set_array_len(&mut self, new_len: usize) {
        assert!(new_len <= self.array_capacity());
        match self.obj_type() {
            DDWAF_OBJ_ARRAY => {
                self.via.array.size = new_len.try_into().expect("compact array length overflow");
            }
            DDWAF_OBJ_LARGE_ARRAY => unsafe {
                self.via.large_array.set_size(new_len as u64);
            },
            _ => unreachable!(),
        }
    }

    /// Returns the length of the map associated with the receiving [`ddwaf_object`].
    ///
    /// # Panics
    /// Panics if the object is not a map.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn map_len(&self) -> usize {
        match self.obj_type() {
            DDWAF_OBJ_MAP => usize::from(unsafe { self.via.map.size }),
            DDWAF_OBJ_LARGE_MAP => unsafe { self.via.large_map.size() as usize },
            object_type => panic!("object of type {object_type} is not a map"),
        }
    }

    /// Returns the capacity of the map associated with the receiving [`ddwaf_object`].
    ///
    /// # Panics
    /// Panics if the object is not a map.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn map_capacity(&self) -> usize {
        match self.obj_type() {
            DDWAF_OBJ_MAP => usize::from(unsafe { self.via.map.capacity }),
            DDWAF_OBJ_LARGE_MAP => unsafe { self.via.large_map.capacity() as usize },
            object_type => panic!("object of type {object_type} is not a map"),
        }
    }

    /// Returns the entry pointer of the map associated with the receiving [`ddwaf_object`].
    ///
    /// # Panics
    /// Panics if the object is not a map.
    #[must_use]
    pub fn map_ptr(&self) -> *mut _ddwaf_object_kv {
        match self.obj_type() {
            DDWAF_OBJ_MAP => unsafe { self.via.map.ptr },
            DDWAF_OBJ_LARGE_MAP => unsafe { self.via.large_map.ptr },
            object_type => panic!("object of type {object_type} is not a map"),
        }
    }

    /// Changes the length of the map associated with the receiving [`ddwaf_object`].
    ///
    /// # Safety
    /// Every entry before `new_len` must be initialized and valid to drop.
    ///
    /// # Panics
    /// Panics if `new_len` exceeds the map's capacity.
    pub unsafe fn set_map_len(&mut self, new_len: usize) {
        assert!(new_len <= self.map_capacity());
        match self.obj_type() {
            DDWAF_OBJ_MAP => {
                self.via.map.size = new_len.try_into().expect("compact map length overflow");
            }
            DDWAF_OBJ_LARGE_MAP => unsafe {
                self.via.large_map.set_size(new_len as u64);
            },
            _ => unreachable!(),
        }
    }

    /// Returns a slice of the bytes from the string associated with the receiving [`ddwaf_object`].
    ///
    /// # Safety
    /// - The [`ddwaf_object`] must be a valid representation of a string.
    unsafe fn string_vec(&self) -> &[u8] {
        debug_assert!(self.is_string());

        if self.obj_type() == DDWAF_OBJ_STRING || self.obj_type() == DDWAF_OBJ_LITERAL_STRING {
            let str = unsafe { self.via.str_ };
            if str.size == 0 {
                return &[];
            }
            unsafe { slice::from_raw_parts(str.ptr.cast(), str.size as usize) }
        } else {
            let sstr = unsafe { &self.via.sstr };
            let data = &sstr.data[..sstr.size as usize];
            // reinterpret &[i8] as &[u8]
            unsafe { std::slice::from_raw_parts(data.as_ptr().cast(), data.len()) }
        }
    }
}

impl std::cmp::PartialEq<ddwaf_object> for ddwaf_object {
    fn eq(&self, other: &ddwaf_object) -> bool {
        if self.is_string() && other.is_string() {
            let left = unsafe { self.string_vec() };
            let right = unsafe { other.string_vec() };
            return left == right;
        }

        if self.is_array() && other.is_array() {
            if self.array_len() != other.array_len() {
                return false;
            }
            for i in 0..self.array_len() {
                let left = unsafe { &*self.array_ptr().add(i) };
                let right = unsafe { &*other.array_ptr().add(i) };
                if left != right {
                    return false;
                }
            }
            return true;
        }

        if self.is_map() && other.is_map() {
            if self.map_len() != other.map_len() {
                return false;
            }
            for i in 0..self.map_len() {
                let left = unsafe { &*self.map_ptr().add(i) };
                let right = unsafe { &*other.map_ptr().add(i) };
                if left.key != right.key || left.val != right.val {
                    return false;
                }
            }
            return true;
        }

        if unsafe { self.type_ != other.type_ } {
            return false;
        }
        match self.obj_type() {
            DDWAF_OBJ_INVALID | DDWAF_OBJ_NULL => true,
            DDWAF_OBJ_SIGNED => unsafe { self.via.i64_.val == other.via.i64_.val },
            DDWAF_OBJ_UNSIGNED => unsafe { self.via.u64_.val == other.via.u64_.val },
            DDWAF_OBJ_BOOL => unsafe { self.via.b8.val == other.via.b8.val },
            DDWAF_OBJ_FLOAT => unsafe { self.via.f64_.val == other.via.f64_.val },
            _ => false,
        }
    }
}
impl std::fmt::Debug for ddwaf_object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("ddwaf_object");
        match self.obj_type() {
            DDWAF_OBJ_BOOL => dbg
                .field("type", &stringify!(DDWAF_OBJ_BOOL))
                .field("boolean", unsafe { &self.via.b8.val }),
            DDWAF_OBJ_FLOAT => dbg
                .field("type", &stringify!(DDWAF_OBJ_FLOAT))
                .field("f64", unsafe { &self.via.f64_.val }),
            DDWAF_OBJ_SIGNED => dbg
                .field("type", &stringify!(DDWAF_OBJ_SIGNED))
                .field("int", unsafe { &self.via.i64_.val }),
            DDWAF_OBJ_UNSIGNED => dbg
                .field("type", &stringify!(DDWAF_OBJ_UNSIGNED))
                .field("uint", unsafe { &self.via.u64_.val }),
            DDWAF_OBJ_STRING | DDWAF_OBJ_LITERAL_STRING => {
                let sval = unsafe { self.string_vec() };
                let sval = String::from_utf8_lossy(sval);
                dbg.field(
                    "type",
                    if self.obj_type() == DDWAF_OBJ_STRING {
                        &stringify!(DDWAF_OBJ_STRING)
                    } else {
                        &stringify!(DDWAF_OBJ_LITERAL_STRING)
                    },
                )
                .field("string", &sval)
            }
            DDWAF_OBJ_SMALL_STRING => {
                let sval = unsafe { self.string_vec() };
                let sval = String::from_utf8_lossy(sval);
                dbg.field("type", &stringify!(DDWAF_OBJ_SMALL_STRING))
                    .field("string", &sval)
            }
            DDWAF_OBJ_ARRAY | DDWAF_OBJ_LARGE_ARRAY => {
                let array: &[ddwaf_object] = if self.array_len() == 0 {
                    &[]
                } else {
                    unsafe { slice::from_raw_parts(self.array_ptr(), self.array_len()) }
                };
                let object_type = if self.obj_type() == DDWAF_OBJ_ARRAY {
                    stringify!(DDWAF_OBJ_ARRAY)
                } else {
                    stringify!(DDWAF_OBJ_LARGE_ARRAY)
                };
                dbg.field("type", &object_type).field("array", &array)
            }
            DDWAF_OBJ_MAP | DDWAF_OBJ_LARGE_MAP => {
                let map: &[_ddwaf_object_kv] = if self.map_len() == 0 {
                    &[]
                } else {
                    unsafe { slice::from_raw_parts(self.map_ptr(), self.map_len()) }
                };
                let object_type = if self.obj_type() == DDWAF_OBJ_MAP {
                    stringify!(DDWAF_OBJ_MAP)
                } else {
                    stringify!(DDWAF_OBJ_LARGE_MAP)
                };
                dbg.field("type", &object_type).field("map", &map)
            }
            DDWAF_OBJ_NULL => dbg.field("type", &stringify!(DDWAF_OBJ_NULL)),
            DDWAF_OBJ_INVALID => dbg.field("type", &stringify!(DDWAF_OBJ_INVALID)),
            unknown => dbg.field("type", &unknown),
        };

        dbg.finish_non_exhaustive()
    }
}

impl std::fmt::Debug for _ddwaf_object_kv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("ddwaf_object_kv");
        dbg.field("key", &self.key)
            .field("val", &self.val)
            .finish_non_exhaustive()
    }
}
