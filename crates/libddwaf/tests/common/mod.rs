#![allow(dead_code)]

use std::sync::LazyLock;

use libddwaf::object::WafMap;
use libddwaf::{waf_array, waf_map};

pub static ARACHNI_RULE: LazyLock<WafMap> = LazyLock::new(|| {
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

pub static PASSWORD_RULE: LazyLock<WafMap> = LazyLock::new(|| {
    waf_map! {
        ("version", "2.1"),
        ("rules", waf_array![
            waf_map!{
                ("id", "password_rule"),
                ("name", "Block with default action"),
                ("tags", waf_map!{ ("category", "attack_attempt"), ("type", "security_scanner") }),
                ("conditions", waf_array![
                    waf_map!{
                        ("operator", "match_regex"),
                        ("parameters", waf_map!{
                            ("inputs", waf_array![
                                waf_map!{
                                    ("address", "server.request.query"),
                                    ("key_path", waf_array!["password"]),
                                },
                                waf_map!{
                                    ("address", "server.request.query"),
                                    ("key_path", waf_array!["p"]),
                                }
                            ]),
                            ("regex", ".*"),
                        }),
                    },
                ]),
                ("on_match", waf_array!["block"])
            },
        ]),
    }
});
