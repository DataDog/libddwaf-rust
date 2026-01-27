#[cfg(not(miri))]
use std::time::Duration;

use libddwaf::Obfuscator;

#[cfg(not(miri))]
use libddwaf::{
    Config, RunResult, RunnableContext,
    object::{WafArray, WafMap},
};

#[cfg(not(miri))]
use crate::common::PASSWORD_RULE;

mod common;

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
        let def = Obfuscator::new(Some("a"), Some("b"));
        def.clone()
    };
    assert_eq!(
        obfuscator
            .key_regex()
            .map(|r| unsafe { std::str::from_utf8_unchecked(r) }),
        Some("a")
    );
    assert_eq!(
        obfuscator
            .value_regex()
            .map(|r| unsafe { std::str::from_utf8_unchecked(r) }),
        Some("b")
    );
}

#[cfg(not(miri))]
fn get_match_value(rr: &RunResult) -> &str {
    match rr {
        RunResult::Match(output) => {
            let events = output.events().unwrap();

            (events[0]
                .as_type::<WafMap>()
                .unwrap()
                .get_str("rule_matches")
                .unwrap()
                .as_type::<WafArray>()
                .unwrap()[0]
                .as_type::<WafMap>()
                .unwrap()
                .get_str("parameters")
                .unwrap()
                .as_type::<WafArray>()
                .unwrap()[0]
                .as_type::<WafMap>()
                .unwrap()
                .get_str("value")
                .unwrap()
                .to_str()
                .unwrap()) as _
        }
        _ => {
            panic!("Unexpected result: {rr:?}");
        }
    }
}

#[test]
#[cfg(not(miri))]
fn default_uses_default_obfuscator() {
    use libddwaf::{Builder, waf_map};

    let mut builder = Builder::new(Some(&Config::default())).expect("Failed to create builder");
    assert!(builder.add_or_update_config("rules", &*PASSWORD_RULE, None));
    let waf = builder.build().unwrap();

    let data = waf_map! {
        ("server.request.query",
        waf_map! {
            ("password", "foobar"),
        }),
    };

    let mut ctx = waf.new_context();
    let res = ctx.run(data, Duration::from_secs(1)).unwrap();

    let match_value = get_match_value(&res);
    assert_eq!(match_value, "<Redacted>");
}

#[test]
#[cfg(not(miri))]
fn none_uses_no_obfuscator() {
    use libddwaf::{Builder, waf_map};

    let mut builder = Builder::new(None).expect("Failed to create builder");
    assert!(builder.add_or_update_config("rules", &*PASSWORD_RULE, None));
    let waf = builder.build().unwrap();

    let data = waf_map! {
        ("server.request.query",
        waf_map! {
            ("password", "foobar"),
        }),
    };

    let mut ctx = waf.new_context();
    let res = ctx.run(data, Duration::from_secs(1)).unwrap();

    let match_value = get_match_value(&res);
    assert_eq!(match_value, "foobar");
}

#[test]
#[cfg(not(miri))]
fn empty_values_use_effectively_no_obfuscator() {
    use libddwaf::{Builder, waf_map};

    let obfuscator = Obfuscator::new(Some(""), Some(""));
    let mut builder =
        Builder::new(Some(&Config::new(obfuscator))).expect("Failed to create builder");
    assert!(builder.add_or_update_config("rules", &*PASSWORD_RULE, None));
    let waf = builder.build().unwrap();

    let data = waf_map! {
        ("server.request.query",
        waf_map! {
            ("password", "foobar"),
        }),
    };

    let mut ctx = waf.new_context();
    let res = ctx.run(data, Duration::from_secs(1)).unwrap();

    let match_value = get_match_value(&res);
    assert_eq!(match_value, "foobar");
}

#[test]
#[cfg(not(miri))]
pub fn with_an_actual_obfuscator() {
    use libddwaf::{Builder, waf_map};

    let obfuscator = Obfuscator::new(Some(""), Some("bar"));
    let mut builder =
        Builder::new(Some(&Config::new(obfuscator))).expect("Failed to create builder");
    assert!(builder.add_or_update_config(
        "rules",
        std::sync::LazyLock::force(&PASSWORD_RULE),
        None
    ));
    let waf = builder.build().unwrap();

    let data = waf_map! {
        ("server.request.query",
        waf_map! {
            ("p", "foobar"),
        }),
    };

    let mut ctx = waf.new_context();
    let res = ctx.run(data, Duration::from_secs(1)).unwrap();

    let match_value = get_match_value(&res);
    assert_eq!(match_value, "<Redacted>");

    let data = waf_map! {
        ("server.request.query",
        waf_map! {
            ("p", "foobaz"),
        }),
    };

    let mut ctx = waf.new_context();
    let res = ctx.run(data, Duration::from_secs(1)).unwrap();

    let match_value = get_match_value(&res);
    assert_eq!(match_value, "foobaz");
}
