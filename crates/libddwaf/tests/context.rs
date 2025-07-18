#![warn(
    clippy::correctness,
    clippy::pedantic,
    clippy::perf,
    clippy::style,
    clippy::suspicious
)]

use std::sync::LazyLock;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use libddwaf::{
    object::{WafArray, WafMap, WafObject, WafOwned},
    waf_array, waf_map, Builder, Config, RunResult,
};

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
fn basic_run_rule_with_match() {
    let mut builder = Builder::new(&Config::default()).expect("Failed to create builder");
    let mut diagnostics = WafOwned::<WafMap>::default();
    assert!(builder.add_or_update_config(
        "rules",
        LazyLock::force(&ARACHNI_RULE),
        Some(&mut diagnostics)
    ));

    assert!(diagnostics.is_valid());
    let loaded_rule_name = diagnostics
        .get_str("rules")
        .unwrap()
        .as_type::<WafMap>()
        .unwrap()
        .get_str("loaded")
        .unwrap()
        .as_type::<WafArray>()
        .unwrap()[0]
        .to_str()
        .unwrap();
    assert_eq!(loaded_rule_name, "arachni_rule");

    let waf = builder.build().unwrap();
    let mut ctx = waf.new_context();

    let mut header = WafMap::new(1);
    header[0] = ("user-agent", "Arachni").into();
    let mut data = WafMap::new(1);
    data[0] = ("server.request.headers.no_cookies", header).into();

    let res = ctx.run(Some(data), None, Duration::from_secs(1));

    match res {
        Ok(RunResult::Match(result)) => {
            assert!(!result.timeout());
            assert!(result.keep());
            assert!(result.duration() > Duration::default());

            let events = result.events().expect("Expected some events");
            assert_eq!(events.len(), 1);
            let first_event: &WafMap = events[0].as_type().unwrap();
            let rule_first_event: &WafMap = first_event.get_str("rule").unwrap().as_type().unwrap();
            assert_eq!(
                rule_first_event.get_str("id").unwrap().to_str().unwrap(),
                "arachni_rule"
            );

            let actions = result.actions().expect("Expected some actions");
            assert_eq!(actions.len(), 1);
            assert!(actions.get(b"block_request").is_some(),);
        }
        _ => {
            panic!("Unexpected result: {res:?}");
        }
    }
}

#[test]
fn basic_run_rule_with_no_match() {
    let mut builder = Builder::new(&Config::default()).expect("Failed to create builder");
    let mut diagnostics = WafOwned::<WafMap>::default();
    assert!(builder.add_or_update_config(
        "rules",
        LazyLock::force(&ARACHNI_RULE),
        Some(&mut diagnostics)
    ));

    assert!(diagnostics.is_valid());
    let loaded_rule_name = diagnostics
        .get_str("rules")
        .unwrap()
        .as_type::<WafMap>()
        .unwrap()
        .get_str("loaded")
        .unwrap()
        .as_type::<WafArray>()
        .unwrap()[0]
        .to_str()
        .unwrap();
    assert_eq!(loaded_rule_name, "arachni_rule");

    let waf = builder.build().unwrap();
    let mut ctx = waf.new_context();

    let mut header = WafMap::new(1);
    header[0] = ("user-agent", "JDatabaseDriverMysqli").into();
    let mut data = WafMap::new(1);
    data[0] = ("server.request.headers.no_cookies", header).into();

    let res = ctx.run(Some(data), None, Duration::from_secs(1));

    match res {
        Ok(RunResult::NoMatch(result)) => {
            assert!(!result.timeout());
            assert!(!result.keep());
            assert!(result.duration() > Duration::default());

            if let Some(events) = result.events() {
                assert!(events.is_empty());
            }
            if let Some(actions) = result.actions() {
                assert!(actions.is_empty());
            }
            if let Some(attributes) = result.attributes() {
                assert!(attributes.is_empty());
            }
        }
        _ => {
            panic!("Unexpected result: {res:?}");
        }
    }
}

#[test]
fn run_rule_threaded() {
    let mut builder = Builder::new(&Config::default()).expect("Failed to create builder");
    assert!(builder.add_or_update_config("rules", LazyLock::force(&ARACHNI_RULE), None));
    let waf = Arc::new(builder.build().unwrap());

    let mut header = WafMap::new(1);
    header[0] = ("user-agent", "Arachni").into();
    let mut data = WafMap::new(1);
    data[0] = (
        "server.request.headers.no_cookies",
        Into::<WafObject>::into(header),
    )
        .into();
    let adata = Arc::new(data);

    let t: Vec<_> = (0..2)
        .map(|_| {
            let data = adata.clone();
            let waf = waf.clone();
            std::thread::spawn(move || {
                let ctx = Arc::new(Mutex::new(waf.new_context()));

                (0..2)
                    .map(|_| {
                        let data = data.clone();
                        let ctx = Arc::clone(&ctx);
                        std::thread::spawn(move || {
                            let mut ctx = ctx.lock().unwrap();

                            let res = ctx.run(None, Some(&data), Duration::from_secs(1));

                            assert!(matches!(res, Ok(RunResult::Match(_))));
                        })
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .for_each(|t| t.join().unwrap());
            })
        })
        .collect();

    t.into_iter().for_each(|t| t.join().unwrap());
}
