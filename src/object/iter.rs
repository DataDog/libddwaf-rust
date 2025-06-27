use std::alloc::Layout;
use std::ptr::null_mut;

impl IntoIterator for super::WAFArray {
    type Item = super::WAFObject;
    type IntoIter = WAFIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let array: *mut Self::Item = unsafe { self.raw.__bindgen_anon_1.array.cast() };
        let len = if array.is_null() { 0 } else { self.len() };
        // Forget about self, since the iterator is now the owner of the memory.
        std::mem::forget(self);
        WAFIter { array, len, pos: 0 }
    }
}

impl IntoIterator for super::Keyed<super::WAFArray> {
    type Item = super::WAFObject;
    type IntoIter = WAFIter<Self::Item>;

    fn into_iter(mut self) -> Self::IntoIter {
        let mut arr = std::mem::take(&mut self.value);

        // We're stopping the destructor from [super::Keyed] from running, so we need to explicitly
        // drop the key from the value, or we'd be leaking it.
        let raw_obj = unsafe { super::AsRawMutObject::as_raw_mut(&mut arr) };
        unsafe { raw_obj.drop_key() };
        raw_obj.parameterName = null_mut();
        raw_obj.parameterNameLength = 0;

        // Finally delegate back to the [super::WAFArray] implementation.
        arr.into_iter()
    }
}

impl IntoIterator for super::WAFMap {
    type Item = super::Keyed<super::WAFObject>;
    type IntoIter = WAFIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let array: *mut super::Keyed<super::WAFObject> =
            unsafe { self.raw.__bindgen_anon_1.array.cast() };
        let len = if array.is_null() { 0 } else { self.len() };
        // Forget about self, since the iterator is now the owner of the memory.
        std::mem::forget(self);
        WAFIter { array, len, pos: 0 }
    }
}

impl IntoIterator for super::Keyed<super::WAFMap> {
    type Item = super::Keyed<super::WAFObject>;
    type IntoIter = WAFIter<Self::Item>;

    fn into_iter(mut self) -> Self::IntoIter {
        let mut arr = std::mem::take(&mut self.value);

        // We're stopping the destructor from [super::Keyed] from running, so we need to explicitly
        // drop the key from the value, or we'd be leaking it.
        let raw_obj = unsafe { super::AsRawMutObject::as_raw_mut(&mut arr) };
        unsafe { raw_obj.drop_key() };
        raw_obj.parameterName = null_mut();
        raw_obj.parameterNameLength = 0;

        // Finally delegate back to the [super::WAFMap] implementation.
        arr.into_iter()
    }
}

/// An iterator over an [`WAFArray`][super::WAFArray] or [`WAFMap`][super::WAFMap].
pub struct WAFIter<T> {
    array: *mut T,
    len: usize,
    pos: usize,
}
impl<T: Default> Iterator for WAFIter<T> {
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
impl<T> Drop for WAFIter<T> {
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
