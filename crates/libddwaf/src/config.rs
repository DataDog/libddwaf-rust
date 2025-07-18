use std::ffi::{CStr, CString};
use std::ptr::null_mut;

/// The configuration for a new [Builder](crate::Builder).
#[derive(Clone)]
pub struct Config {
    pub(crate) raw: libddwaf_sys::ddwaf_config,
    _obfuscator: Obfuscator, // For keeping the memory alive
}
impl Config {
    /// Creates a new [Config] with the provided [Limits] and [Obfuscator].
    #[must_use]
    pub fn new(limits: Limits, obfuscator: Obfuscator) -> Self {
        Self {
            raw: libddwaf_sys::ddwaf_config {
                limits,
                obfuscator: obfuscator.raw,
                free_fn: None,
            },
            _obfuscator: obfuscator,
        }
    }
}
impl Default for Config {
    fn default() -> Self {
        Self::new(Limits::default(), Obfuscator::default())
    }
}

/// The limits attached to a [Config].
pub type Limits = libddwaf_sys::_ddwaf_config__ddwaf_config_limits;

/// Obfuscation configuration for the WAF.
///
/// This is effectively a pair of regular expressions that are respectively used
/// to determine which key and value data to obfuscate when producing WAF
/// outputs.
#[repr(transparent)]
pub struct Obfuscator {
    raw: libddwaf_sys::_ddwaf_config__ddwaf_config_obfuscator,
}
impl Obfuscator {
    /// Creates a new [Obfuscator] with the provided key and value regular
    /// expressions.
    ///
    /// # Panics
    /// Panics if the provided key or value cannot be turned into a [`CString`].
    pub fn new<T: Into<Vec<u8>>, U: Into<Vec<u8>>>(
        key_regex: Option<T>,
        value_regex: Option<U>,
    ) -> Self {
        let key_regex = key_regex.map_or(null_mut(), |s| {
            CString::new(s).expect("Invalid key regex").into_raw()
        });
        let value_regex = value_regex.map_or(null_mut(), |s| {
            CString::new(s).expect("Invalid value regex").into_raw()
        });
        Self {
            #[allow(clippy::used_underscore_items)]
            raw: libddwaf_sys::_ddwaf_config__ddwaf_config_obfuscator {
                key_regex,
                value_regex,
            },
        }
    }

    /// Returns the regular expression used to determine key data to be obfuscated, if one has been
    /// set.
    #[must_use]
    pub const fn key_regex(&self) -> Option<&CStr> {
        if self.raw.key_regex.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(self.raw.key_regex) })
        }
    }

    /// Returns the regular expression used to determine value data to be obfuscated, if one has
    /// been set.
    #[must_use]
    pub const fn value_regex(&self) -> Option<&CStr> {
        if self.raw.value_regex.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(self.raw.value_regex) })
        }
    }
}
impl Clone for Obfuscator {
    fn clone(&self) -> Self {
        Self::new(
            self.key_regex().map(CStr::to_bytes),
            self.value_regex().map(CStr::to_bytes),
        )
    }
}
/// The regular expression used by [`Obfuscator::default`] to determine which key data to obfuscate.
pub const OBFUSCATOR_DEFAULT_KEY_REGEX: &str = r"(?i)pass|pw(?:or)?d|secret|(?:api|private|public|access)[_-]?key|token|consumer[_-]?(?:id|key|secret)|sign(?:ed|ature)|bearer|authorization|jsessionid|phpsessid|asp\.net[_-]sessionid|sid|jwt";
/// The regular expression used by [`Obfuscator::default`] to determine which value data to obfuscate.
pub const OBFUSCATOR_DEFAULT_VAL_REGEX: &str = r#"(?i)(?:p(?:ass)?w(?:or)?d|pass(?:[_-]?phrase)?|secret(?:[_-]?key)?|(?:(?:api|private|public|access)[_-]?)key(?:[_-]?id)?|(?:(?:auth|access|id|refresh)[_-]?)?token|consumer[_-]?(?:id|key|secret)|sign(?:ed|ature)?|auth(?:entication|orization)?|jsessionid|phpsessid|asp\.net(?:[_-]|-)sessionid|sid|jwt)(?:\s*=([^;&]+)|"\s*:\s*("[^"]+"|\d+))|bearer\s+([a-z0-9\._\-]+)|token\s*:\s*([a-z0-9]{13})|gh[opsu]_([0-9a-zA-Z]{36})|ey[I-L][\w=-]+\.(ey[I-L][\w=-]+(?:\.[\w.+\/=-]+)?)|[\-]{5}BEGIN[a-z\s]+PRIVATE\sKEY[\-]{5}([^\-]+)[\-]{5}END[a-z\s]+PRIVATE\sKEY|ssh-rsa\s*([a-z0-9\/\.+]{100,})"#;
impl Default for Obfuscator {
    fn default() -> Self {
        Obfuscator::new(
            Some(OBFUSCATOR_DEFAULT_KEY_REGEX),
            Some(OBFUSCATOR_DEFAULT_VAL_REGEX),
        )
    }
}
impl Drop for Obfuscator {
    fn drop(&mut self) {
        if !self.raw.key_regex.is_null() {
            unsafe {
                drop(CString::from_raw(self.raw.key_regex.cast_mut()));
            }
        }
        if !self.raw.value_regex.is_null() {
            unsafe {
                drop(CString::from_raw(self.raw.value_regex.cast_mut()));
            }
        }
    }
}
