#![cfg(feature = "serde_test")]
#![cfg(not(miri))]

use std::{
    ffi::CStr,
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc, Mutex, Once,
    },
    thread::{self, sleep},
    time::Duration,
};

use libddwaf::{
    ddwaf_obj_map, CommonDdwafObj, DdwafConfig, DdwafLogLevel, DdwafObj, DdwafObjArray,
    DdwafObjMap, DdwafObjType, UpdateableWafInstance, WafInstance, WafOwnedDdwafObj, WafRunResult,
};

const ARACHNI_RULE: &str = r#"
{
   "rules" : [
      {
         "conditions" : [
            {
               "operator" : "match_regex",
               "parameters" : {
                  "inputs" : [
                     {
                        "address" : "server.request.headers.no_cookies",
                        "key_path" : [
                           "user-agent"
                        ]
                     },
                     {
                        "address" : "server.request.body"
                     }
                  ],
                  "regex" : "Arachni"
               }
            }
         ],
         "id" : "arachni_rule",
         "name" : "Block with default action",
         "on_match" : [
            "block"
         ],
         "tags" : {
            "category" : "attack_attempt",
            "type" : "security_scanner"
         }
      }
   ],
   "version" : "2.1"
}
"#;

fn log_callback(
    level: DdwafLogLevel,
    function: &'static CStr,
    file: &'static CStr,
    line: u32,
    message: &[std::os::raw::c_char],
) {
    let msg_str = String::from_utf8_lossy(unsafe { &*(message as *const [i8] as *const [u8]) });
    println!(
        "[{:?}] {} at {} on {}:{}",
        level,
        msg_str,
        function.to_string_lossy(),
        file.to_string_lossy(),
        line,
    );
}

static INIT: Once = Once::new();
fn init() {
    INIT.call_once(|| {
        unsafe { libddwaf::set_log_cb(Some(log_callback), DdwafLogLevel::Debug) };
    })
}

#[test]
fn basic_run_rule() {
    init();

    let ruleset: DdwafObj = serde_json::from_str(ARACHNI_RULE).unwrap();

    let mut diagnostics = WafOwnedDdwafObj::default();
    let waf = WafInstance::new(&ruleset, DdwafConfig::default(), Some(&mut diagnostics)).unwrap();

    assert_eq!(diagnostics.get_type(), DdwafObjType::Map);
    let loaded_rule_name = diagnostics
        .as_type::<DdwafObjMap>()
        .unwrap()
        .get_str("rules")
        .unwrap()
        .as_type::<DdwafObjMap>()
        .unwrap()
        .get_str("loaded")
        .unwrap()
        .as_type::<DdwafObjArray>()
        .unwrap()[0]
        .to_str()
        .unwrap();
    assert_eq!(loaded_rule_name, "arachni_rule");

    let mut ctx = waf.create_context();

    let mut header = DdwafObjMap::new(1);
    header[0] = ("user-agent", "Arachni").into();
    let mut data = DdwafObjMap::new(1);
    data[0] = (
        "server.request.headers.no_cookies",
        Into::<DdwafObj>::into(header),
    )
        .into();

    let res = ctx.run(Some(data), None, Duration::from_secs(1));

    match res {
        WafRunResult::Match(result) => {
            assert!(!result.is_timeout());

            let events = result.events();
            assert_eq!(events.len(), 1);
            let first_event: &DdwafObjMap = events[0].as_type().unwrap();
            let rule_first_event: &DdwafObjMap =
                first_event.get_str("rule").unwrap().as_type().unwrap();
            assert_eq!(
                rule_first_event.get_str("id").unwrap().to_str().unwrap(),
                "arachni_rule"
            );
        }
        _ => {
            panic!("Unexpected result");
        }
    }
}

#[test]
fn test_known_actions() {
    let ruleset: DdwafObj = serde_json::from_str(ARACHNI_RULE).unwrap();
    let mut waf = WafInstance::new(&ruleset, DdwafConfig::default(), None).unwrap();

    let actions = waf.known_actions();
    assert!(!actions.is_empty());
    assert_eq!(actions[0].to_str().unwrap(), "block_request");
}

