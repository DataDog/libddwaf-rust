# libddwaf-rust

This project's goal is to produce a higher level API for the Rust bindings to libddwaf: DataDog in-app WAF. It consists of 2 separate entities: the bindings for the calls to libddwaf, and the encoder which job is to convert any go value to its libddwaf object representation.

## Warning

This library is still in **preview**

## Build Dependencies

The build script for `libddwaf-sys` downloads C library releases from [DataDog/libddwaf][c-libddwaf].

[c-libddwaf]: https://github.com/DataDog/libddwaf/releases

### C standard library
In order to allow for FIPS-compliant builds, the release download uses the [`aws-lc-rs`][aws-lc-rs] crate as the
cryptographic provider for TLS, and this requires headers for the C standard library to be available.
- On `debian`-based platforms, this is provided by `apt install -y build-essential`
- On `alpine`-based platforms, this is provided by `apk add musl-dev`

[aws-lc-rs]: https://crates.io/crates/aws-lc-rs

### Clang
The `libddwaf-sys` crate uses [`bindgen`][bindgen], which requires `libclang.so` to be available.
- On `debian`-based platforms, this is provided by `apt install -y libclang-dev`
- On `alpine`-based platforms, this is provided by `apk add clang-libclang`


[bindgen]: https://crates.io/crates/bindgen


## Crate Features
### `serde`
The `serde` feature (enabled by default) provides `serde` implementations for `libddwaf::objects::*` types.

### `dynamic`
The `dynamic` feature (disabled by default) causes the native `libddwaf` library to be loaded at run-time using
`libloading` instead of being statically linked into the `libddwaf-sys` crate. Enabling the `dynamic` feature can be
useful to reduce the stat-up overhead in case the `libddwaf` features are not always used (such as if the security
features are opt-in); but it increases the size of the final binary & decreases overall performance of using those
functions (due to their being dynamically dispatched).
