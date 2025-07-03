use std::ffi::CStr;
use std::sync::atomic::{AtomicUsize, Ordering};

use libddwaf::log::*;

static LOG_COUNT: AtomicUsize = AtomicUsize::new(0);

fn test_callback(_: LogLevel, _: &CStr, _: &CStr, _: u32, _: &[u8]) {
    LOG_COUNT.fetch_add(1, Ordering::Relaxed);
}

#[test]
fn test_log_callback() {
    // We start with 0 logs processed
    assert_eq!(LOG_COUNT.load(Ordering::SeqCst), 0);
    unsafe { set_log_cb(test_callback, LogLevel::Debug) };
    // Setting the logger emits 1 log entry
    assert_eq!(LOG_COUNT.load(Ordering::SeqCst), 1);

    // Un-setting the logger would emit 1 log entry, but there is no logger...
    unsafe { reset_log_cb() };
    assert_eq!(LOG_COUNT.load(Ordering::SeqCst), 1);
}
