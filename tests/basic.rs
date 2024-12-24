#![cfg(feature = "serde_test")]
#![cfg(not(miri))]

use std::{
    ffi::CStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use libddwaf::{
    CommonDdwafObj, Config, DdwafLogLevel, DdwafObj, DdwafObjArray, DdwafObjMap, DdwafObjType,
    WafInstance, WafOwnedDdwafObj, WafRunResult,
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

#[test]
fn basic_run_rule() {
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
    unsafe { libddwaf::set_log_cb(Some(log_callback), DdwafLogLevel::Debug) };

    let ruleset: DdwafObj = serde_json::from_str(ARACHNI_RULE).unwrap();

    let mut diagnostics = WafOwnedDdwafObj::default();
    let waf = WafInstance::new(&ruleset, Config::default(), Some(&mut diagnostics)).unwrap();

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
    let mut waf = WafInstance::new(&ruleset, Config::default(), None).unwrap();

    let actions = waf.known_actions();
    assert!(actions.is_some());
    let actions = actions.unwrap();
    assert_eq!(actions[0].to_str().unwrap(), "block_request");
}

#[test]
fn run_rule_threaded() {
    let ruleset: DdwafObj = serde_json::from_str(ARACHNI_RULE).unwrap();
    let waf = Arc::new(WafInstance::new(&ruleset, Config::default(), None).unwrap());

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
