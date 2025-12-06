use std::ffi::CStr;
use std::sync::atomic::{AtomicUsize, Ordering};

use libddwaf::log::*;

static LOG_COUNT: AtomicUsize = AtomicUsize::new(0);

fn test_callback(_: Level, _: &CStr, _: &CStr, _: u32, _: &[u8]) {
    LOG_COUNT.fetch_add(1, Ordering::Relaxed);
}

#[test]
fn test_log_callback() {
    // We start with 0 logs processed
    assert_eq!(LOG_COUNT.load(Ordering::SeqCst), 0);
    unsafe { set_log_cb(test_callback, Level::Debug) };
    // Setting the logger emits 1 log entry
    assert_eq!(LOG_COUNT.load(Ordering::SeqCst), 1);

    // Un-setting the logger would emit 1 log entry, but there is no logger...
    unsafe { reset_log_cb() };
    assert_eq!(LOG_COUNT.load(Ordering::SeqCst), 1);
}

#[test]
fn test_level_display() {
    assert_eq!(format!("{}", Level::Trace), "TRACE");
    assert_eq!(format!("{}", Level::Debug), "DEBUG");
    assert_eq!(format!("{}", Level::Info), "INFO");
    assert_eq!(format!("{}", Level::Warn), "WARN");
    assert_eq!(format!("{}", Level::Error), "ERROR");
    assert_eq!(format!("{}", Level::Off), "OFF");
}

#[test]
fn test_level_try_from() {
    assert_eq!(
        Level::try_from(libddwaf_sys::DDWAF_LOG_TRACE).unwrap(),
        Level::Trace
    );
    assert_eq!(
        Level::try_from(libddwaf_sys::DDWAF_LOG_DEBUG).unwrap(),
        Level::Debug
    );
    assert_eq!(
        Level::try_from(libddwaf_sys::DDWAF_LOG_INFO).unwrap(),
        Level::Info
    );
    assert_eq!(
        Level::try_from(libddwaf_sys::DDWAF_LOG_WARN).unwrap(),
        Level::Warn
    );
    assert_eq!(
        Level::try_from(libddwaf_sys::DDWAF_LOG_ERROR).unwrap(),
        Level::Error
    );
    assert_eq!(
        Level::try_from(libddwaf_sys::DDWAF_LOG_OFF).unwrap(),
        Level::Off
    );

    // Test unknown level
    let result = Level::try_from(0xFF);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(format!("{}", err), "Unknown log level: 0xFF");
}
