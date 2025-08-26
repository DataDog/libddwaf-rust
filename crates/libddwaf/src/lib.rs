#![deny(
    clippy::correctness,
    clippy::pedantic,
    clippy::perf,
    clippy::style,
    clippy::suspicious
)]
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

//! Rust bindings for the [`libddwaf` library](https://github.com/DataDog/libddwaf).
//!
//! # Basic Use
//!
//! The following high-level steps are typically used:
//! 1. Create a new [Builder]
//! 2. Add new configurations to it using [`Builder::add_or_update_config`]
//!     * Rulesets are often parsed from JSON documents using `serde_json`, via
//!       the `serde` feature.
//! 3. Call [`Builder::build`] to obtain a new [`Handle`]
//! 4. For any service request:
//!     1. Call [`Handle::new_context`] to obtain a new [`Context`]
//!     2. Call [`Context::run`] as appropriate with the necessary address data
//!
//! ```rust
//! use libddwaf::{
//!     object::*,
//!     waf_array,
//!     waf_map,
//!     Builder,
//!     Config,
//!     RunResult,
//! };
//!
//! let mut builder = Builder::new(&Config::default())
//!     .expect("Failed to build WAF instance");
//! let rule_set = waf_map!{
//!     /* Typically obtained by parsing a rules file using the serde feature */
//!     ("rules", waf_array!{ waf_map!{
//!         ("id", "1"),
//!         ("name", "rule 1"),
//!         ("tags", waf_map!{ ("type", "flow1"), ("category", "test") }),
//!         ("conditions", waf_array!{ waf_map!{
//!             ("operator", "match_regex"),
//!             ("parameters", waf_map!{
//!                 ("regex", ".*"),
//!                 ("inputs", waf_array!{ waf_map!{ ("address", "arg1" )} }),
//!             }),
//!         } }),
//!         ("on_match", waf_array!{ "block" })
//!     } }),
//! };
//! let mut diagnostics = WafOwned::<WafMap>::default();
//! if !builder.add_or_update_config("config/file/logical/path", &rule_set, Some(&mut diagnostics)) {
//!     panic!("Failed to add or update config!");
//! }
//! let waf = builder.build().expect("Failed to build WAF instance");
//!
//! // For each new request to be monitored...
//! let mut waf_ctx = waf.new_context();
//! let data = waf_map!{
//!     ("arg1", "value1"),
//! };
//! match waf_ctx.run(Some(data), None, std::time::Duration::from_millis(1)) {
//!     // Deal with the result as appropriate...
//!     Ok(RunResult::Match(res)) => {
//!         assert!(!res.timeout());
//!         assert!(res.keep());
//!         assert!(res.duration() >= std::time::Duration::default());
//!         assert_eq!(res.events().expect("Expected events").len(), 1);
//!         assert_eq!(res.actions().expect("Expected actions").len(), 1);
//!         assert_eq!(res.attributes().expect("Expected attributes").len(), 0);
//!     },
//!     Err(e) => panic!("Error while running the in-app WAF: {e}"),
//!     _ => panic!("Unexpected result"),
//! }
//! ```

use std::ffi::CStr;

#[cfg(feature = "serde")]
pub mod serde;

pub mod log;
pub mod object;
mod private;

macro_rules! forward {
    ($($name:ident),*) => {
        $(
            mod $name;
            #[doc(inline = true)]
            pub use $name::*;
        )*
    };
}

forward!(builder, config, context, handle);

/// Returns the version of the underlying `libddwaf` library.
#[must_use]
pub fn get_version() -> &'static CStr {
    let ptr = unsafe { libddwaf_sys::ddwaf_get_version() };
    if ptr.is_null() {
        unsafe { CStr::from_ptr("\0".as_ptr().cast()) }
    } else {
        unsafe { CStr::from_ptr(ptr) }
    }
}

#[cfg(test)]
mod tests {
    use crate::get_version;

    #[test]
    fn test_get_version() {
        assert_eq!(
            env!("CARGO_PKG_VERSION"),
            get_version()
                .to_str()
                .expect("Failed to convert version to str")
        );
    }
}
