use std::alloc::{GlobalAlloc, Layout, System};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use libddwaf::object::{WafArray, WafObject};

struct TrackingAllocator;

static TARGET_ALLOCATION: AtomicPtr<u8> = AtomicPtr::new(null_mut());
static DEALLOCATION_SIZE: AtomicUsize = AtomicUsize::new(usize::MAX);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr == TARGET_ALLOCATION.load(Ordering::Relaxed) {
            DEALLOCATION_SIZE.store(layout.size(), Ordering::Relaxed);
        }
        unsafe { System.dealloc(ptr, layout) };
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

#[test]
fn consuming_truncated_empty_array_deallocates_its_storage() {
    const CAPACITY: usize = 257;

    let mut array = WafArray::new(257).unwrap();
    let elements: &[WafObject] = array.as_ref();
    TARGET_ALLOCATION.store(elements.as_ptr().cast_mut().cast(), Ordering::Relaxed);

    array.truncate(0);
    drop(array.into_iter());

    assert_eq!(
        DEALLOCATION_SIZE.load(Ordering::Relaxed),
        CAPACITY * size_of::<WafObject>()
    );
    TARGET_ALLOCATION.store(null_mut(), Ordering::Relaxed);
}
