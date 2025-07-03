#![warn(
    clippy::correctness,
    clippy::pedantic,
    clippy::perf,
    clippy::style,
    clippy::suspicious
)]
#![cfg(feature = "serde_test")]
#![cfg(not(miri))]

use std::{
    ffi::CStr,
    sync::{Arc, Mutex, Once},
    time::Duration,
};

use libddwaf::object::WAFMap;
use libddwaf::Config;
use libddwaf::{
    log::LogLevel,
    object::{WAFArray, WAFObject, WAFOwned},
    Builder, RunResult,
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
    level: LogLevel,
    function: &'static CStr,
    file: &'static CStr,
    line: u32,
    message: &[u8],
) {
    let msg_str = String::from_utf8_lossy(message);
    eprintln!(
        "[{:-5}] {}:{} ({}) {}",
        level,
        file.to_string_lossy(),
        line,
        function.to_string_lossy(),
        msg_str,
    );
}

static INIT: Once = Once::new();
fn init() {
    INIT.call_once(|| {
        unsafe { libddwaf::log::set_log_cb(Some(log_callback), LogLevel::Debug) };
    });
}

#[test]
fn basic_run_rule() {
    init();

    let ruleset: WAFObject = serde_json::from_str(ARACHNI_RULE).unwrap();
    let mut builder = Builder::new(&Config::default()).expect("Failed to create builder");
    let mut diagnostics = WAFOwned::<WAFMap>::default();
    assert!(builder.add_or_update_config("rules", &ruleset, Some(&mut diagnostics)));

    assert!(diagnostics.is_valid());
    let loaded_rule_name = diagnostics
        .get_str("rules")
        .unwrap()
        .as_type::<WAFMap>()
        .unwrap()
        .get_str("loaded")
        .unwrap()
        .as_type::<WAFArray>()
        .unwrap()[0]
        .to_str()
        .unwrap();
    assert_eq!(loaded_rule_name, "arachni_rule");

    let waf = builder.build().unwrap();
    let mut ctx = waf.new_context();

    let mut header = WAFMap::new(1);
    header[0] = ("user-agent", "Arachni").into();
    let mut data = WAFMap::new(1);
    data[0] = (
        "server.request.headers.no_cookies",
        Into::<WAFObject>::into(header),
    )
        .into();

    let res = ctx.run(Some(data), None, Duration::from_secs(1));

    match res {
        Ok(RunResult::Match(result)) => {
            assert!(!result.timeout());
            assert!(result.keep());
            assert!(result.duration() > Duration::default());

            let events = result.events().expect("Expected some events");
            assert_eq!(events.len(), 1);
            let first_event: &WAFMap = events[0].as_type().unwrap();
            let rule_first_event: &WAFMap = first_event.get_str("rule").unwrap().as_type().unwrap();
            assert_eq!(
                rule_first_event.get_str("id").unwrap().to_str().unwrap(),
                "arachni_rule"
            );
        }
        _ => {
            panic!("Unexpected result: {res:?}");
        }
    }
}

#[test]
fn test_known_actions() {
    let ruleset: WAFObject = serde_json::from_str(ARACHNI_RULE).unwrap();
    let mut builder = Builder::new(&Config::default()).expect("Failed to create builder");
    assert!(builder.add_or_update_config("rules", &ruleset, None));
    let waf = Arc::new(builder.build().unwrap());

    let actions = waf.known_actions();
    assert!(!actions.is_empty());
    assert_eq!(actions[0].to_str().unwrap(), "block_request");
}

#[test]
fn run_rule_threaded() {
    let ruleset: WAFObject = serde_json::from_str(ARACHNI_RULE).unwrap();
    let mut builder = Builder::new(&Config::default()).expect("Failed to create builder");
    assert!(builder.add_or_update_config("rules", &ruleset, None));
    let waf = Arc::new(builder.build().unwrap());

    let mut header = WAFMap::new(1);
    header[0] = ("user-agent", "Arachni").into();
    let mut data = WAFMap::new(1);
    data[0] = (
        "server.request.headers.no_cookies",
        Into::<WAFObject>::into(header),
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
