use libddwaf::{OBFUSCATOR_DEFAULT_KEY_REGEX, OBFUSCATOR_DEFAULT_VAL_REGEX, Obfuscator};

#[test]
pub fn default_obfuscator() {
    let obfuscator = Obfuscator::default();
    assert!(obfuscator.key_regex().is_some());
    assert!(obfuscator.value_regex().is_some());
}

#[test]
pub fn key_only_obfuscator() {
    let obfuscator = Obfuscator::new(Some(".*"), Option::<&str>::None);
    assert_eq!(
        obfuscator
            .key_regex()
            .map(|r| unsafe { std::str::from_utf8_unchecked(r) }),
        Some(".*")
    );
    assert!(obfuscator.value_regex().is_none());
}

#[test]
pub fn value_only_obfuscator() {
    let obfuscator = Obfuscator::new(Option::<&str>::None, Some(".*"));
    assert!(obfuscator.key_regex().is_none());
    assert_eq!(
        obfuscator
            .value_regex()
            .map(|r| unsafe { std::str::from_utf8_unchecked(r) }),
        Some(".*")
    );
}

#[test]
pub fn clone_validity() {
    let obfuscator = {
        // Clone from this and let it get dropped.
        let def = Obfuscator::default();
        def.clone()
    };
    assert_eq!(
        obfuscator
            .key_regex()
            .map(|r| unsafe { std::str::from_utf8_unchecked(r) }),
        Some(OBFUSCATOR_DEFAULT_KEY_REGEX)
    );
    assert_eq!(
        obfuscator
            .value_regex()
            .map(|r| unsafe { std::str::from_utf8_unchecked(r) }),
        Some(OBFUSCATOR_DEFAULT_VAL_REGEX)
    );
}
