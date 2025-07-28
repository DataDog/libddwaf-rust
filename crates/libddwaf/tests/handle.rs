use std::sync::LazyLock;

use libddwaf::object::WafMap;
use libddwaf::{Builder, Config, waf_array, waf_map};

static ARACHNI_RULE: LazyLock<WafMap> = LazyLock::new(|| {
    waf_map! {
        ("version", "2.1"),
        ("rules", waf_array![
            waf_map!{
                ("id", "arachni_rule"),
                ("name", "Block with default action"),
                ("tags", waf_map!{ ("category", "attack_attempt"), ("type", "security_scanner") }),
                ("conditions", waf_array![
                    waf_map!{
                        ("operator", "match_regex"),
                        ("parameters", waf_map!{
                            ("inputs", waf_array![
                                waf_map!{
                                    ("address", "server.request.headers.no_cookies"),
                                    ("key_path", waf_array!["user-agent"]),
                                },
                                waf_map!{
                                    ("address", "server.request.body"),
                                },
                            ]),
                            ("regex", "Arachni"),
                        }),
                    },
                ]),
                ("on_match", waf_array!["block"])
            },
        ]),
    }
});

#[test]
fn test_known_actions() {
    let mut builder = Builder::new(&Config::default()).expect("Failed to create builder");
    assert!(builder.add_or_update_config("rules", std::sync::LazyLock::force(&ARACHNI_RULE), None));
    let waf = builder.build().unwrap();

    let actions = waf.known_actions();
    assert!(!actions.is_empty());
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].to_str(), Ok("block_request"));
}

#[test]
fn test_known_addresses() {
    let mut builder = Builder::new(&Config::default()).expect("Failed to create builder");
    assert!(builder.add_or_update_config("rules", std::sync::LazyLock::force(&ARACHNI_RULE), None));
    let waf = builder.build().unwrap();

    let addresses = waf.known_addresses();
    assert!(!addresses.is_empty());
    assert_eq!(addresses.len(), 2);
    assert_eq!(addresses[0].to_str(), Ok("server.request.body"));
    assert_eq!(
        addresses[1].to_str(),
        Ok("server.request.headers.no_cookies")
    );
}
