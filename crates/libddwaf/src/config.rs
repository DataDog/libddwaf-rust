use crate::object::WafMap;
use crate::waf_map;

/// The configuration for a new [`Builder`](crate::Builder).
#[derive(Clone, Default, Debug)]
pub struct Config {
    obfuscator: Obfuscator,
}
impl Config {
    /// Creates a new [`Config`] with the provided [`Limits`] and [`Obfuscator`].
    #[must_use]
    pub fn new(obfuscator: Obfuscator) -> Self {
        Self { obfuscator }
    }

    #[must_use]
    pub fn as_waf_object(&self) -> WafMap {
        let mut map = WafMap::new(2);
        let mut used: u16 = 0;
        if let Some(key_regex) = self.obfuscator.key_regex() {
            let key_regex: &[u8] = key_regex.as_ref();
            map[used as usize] = ("key_regex", key_regex).into();
            used += 1;
        }
        if let Some(value_regex) = self.obfuscator.value_regex() {
            let value_regex: &[u8] = value_regex.as_ref();
            map[used as usize] = ("value_regex", value_regex).into();
            used += 1;
        }
        map.truncate(used);

        waf_map!(("obfuscator", map))
    }
}

/// Obfuscation configuration for the WAF.
///
/// This is effectively a pair of regular expressions that are respectively used
/// to determine which key and value data to obfuscate when producing WAF
/// outputs.
#[derive(Clone, Debug)]
pub struct Obfuscator {
    key_regex: Option<Vec<u8>>,
    value_regex: Option<Vec<u8>>,
}
impl Obfuscator {
    /// Creates a new [`Obfuscator`] with the provided key and value regular
    /// expressions.
    ///
    /// # Panics
    /// Panics if the provided key or value cannot be turned into a [`CString`].
    pub fn new<T: Into<Vec<u8>>, U: Into<Vec<u8>>>(
        key_regex: Option<T>,
        value_regex: Option<U>,
    ) -> Self {
        Self {
            key_regex: key_regex.map(Into::into),
            value_regex: value_regex.map(Into::into),
        }
    }

    /// Returns the regular expression used to determine key data to be obfuscated, if one has been
    /// set.
    #[must_use]
    pub const fn key_regex(&self) -> Option<&Vec<u8>> {
        self.key_regex.as_ref()
    }

    /// Returns the regular expression used to determine value data to be obfuscated, if one has
    /// been set.
    #[must_use]
    pub const fn value_regex(&self) -> Option<&Vec<u8>> {
        self.value_regex.as_ref()
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
