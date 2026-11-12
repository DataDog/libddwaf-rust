# libddwaf

Rust bindings to [libddwaf](https://github.com/DataDog/libddwaf), Datadog's
in-app WAF (Web Application Firewall).

This crate provides a safe, high-level API on top of the raw bindings in
[`libddwaf-sys`](https://crates.io/crates/libddwaf-sys): build a WAF instance
from one or more rulesets, then create a context per request and run it against
the request's data.

```rust
use libddwaf::{waf_array, waf_map, Builder, Config, RunResult, RunnableContext};

// Rulesets are typically parsed from JSON, using the `serde` feature.
let ruleset = waf_map! {
    ("rules", waf_array! { waf_map! {
        ("id", "1"),
        ("name", "rule 1"),
        ("tags", waf_map! { ("type", "flow1"), ("category", "test") }),
        ("conditions", waf_array! { waf_map! {
            ("operator", "match_regex"),
            ("parameters", waf_map! {
                ("regex", ".*"),
                ("inputs", waf_array! { waf_map! { ("address", "arg1") } }),
            }),
        } }),
    } }),
};

let mut builder = Builder::new(Some(&Config::default())).expect("failed to create builder");
assert!(builder.add_or_update_config("rules", &ruleset, None));
let waf = builder.build().expect("failed to build WAF instance");

// For each request:
let mut context = waf.new_context();
let data = waf_map! { ("arg1", "value1") };
match context.run(data, std::time::Duration::from_millis(1)) {
    Ok(RunResult::Match(result)) => println!("matched: {:?}", result.events()),
    Ok(RunResult::NoMatch(_)) => println!("no match"),
    Err(error) => eprintln!("WAF error: {error}"),
}
```

See the [API documentation](https://docs.rs/libddwaf) for details.

By default, the build downloads a prebuilt libddwaf release; it can also link a
local installation or build libddwaf from source. See the
[repository README](https://github.com/DataDog/libddwaf-rust#readme) for the
build requirements and the available features.

## License

Apache-2.0
