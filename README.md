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

### `static`
The `static` feature (enabled by default) causes the native `libddwaf` library to be statically linked into the
`libddwaf-sys` crate, which makes distribution easier. On Linux, is only supported on `x86_64` and `arm64` platforms at
the moment.

While this feature should generally be safe to use, it is possible for it to cause symbol resolution conflicts with
other native libraries. Should that be the case, disabling this feature should alleviate the problem.

> [!CAUTION]
> Disabling the `static` feature implies the correct version of `libddwaf.so` must be made available to the dynamic
> loader at run-time. This is automatically handled by commands such as `cargo test`, but needs to be manually fulfilled
> when deploying a binary produced by `cargo build`.
