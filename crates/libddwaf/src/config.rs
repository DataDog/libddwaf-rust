use crate::object::WafMap;
use crate::waf_map;

/// The configuration for a new [`Builder`](crate::Builder).
#[derive(Clone, Default, Debug)]
pub struct Config {
    obfuscator: Obfuscator,
}
impl Config {
    /// Creates a new [`Config`] with the provided [`Obfuscator`].
    #[must_use]
    pub fn new(obfuscator: Obfuscator) -> Self {
        Self { obfuscator }
    }

    #[must_use]
    pub fn as_waf_object(&self) -> WafMap {
        let mut map = WafMap::new(2);
        let mut used: u16 = 0;
        if let Some(key_regex) = self.obfuscator.key_regex() {
            map[used as usize] = ("key_regex", key_regex).into();
            used += 1;
        }
        if let Some(value_regex) = self.obfuscator.value_regex() {
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
    pub fn key_regex(&self) -> Option<&[u8]> {
        self.key_regex.as_deref()
    }

    /// Returns the regular expression used to determine value data to be obfuscated, if one has
    /// been set.
    #[must_use]
    pub fn value_regex(&self) -> Option<&[u8]> {
        self.value_regex.as_deref()
    }
}

impl Default for Obfuscator {
    fn default() -> Self {
        // This actually uses the default regexes from libddwaf
        Obfuscator::new(None::<&str>, None::<&str>)
    }
}
