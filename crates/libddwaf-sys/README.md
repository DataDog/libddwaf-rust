# libddwaf-sys

Low-level, `bindgen`-generated bindings to the C API of
[libddwaf](https://github.com/DataDog/libddwaf), Datadog's in-app WAF (Web
Application Firewall).

Most users should depend on [`libddwaf`](https://crates.io/crates/libddwaf)
instead, which provides a safe, high-level API on top of these bindings.

By default, the build script downloads the libddwaf release matching this
crate's version and links it statically. It can instead load or link a shared
library, use a local installation (`LIBDDWAF_PREFIX`), or build libddwaf from
source. See the
[repository README](https://github.com/DataDog/libddwaf-rust#readme) for the
build requirements and the available features.

## License

Apache-2.0
