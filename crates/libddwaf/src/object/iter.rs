use std::alloc::Layout;

use crate::object::{Keyed, WafArray, WafMap, WafObject};

impl IntoIterator for WafArray {
    type Item = WafObject;
    type IntoIter = WafIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let array: *mut Self::Item = unsafe { self.raw.via.array.ptr.cast() };
        let len = if array.is_null() {
            0
        } else {
            self.len() as usize
        };
        // Forget about self, since the iterator is now the owner of the memory.
        std::mem::forget(self);
        WafIter { array, len, pos: 0 }
    }
}

impl IntoIterator for Keyed<WafArray> {
    type Item = WafObject;
    type IntoIter = WafIter<Self::Item>;

    fn into_iter(mut self) -> Self::IntoIter {
        let arr = std::mem::take(self.value_mut());
        arr.into_iter()
    }
}

impl IntoIterator for WafMap {
    type Item = Keyed<WafObject>;
    type IntoIter = WafIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let array: *mut Keyed<WafObject> = unsafe { self.raw.via.map.ptr.cast() };
        let len = if array.is_null() {
            0
        } else {
            self.len() as usize
        };
        // Forget about self, since the iterator is now the owner of the memory.
        std::mem::forget(self);
        WafIter { array, len, pos: 0 }
    }
}

impl IntoIterator for Keyed<WafMap> {
    type Item = Keyed<WafObject>;
    type IntoIter = WafIter<Self::Item>;

    fn into_iter(mut self) -> Self::IntoIter {
        let arr = std::mem::take(self.value_mut());
        arr.into_iter()
    }
}

/// An iterator over an [`WafArray`] or [`WafMap`].
pub struct WafIter<T> {
    array: *mut T,
    len: usize,
    pos: usize,
}
impl<T: Default> Iterator for WafIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.len {
            return None;
        }

        let obj = unsafe { &mut *self.array.add(self.pos) };
        self.pos += 1;
        Some(std::mem::take(obj))
    }
}
impl<T> Drop for WafIter<T> {
    fn drop(&mut self) {
        // Drop the remaining elements in the array...
        for i in self.pos..self.len {
            let elem = unsafe { self.array.add(i) };
            unsafe { elem.drop_in_place() };
        }
        if self.len != 0 {
            // Finally, drop the array itself.
            let layout = Layout::array::<T>(self.len).unwrap();
            unsafe { std::alloc::dealloc(self.array.cast(), layout) }
        }
    }
}