#[test]
fn run_rule_threaded() {
    let ruleset: DdwafObj = serde_json::from_str(ARACHNI_RULE).unwrap();
    let waf = Arc::new(WafInstance::new(&ruleset, DdwafConfig::default(), None).unwrap());

    let mut header = DdwafObjMap::new(1);
    header[0] = ("user-agent", "Arachni").into();
    let mut data = DdwafObjMap::new(1);
    data[0] = (
        "server.request.headers.no_cookies",
        Into::<DdwafObj>::into(header),
    )
        .into();
    let adata = Arc::new(data);

    let t: Vec<_> = (0..2)
        .map(|_| {
            let data = adata.clone();
            let waf = waf.clone();
            std::thread::spawn(move || {
                let ctx = Arc::new(Mutex::new(waf.create_context()));

                (0..2)
                    .map(|_| {
                        let data = data.clone();
                        let ctx = Arc::clone(&ctx);
                        std::thread::spawn(move || {
                            let mut ctx = ctx.lock().unwrap();

                            let res = ctx.run(None, Some(&data), Duration::from_secs(1));

                            assert!(matches!(res, WafRunResult::Match(_)));
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

const DISABLE_ARACHNI_RULE_PATH: &[u8] = b"disable_arachni";
const DISABLE_ARACHNI_RULE: &str = r#"
{
    "rules_override": [
        {
            "rules_target": [
                {
                    "rule_id": "arachni_rule"
                }
            ],
            "enabled": false
        }
    ]
}
"#;

#[test]
fn threaded_updateable_waf_instance() {
    init();

    let ruleset: DdwafObj = serde_json::from_str(ARACHNI_RULE).unwrap();
    let upd_waf = UpdateableWafInstance::new(&ruleset, None, None).unwrap();

    // add a second rule because it's forbidden to have no rules
    let ruleset2: DdwafObj = serde_json::from_str(
        &ARACHNI_RULE
            .replace("Arachni", "Inhcara")
            .replace("arachni_rule", "inhcara_rule"),
    )
    .unwrap();
    upd_waf.add_or_update_config(b"2nd rule", &ruleset2, None);

    assert_eq!(upd_waf.count_config_paths(b"2nd rule"), 1);
    let paths = upd_waf.get_config_paths(b"2nd rule");
    let paths: &DdwafObjArray = paths.as_type().unwrap();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].to_str().unwrap(), "2nd rule");

    let update_thread = std::thread::spawn({
        let upd_waf_copy = upd_waf.clone();
        let disable_ruleset: DdwafObj = serde_json::from_str(DISABLE_ARACHNI_RULE).unwrap();
        move || {
            let mut disable_next = true;
            for _ in 0..10 {
                sleep(Duration::from_millis(100));
                if disable_next {
                    let res = upd_waf_copy.add_or_update_config(
                        DISABLE_ARACHNI_RULE_PATH,
                        &disable_ruleset,
                        None,
                    );
                    if !res {
                        panic!("add_or_update_config failed");
                    }
                    println!("disable");
                } else {
                    upd_waf_copy.remove_config(DISABLE_ARACHNI_RULE_PATH);
                    println!("enable");
                }
                upd_waf_copy.update().expect("update did not succeed");
                disable_next = !disable_next;
            }
        }
    });

    let data = Arc::new(ddwaf_obj_map!((
        "server.request.headers.no_cookies",
        ddwaf_obj_map!(("user-agent", "Arachni"))
    )));

    let stop_signal = &*Box::leak(Box::new(AtomicBool::new(false)));
    let t: Vec<_> = (0..2)
        .map(|_| {
            std::thread::spawn({
                let upd_waf_copy = upd_waf.clone();
                let data_copy = data.clone();
                let mut matches = 0u64;
                let mut non_matches = 0u64;
                move || {
                    while !stop_signal.load(Relaxed) {
                        let cur_instance = upd_waf_copy.current();
                        println!("address of instance: {:p}", Arc::as_ptr(&cur_instance));
                        let mut ctx = cur_instance.create_context();
                        let res = ctx.run(None, Some(&*data_copy), Duration::from_millis(500));
                        match res {
                            WafRunResult::Match(_) => {
                                matches += 1;
                            }
                            _ => non_matches += 1,
                        };
                        thread::sleep(Duration::from_millis(20))
                    }
                    (matches, non_matches)
                }
            })
        })
        .collect::<Vec<_>>();

    update_thread.join().unwrap();
    stop_signal.store(true, Relaxed);

    for jh in t {
        let (matches, non_matches) = jh.join().unwrap();
        println!("positive: {matches}, negative: {non_matches}");
        assert!(matches > 10);
        assert!(non_matches > 10);
    }
}
