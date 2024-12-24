use std::{alloc::Layout, ops::Deref};

use crate::{bindings, AsRawDdwafObjMut, DdwafObjArray, DdwafObjMap, Keyed};

#[repr(C)]
pub struct DdwafObjArrayShallow<'a> {
    inner: std::mem::ManuallyDrop<DdwafObjArray>,
    _phantom: std::marker::PhantomData<fn(&'a ())>,
}
impl<'a> DdwafObjArrayShallow<'a> {
    pub fn new(size: u64) -> Self {
        Self {
            inner: std::mem::ManuallyDrop::new(DdwafObjArray::new(size)),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn set_slot(&mut self, idx: u64, obj: &'a impl AsRef<bindings::ddwaf_object>) {
        let dobj_arr = &mut *self.inner;
        if idx as usize >= dobj_arr.len() {
            panic!("Index out of bounds");
        }
        let array = unsafe { dobj_arr._obj.__bindgen_anon_1.array };
        unsafe {
            *array.add(idx as usize) = *obj.as_ref();
        }
    }
}
impl Deref for DdwafObjArrayShallow<'_> {
    type Target = DdwafObjArray;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl Drop for DdwafObjArrayShallow<'_> {
    fn drop(&mut self) {
        let array_len = self.len();
        if array_len == 0 {
            return;
        }
        let dobj = &mut self.inner._obj;
        let array = unsafe { dobj.__bindgen_anon_1.array };
        let layout = Layout::array::<bindings::ddwaf_object>(array_len).unwrap();
        unsafe { std::alloc::dealloc(array as *mut u8, layout) };
    }
}

#[repr(C)]
pub struct DdwafObjMapShallow<'a> {
    inner: std::mem::ManuallyDrop<DdwafObjMap>,
    _phantom: std::marker::PhantomData<fn(&'a ())>,
}
impl<'a> DdwafObjMapShallow<'a> {
    pub fn new(size: u64) -> Self {
        Self {
            inner: std::mem::ManuallyDrop::new(DdwafObjMap::new(size)),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn set_slot<T: AsRef<bindings::ddwaf_object> + AsRawDdwafObjMut>(
        &mut self,
        idx: u64,
        obj: &'a Keyed<T>,
    ) {
        let dobj_map = &mut *self.inner;
        if idx as usize >= dobj_map.len() {
            panic!("Index out of bounds");
        }
        let array = unsafe { dobj_map._obj.__bindgen_anon_1.array };
        unsafe {
            let el = &mut *array.add(idx as usize);
            std::ptr::copy_nonoverlapping(obj.as_ref() as *const _, el as *mut _, 1);
        }
    }
}
impl Deref for DdwafObjMapShallow<'_> {
    type Target = DdwafObjMap;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl Drop for DdwafObjMapShallow<'_> {
    fn drop(&mut self) {
        let array_len = self.len();
        if array_len == 0 {
            return;
        }
        let dobj = &mut self.inner._obj;
        let array = unsafe { dobj.__bindgen_anon_1.array };
        let layout = Layout::array::<bindings::ddwaf_object>(array_len).unwrap();
        unsafe { std::alloc::dealloc(array as *mut u8, layout) };
    }
}

mod test {
    #[allow(unused_imports)]
    use crate::shallow::DdwafObjArrayShallow;
    #[allow(unused_imports)]
    use crate::{ddwaf_obj, ddwaf_obj_array, CommonDdwafObj, DdwafObjArray};

    #[test]
    fn test_shallow_array() {
        let obj1 = ddwaf_obj!("foobar");
        let obj2 = ddwaf_obj!(ddwaf_obj_array!(1, 2, 3));
        let mut array = DdwafObjArrayShallow::new(3);
        array.set_slot(0, &obj1);
        array.set_slot(1, &obj2);
        println!("{}", array.debug_str(0));
    }
}
