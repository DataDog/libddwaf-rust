//! Access to the in-app WAF's logging facility.

use std::ffi::CStr;
use std::{error, fmt, slice};

type LogCallback = Box<dyn Fn(Level, &'static CStr, &'static CStr, u32, &[u8])>;

static mut LOG_CB: Option<LogCallback> = None;

/// Sets the log callback function.
///
/// # Safety
///
/// This function is unsafe because it writes to a static variable without synchronization.
/// It should only be used during startup.
pub unsafe fn set_log_cb(
    cb: impl Fn(Level, &'static CStr, &'static CStr, u32, &[u8]) + 'static,
    min_level: Level,
) {
    unsafe { LOG_CB = Some(Box::new(cb)) };
    unsafe { libddwaf_sys::ddwaf_set_log_cb(Some(bridge_log_cb), min_level.as_raw()) };
}

/// Resets the log callback function (to the default of "none").
///
/// # Safety
///
/// This function is unsafe because it writes to a static variable without synchronization.
/// It should only be used during startup.
pub unsafe fn reset_log_cb() {
    unsafe { libddwaf_sys::ddwaf_set_log_cb(None, Level::Off.as_raw()) };
    unsafe { LOG_CB = None };
}

/// Logging levels supported by the WAF.
#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Level {
    /// Extremely detailed logging.
    Trace,
    /// Detailed logging.
    Debug,
    /// Informational logging.
    Info,
    /// Log only warnings and errors.
    Warn,
    /// Log only errors.
    Error,
    /// Do not log anything.
    Off,
}
impl Level {
    const fn as_raw(self) -> libddwaf_sys::DDWAF_LOG_LEVEL {
        match self {
            Self::Trace => libddwaf_sys::DDWAF_LOG_TRACE,
            Self::Debug => libddwaf_sys::DDWAF_LOG_DEBUG,
            Self::Info => libddwaf_sys::DDWAF_LOG_INFO,
            Self::Warn => libddwaf_sys::DDWAF_LOG_WARN,
            Self::Error => libddwaf_sys::DDWAF_LOG_ERROR,
            Self::Off => libddwaf_sys::DDWAF_LOG_OFF,
        }
    }
}
impl fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trace => write!(f, "TRACE"),
            Self::Debug => write!(f, "DEBUG"),
            Self::Info => write!(f, "INFO"),
            Self::Warn => write!(f, "WARN"),
            Self::Error => write!(f, "ERROR"),
            Self::Off => write!(f, "OFF"),
        }
    }
}
impl TryFrom<libddwaf_sys::DDWAF_LOG_LEVEL> for Level {
    type Error = UnknownLogLevelError;

    fn try_from(value: libddwaf_sys::DDWAF_LOG_LEVEL) -> Result<Self, UnknownLogLevelError> {
        match value {
            libddwaf_sys::DDWAF_LOG_TRACE => Ok(Level::Trace),
            libddwaf_sys::DDWAF_LOG_DEBUG => Ok(Level::Debug),
            libddwaf_sys::DDWAF_LOG_INFO => Ok(Level::Info),
            libddwaf_sys::DDWAF_LOG_WARN => Ok(Level::Warn),
            libddwaf_sys::DDWAF_LOG_ERROR => Ok(Level::Error),
            libddwaf_sys::DDWAF_LOG_OFF => Ok(Level::Off),
            unknown => Err(UnknownLogLevelError { raw: unknown }),
        }
    }
}

/// An error that is produced when encountering an unknown log level value.
#[derive(Debug)]
pub struct UnknownLogLevelError {
    raw: libddwaf_sys::DDWAF_LOG_LEVEL,
}
impl fmt::Display for UnknownLogLevelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown log level: 0x{:02X}", self.raw)
    }
}
impl error::Error for UnknownLogLevelError {}

/// Wraps the log callback function (stored in [`LOG_CB`]) to convert the raw pointers provided by the C/C++ library into
/// somewhat easier to consume types.
extern "C" fn bridge_log_cb(
    level: libddwaf_sys::DDWAF_LOG_LEVEL,
    file: *const std::os::raw::c_char,
    function: *const std::os::raw::c_char,
    line: u32,
    message: *const std::os::raw::c_char,
    message_len: u64,
) {
    unsafe {
        #[allow(static_mut_refs)]
        if let Some(cb) = &LOG_CB {
            let file = CStr::from_ptr(file);
            let function = CStr::from_ptr(function);
            let message =
                slice::from_raw_parts(message.cast(), message_len.try_into().unwrap_or(usize::MAX));
            cb(
                Level::try_from(level).unwrap_or(Level::Error),
                file,
                function,
                line,
                message,
            );
        }
    }
}
