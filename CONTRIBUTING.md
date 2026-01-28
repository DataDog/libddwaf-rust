# Contributing to libddwaf-rust

## Build Dependencies

Before contributing, ensure you have the required dependencies installed. See the [README](README.md) for details on:
- C standard library headers (for `aws-lc-rs`)
- Clang/libclang (for `bindgen`)

## Makefile Targets

The project provides several Makefile targets for development:

### `make check`

Runs the complete quality assurance suite: tests, miri, clippy, and format
checking. This is the recommended target to run before submitting a pull
request.

### `make test`

Runs the full test suite:
```bash
cargo test --all-targets
cargo test --doc
```

### `make miri`

Runs tests under [Miri](https://github.com/rust-lang/miri), a tool for detecting
undefined behavior in Rust code. Uses the nightly toolchain.

Miri cannot interpret foreign function interface (FFI) calls, which means tests
that call into the native `libddwaf` C library cannot run under Miri.

Tests that use FFI must be excluded from Miri runs. There are two patterns used
in this codebase:

**1. File-level exclusion** - For test files that exclusively test FFI
functionality:
```rust
#![cfg(not(miri))]
```
    Place this at the top of the test file. This is used for integration tests
    in files like `tests/context.rs`, `tests/handle.rs`, `tests/config.rs`, etc.

**2. Function-level exclusion** - For individual tests within files that have a
mix of pure Rust and FFI tests:
```rust
#[test]
#[cfg(not(miri))]
fn test_that_uses_ffi() {
    // ...
}
```

Some tests are also excluded from Miri because they take too long to run under
the interpreter (Miri is significantly slower than native execution):
```rust
#[test]
#[cfg(not(miri))] // takes too long
fn test_large_array_operations() {
   // ...
}
```

### `make coverage`

Generates code coverage reports using `llvm-cov`. Requires the nightly
toolchain. Outputs:
- LCOV report to `target/lcov.info`
- HTML report to `target/coverage/`
- Fails if line coverage drops below 85%

### `make clippy`

Runs Clippy lints on all targets:
```bash
cargo clippy --all-targets
```

### `make format_check`

Checks code formatting without modifying files:
```bash
cargo fmt -- --check
```

To fix formatting issues, run `cargo fmt` directly.


## Using `LIBDDWAF_PREFIX`

By default, the build script downloads the `libddwaf` C library from GitHub
releases. You can override this behavior by setting the `LIBDDWAF_PREFIX`
environment variable to point to a local installation:

```bash
export LIBDDWAF_PREFIX=/path/to/libddwaf
cargo build
```

The prefix directory must contain:
- `include/ddwaf.h` - The header file for bindgen
- `lib/libddwaf.a` or `lib/libddwaf.so`/`lib/libddwaf.dylib` - The library file

This is useful for:
- Testing against a custom build of libddwaf
- Development when working on libddwaf itself
- Environments where downloading from GitHub is not possible

Note: Some tests that verify version matching will be skipped when
`LIBDDWAF_PREFIX` is set, since the installed version may differ from the
expected crate version.

## C++ Runtime Linking

The `libddwaf` C library is written in C++ and requires linking against the C++
standard library in certain scenarios.

On macOS, `libc++` is only available as a dynamic library, so the build script
automatically links against it:
```
cargo::rustc-link-lib=c++
```

On Linux, linking against `libstdc++` is controlled via the `link-stdcxx` feature:

```bash
# Enable static linking to libstdc++
cargo build --features link-stdcxx
```

When enabled, the build script adds:
```
cargo::rustc-link-lib=static=stdc++
```

This is only needed when linking against a dynamic `libddwaf.so` that wasn't
statically compiled against a C++ runtime or using a `libbdwaf.a` that doesn't
include the C++ runtime inside. This isn't the case with official libddwaf
releases.

## Crate Features

For details on available features (`serde`, `dynamic`, `dynamic-link`, `fips`,
`link-stdcxx`), see the [README](README.md).

# vim: set ts sw=4 ts=4 tw=80:
