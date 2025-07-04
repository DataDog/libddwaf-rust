use libddwaf::{
    object::{WAFMap, WAFOwned},
    waf_array, waf_map, Builder, Config,
};

#[test]
pub fn blank_config() {
    let builder = Builder::new(&Config::default()).expect("builder should be created");
    // Not adding any rules, so we can't get a handle...
    assert!(builder.build().is_none());
}

#[test]
#[cfg_attr(
    debug_assertions,
    should_panic(
        expected = "path cannot be empty (bindings::ddwaf_builder_add_or_update_config would always fail)"
    )
)]
pub fn empty_path() {
    let mut builder = Builder::new(&Config::default()).expect("builder should be created");
    assert!(!builder.add_or_update_config("", &waf_map! {}, None)); // Panics when debug_assertions is enabled
}

#[test]
pub fn add_update_remove_config() {
    let mut builder = Builder::new(&Config::default()).expect("builder should be created");

    let rules_1 = waf_map! {
        ("version", "2.1"),
        ("metadata", waf_map!{
            ("rules_version", "1"),
        }),
        ("rules", waf_array![
            waf_map!{
                ("id", "1"),
                ("name", "rule 1"),
                ("tags", waf_map!{ ("type", "flow1"), ("category", "test") }),
                ("conditions", waf_array![
                    waf_map!{
                        ("operator", "match_regex"),
                        ("parameters", waf_map!{
                            ("inputs", waf_array![
                                waf_map!{("address", "address.1")},
                            ]),
                            ("regex", ".*"),
                        }),
                    },
                ]),
                ("on_match", waf_array!["block"]),
            },
        ]),
    };
    let rules_2 = waf_map! {
        ("version", "2.1"),
        ("metadata", waf_map!{
            ("rules_version", "2"),
        }),
        ("rules", waf_array![
            waf_map!{
                ("id", "1"),
                ("name", "rule 1"),
                ("tags", waf_map!{ ("type", "flow1"), ("category", "test") }),
                ("conditions", waf_array![
                    waf_map!{
                        ("operator", "match_regex"),
                        ("parameters", waf_map!{
                            ("inputs", waf_array![
                                waf_map!{("address", "address.2")},
                            ]),
                            ("regex", ".*"),
                        }),
                    },
                ]),
                ("on_match", waf_array!["block"]),
            },
        ]),
    };

    assert_eq!(builder.config_paths_count(None), 0);

    let mut diagnostics = WAFOwned::<WAFMap>::default();
    assert!(builder.add_or_update_config("test", &rules_1, Some(&mut diagnostics)));
    assert!(diagnostics.is_valid());
    assert_eq!(
        diagnostics.get(b"ruleset_version").and_then(|o| o.to_str()),
        Some("1"),
    );
    assert_eq!(builder.config_paths_count(None), 1);
    for path in builder.config_paths(None).iter() {
        assert_eq!(path.to_str(), Some("test"));
    }

    assert!(builder.add_or_update_config("test", &rules_2, Some(&mut diagnostics)));
    assert!(diagnostics.is_valid());
    assert_eq!(
        diagnostics.get(b"ruleset_version").and_then(|o| o.to_str()),
        Some("2"),
    );
    assert_eq!(builder.config_paths_count(None), 1);
    for path in builder.config_paths(None).iter() {
        assert_eq!(path.to_str(), Some("test"));
    }

    assert!(builder.remove_config("test"));
    assert_eq!(builder.config_paths_count(None), 0);
    assert!(builder.config_paths(None).is_empty());
}
