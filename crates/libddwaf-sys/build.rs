use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

use flate2::read::GzDecoder;
use reqwest::blocking::get;
use tar::Archive;

// Crypto provider for the build script's own downloader. Chooses whichever of
// the `aws-lc-rs`/`ring` features is active (aws-lc-rs takes priority if both
// are somehow enabled); this is independent of the linked libddwaf itself.
#[cfg(feature = "aws-lc-rs")]
fn install_crypto_provider() {
    rustls::crypto::CryptoProvider::install_default(rustls::crypto::aws_lc_rs::default_provider())
        .expect("Failed to set rustls default crypto provider");
}

#[cfg(all(feature = "ring", not(feature = "aws-lc-rs")))]
fn install_crypto_provider() {
    rustls::crypto::CryptoProvider::install_default(rustls::crypto::ring::default_provider())
        .expect("Failed to set rustls default crypto provider");
}

#[cfg(not(any(feature = "aws-lc-rs", feature = "ring")))]
fn install_crypto_provider() {
    panic!(
        "libddwaf-sys's build script needs a rustls crypto provider to download \
         the prebuilt libddwaf archive: enable the `aws-lc-rs` or `ring` feature."
    );
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let feature_dynamic = env::var("CARGO_FEATURE_DYNAMIC").is_ok();
    let feature_dynamic_link = env::var("CARGO_FEATURE_DYNAMIC_LINK").is_ok();
    let feature_source_static = env::var("CARGO_FEATURE_SOURCE_STATIC").is_ok();
    let feature_source_shared = env::var("CARGO_FEATURE_SOURCE_SHARED").is_ok();
    let feature_source = feature_source_static || feature_source_shared;
    let libddwaf_prefix = env::var_os("LIBDDWAF_PREFIX");
    let target_os =
        env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS environment variable not set");
    let target_env = env::var("CARGO_CFG_TARGET_ENV")
        .expect("CARGO_CFG_TARGET_ENV environment variable not set");
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH")
        .expect("CARGO_CFG_TARGET_ARCH environment variable not set");

    if feature_dynamic && feature_dynamic_link {
        panic!(
            "The `dynamic` and `dynamic-link` features are mutually exclusive. Please enable only one."
        );
    }
    if feature_source_static && feature_source_shared {
        panic!("The `source-static` and `source-shared` features are mutually exclusive");
    }
    if feature_source_static && (feature_dynamic || feature_dynamic_link) {
        panic!("The `source-static` feature cannot be combined with `dynamic` or `dynamic-link`");
    }
    if feature_source_shared && !(feature_dynamic || feature_dynamic_link) {
        panic!("The `source-shared` feature requires either `dynamic` or `dynamic-link`");
    }
    if feature_source && libddwaf_prefix.is_some() {
        panic!("LIBDDWAF_PREFIX cannot be used with `source-static` or `source-shared`");
    }
    if target_os == "windows" && target_arch != "x86_64" {
        panic!("Unsupported Windows architecture: {target_arch}; only x86_64 is supported");
    }
    if target_os == "windows"
        && target_env == "gnu"
        && !feature_source_static
        && !feature_dynamic
        && !feature_dynamic_link
    {
        panic!(
            "Static linking on Windows GNU targets requires the `source-static` feature because \
             the prebuilt static library requires the MSVC C++ runtime"
        );
    }

    if cfg!(target_env = "musl") && cfg!(target_feature = "crt-static") {
        println!(
            "cargo::warning=The crt-static target feature must be disabled when building on musl targets."
        );
        println!("cargo::warning=Consider using a RUSTC_WRAPPER script to fix this up.");
    }

    if std::env::var("CARGO_FEATURE_FIPS").is_ok() {
        println!("cargo::warning=FIPS feature is enabled, checking for forbidden dependencies...");

        // List of dependencies that are not FIPS compliant
        let forbidden_dependencies = vec!["ring", "openssl", "boringssl"];

        // Check each forbidden dependency
        for dependency in &forbidden_dependencies {
            if let Err(error_msg) = check_forbidden_dependency(dependency) {
                println!("cargo::error={error_msg}");
                exit(-1);
            }
        }
        println!("cargo::warning=All dependency checks passed. No forbidden dependencies found!");
    }

    // Read the Rust crate version from the environment variable set by Cargo
    let version =
        env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION environment variable not set");

    // Select where libddwaf's headers and libraries come from.
    let (include_dir, lib_dir, soname) = if feature_source {
        from_libddwaf_src()
    } else if let Some(prefix) = libddwaf_prefix {
        from_installed_libddwaf(&prefix)
    } else {
        // No default provider is enabled in reqwest, which makes it easier to
        // maintain FIPS compliance. Install one before downloading a release.
        install_crypto_provider();
        from_github_release(&version, &out_dir)
    };
    println!("cargo::rerun-if-env-changed=LIBDDWAF_PREFIX");

    // Add library search path and link directive
    println!(
        "cargo::rustc-link-search=native={}",
        lib_dir.to_str().unwrap()
    );
    let windows_dll = target_os == "windows" && (feature_dynamic || feature_dynamic_link);
    println!("cargo::rustc-check-cfg=cfg(libddwaf_windows_dll)");
    if windows_dll {
        println!("cargo::rustc-cfg=libddwaf_windows_dll");
    }
    if !feature_source {
        if feature_dynamic_link {
            println!("cargo::rustc-link-lib=dylib=ddwaf");
        } else if !feature_dynamic {
            if target_os == "windows" {
                println!("cargo::rustc-link-lib=static=ddwaf_static");
                println!("cargo::rustc-link-lib=dylib=ws2_32");
            } else {
                println!("cargo::rustc-link-lib=static=ddwaf");
            }
        }
    }
    if target_os == "windows" && feature_dynamic_link {
        stage_windows_dll(&out_dir, &lib_dir, soname);
    }

    // macOS has libc++ only as a dynamic library, so it is not bundled in
    // libddwaf.a/.dylib.
    // Note: We check the TARGET environment variable, not cfg!(target_os), because
    // cfg! evaluates for the build script's host, not the cross-compilation target
    let target = env::var("TARGET").expect("TARGET environment variable not set");
    if target.contains("apple") || target.contains("darwin") {
        println!("cargo::rustc-link-lib=c++");
    }

    // if we want to disable this in final binaries, see maybe
    // https://github.com/rust-lang/cargo/issues/4789#issuecomment-2308131243
    match target_os.as_str() {
        "linux" => {
            println!(
                "cargo::rustc-link-arg=-Wl,-rpath,{}",
                lib_dir.to_str().unwrap()
            );
            println!("cargo::rustc-link-arg=-Wl,-rpath,$ORIGIN");
        }
        "macos" => {
            println!(
                "cargo::rustc-link-arg=-Wl,-rpath,{}",
                lib_dir.to_str().unwrap()
            );
            println!("cargo::rustc-link-arg=-Wl,-rpath,@loader_path");
        }
        "windows" => {}
        target_os => panic!("Unsupported target OS: {target_os}"),
    }

    // Generate bindings with bindgen
    let builder = bindgen::Builder::default()
        .header(include_dir.join("ddwaf.h").to_str().unwrap())
        .clang_arg(format!("-I{}", include_dir.to_str().unwrap()))
        .default_visibility(bindgen::FieldVisibilityKind::Public)
        .derive_default(true)
        .prepend_enum_name(false)
        // Specifically allow-list supported/useful functions to avoid bloat.
        .allowlist_function("^ddwaf_.*");
    // This function is in the Windows static library, but libddwaf 2.1.0 does
    // not export it from ddwaf.dll. A compatible Rust implementation is used
    // whenever the DLL is selected.
    let builder = if windows_dll {
        builder.blocklist_function("^ddwaf_object_set_string_nocopy$")
    } else {
        builder
    };
    let builder = if feature_dynamic {
        let filename = out_dir.join(format!("{soname}.zst"));
        let zstd_file = File::create(&filename).expect("failed to create zstd file");
        let mut zstd = zstd::Encoder::new(zstd_file, 22).expect("failed to create zstd encoder");

        let mut so = File::open(lib_dir.join(soname)).expect("failed to open shared object file");
        io::copy(&mut so, &mut zstd).expect("failed to write compressed shared object file");
        zstd.finish().expect("failed to finish zstd compression");

        println!(
            "cargo::rustc-env=LIBDDWAF_SHARED_OBJECT.zst={}",
            filename.display()
        );

        builder
            .dynamic_library_name("ddwaf")
            .dynamic_link_require_all(true)
    } else {
        builder
    };
    let bindings = builder.generate().expect("Failed to generate bindings");

    // Write the bindings to the output directory
    let bindings_out_path = out_dir.join("bindings.rs");
    bindings
        .write_to_file(bindings_out_path)
        .expect("Failed to write bindings.rs");

    println!("cargo::rerun-if-changed=build.rs");
}

fn from_libddwaf_src() -> (PathBuf, PathBuf, &'static str) {
    let include_dir = PathBuf::from(
        env::var_os("DEP_DDWAF_SRC_INCLUDE")
            .expect("libddwaf-src did not export its include directory"),
    );
    let lib_dir = PathBuf::from(
        env::var_os("DEP_DDWAF_SRC_LIB")
            .expect("libddwaf-src did not export its library directory"),
    );

    assert!(
        include_dir.join("ddwaf.h").is_file(),
        "libddwaf-src did not build ddwaf.h under {}",
        include_dir.display()
    );
    assert!(
        lib_dir.is_dir(),
        "libddwaf-src did not build its libraries under {}",
        lib_dir.display()
    );

    (include_dir, lib_dir, shared_library_name())
}

fn from_installed_libddwaf(prefix: impl AsRef<OsStr>) -> (PathBuf, PathBuf, &'static str) {
    println!(
        "cargo::warning=Using libddwaf installation from prefix: {:?}",
        prefix.as_ref()
    );
    let prefix_path = PathBuf::from(prefix.as_ref());
    let include_dir = prefix_path.join("include");
    let lib_dir = prefix_path.join("lib");

    // Validate that the directories exist
    if !include_dir.exists() {
        panic!("Include directory not found at {}", include_dir.display());
    }
    if !lib_dir.exists() {
        panic!("Library directory not found at {}", lib_dir.display());
    }

    // Determine the shared library name based on the target platform
    (include_dir, lib_dir, shared_library_name())
}

fn shared_library_name() -> &'static str {
    match env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("macos") => "libddwaf.dylib",
        Ok("linux") => "libddwaf.so",
        Ok("windows") => "ddwaf.dll",
        Ok(target_os) => panic!("Unsupported target OS: {target_os}"),
        Err(error) => panic!("CARGO_CFG_TARGET_OS is unavailable: {error}"),
    }
}

fn stage_windows_dll(out_dir: &Path, lib_dir: &Path, soname: &str) {
    // Windows searches beside the executable for dependent DLLs. Cargo places
    // normal binaries in the profile directory, test binaries in `deps`, and
    // example binaries in `examples`.
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("OUT_DIR did not contain a Cargo profile directory");
    let source = lib_dir.join(soname);

    for destination_dir in [
        profile_dir.to_owned(),
        profile_dir.join("deps"),
        profile_dir.join("examples"),
    ] {
        fs::create_dir_all(&destination_dir).expect("Failed to create Cargo output directory");
        fs::copy(&source, destination_dir.join(soname))
            .expect("Failed to copy ddwaf.dll to Cargo output directory");
    }
}

fn from_github_release(version: &str, out_dir: &Path) -> (PathBuf, PathBuf, &'static str) {
    // Download and extract libddwaf from GitHub releases

    // Target triple for the current build
    let target = env::var("TARGET").expect("TARGET environment variable not set");
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS unavailable");

    // Output directory
    let download_dir = out_dir.join("download").join(&target);
    let include_dir = download_dir.join("include");
    let lib_dir = download_dir.join("lib");

    let (archive, soname, is_override) = {
        // Base URL for downloading the library
        let base_url = "https://github.com/DataDog/libddwaf/releases/download";

        // Map the target triple to the correct library archive
        let (archive_name, soname) = match target.as_str() {
            "x86_64-unknown-linux-gnu" => (
                format!("libddwaf-{version}-x86_64-linux-musl.tar.gz"),
                "libddwaf.so",
            ),
            // "x86_64-alpine-linux-musl" is Alpine's own (non-rustup) cargo/rustc
            // reporting its host triple with an "alpine" vendor instead of "unknown".
            "x86_64-unknown-linux-musl" | "x86_64-alpine-linux-musl" => (
                format!("libddwaf-{version}-x86_64-linux-musl.tar.gz"),
                "libddwaf.so",
            ),
            "aarch64-unknown-linux-gnu" => (
                format!("libddwaf-{version}-aarch64-linux-musl.tar.gz"),
                "libddwaf.so",
            ),
            "aarch64-unknown-linux-musl" | "aarch64-alpine-linux-musl" => (
                format!("libddwaf-{version}-aarch64-linux-musl.tar.gz"),
                "libddwaf.so",
            ),
            "armv7-unknown-linux-musleabihf" => (
                format!("libddwaf-{version}-armv7-linux-musl.tar.gz"),
                "libddwaf.so",
            ),
            "aarch64-apple-darwin" => (
                format!("libddwaf-{version}-darwin-arm64.tar.gz"),
                "libddwaf.dylib",
            ),
            "x86_64-apple-darwin" => (
                format!("libddwaf-{version}-darwin-x86_64.tar.gz"),
                "libddwaf.dylib",
            ),
            _ if target_os == "windows" => (
                format!("libddwaf-{version}-windows-x64.tar.gz"),
                "ddwaf.dll",
            ),
            target => panic!("Unsupported target platform: {target}"),
        };

        // Construct the download URL
        let archive_url = format!("{base_url}/{version}/{archive_name}");
        let response = get(&archive_url).expect("Failed to download archive");
        assert!(
            response.status().is_success(),
            "Failed to download archive from {archive_url}: {status}",
            status = response.status()
        );
        (response, soname, false)
    };

    // Extract the archive
    let ar = env::var("AR").unwrap_or("ar".to_string());
    if is_override || !include_dir.exists() || !lib_dir.exists() {
        fs::create_dir_all(&download_dir).expect("Failed to create extraction directory");

        let reader = GzDecoder::new(archive);
        let mut tar = Archive::new(reader);
        for entry in tar.entries().expect("Failed to get tar archive entries") {
            let mut entry = entry.expect("Failed to get tar archive entry");
            if entry.header().entry_type().is_dir() {
                continue;
            }

            let path = entry.path().expect("Failed to get tar archive entry path");
            let mut components = path.components();
            if components.next().is_none() {
                continue;
            }
            let out_path = download_dir.join(components.as_path());
            let out_dir = out_path
                .parent()
                .expect("Failed to compute dir name of output file");
            fs::create_dir_all(out_dir).expect("Failed to create directory for archive entry");
            let mut file =
                File::create(&out_path).expect("Failed to create file for archive entry");
            io::copy(&mut entry, &mut file)
                .expect("Failed to write archive entry contents to file");

            if out_path.extension() == Some(OsStr::new("a")) {
                // We remove the `Unwind*` objects from the static archives, as they are the LLVM `libunwind` unwinder,
                // which conflicts with the unwinder provided by the rust standard library (there can only be one
                // unwinder in any given program). Failure to do so breaks the `panic` unwinding logic (resulting in a
                // `SIGABORT` caused by `libunwind` hitting error 3). This is not an issue with dynamic libraries, as
                // the `libunwind` symbols there will just never be used.
                let entries = Command::new(&ar)
                    .arg("t")
                    .arg(&out_path)
                    .output()
                    .expect("failed to run ar t");
                let to_remove = entries
                    .stdout
                    .lines()
                    .map(|line| line.expect("failed to read line"))
                    .filter(|line| line.starts_with("Unwind"))
                    .collect::<Vec<_>>();
                if !to_remove.is_empty() {
                    assert!(
                        Command::new(&ar)
                            .arg("ds")
                            .arg(&out_path)
                            .args(to_remove)
                            .status()
                            .expect("failed to run ar d")
                            .success(),
                        "failed to run ar ds"
                    );
                }
            }
        }
    }

    // Check the extracted contents
    if !include_dir.exists() || !lib_dir.exists() {
        panic!("Failed to extract include and lib directories");
    }

    (include_dir, lib_dir, soname)
}

/// Checks if a specific dependency is present in the dependency tree when FIPS is enabled.
fn check_forbidden_dependency(dependency_name: &str) -> Result<(), String> {
    println!("cargo::warning=Checking for {dependency_name} dependency...");

    // First run cargo tree to get dependency with detailed info
    let output = Command::new("cargo")
        .args([
            "tree",
            "-i",
            dependency_name,
            "--format={p} {f}",
            "--prefix=none",
            "--features=fips",
            "--no-default-features",
        ])
        .output()
        .map_err(|e| format!("Failed to execute cargo tree command for {dependency_name}: {e}"))?;

    // Also get the complete dependency path to help debugging
    let path_output = Command::new("cargo")
        .args([
            "tree",
            "-i",
            dependency_name,
            "--edges=features",
            "--features=fips",
            "--no-default-features",
        ])
        .output()
        .map_err(|e| {
            format!("Failed to execute detailed cargo tree command for {dependency_name}: {e}")
        })?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    let dependency_pattern = format!("{dependency_name} v");

    // Check if the dependency is in the dependency tree
    if output_str.contains(&dependency_pattern) {
        // Get the dependency paths
        let deps: Vec<&str> = output_str
            .lines()
            .filter(|line| line.contains(&dependency_pattern))
            .collect();

        // Get the detailed dependency path
        let path_str = String::from_utf8_lossy(&path_output.stdout);

        // Create detailed error message with dependency paths
        let error_msg = format!(
            "\n\nERROR: {dependency_name} dependency detected with FIPS feature enabled!\n\
            FIPS compliance requires eliminating this dependency.\n\
            \n\
            {dependency_name} dependency versions and features:\n{deps}\n\
            \n\
            Detailed dependency paths to {dependency_name}:\n{path_str}\n\
            \n\
            Ensure all dependencies use aws-lc-rs instead of non-FIPS compliant cryptographic libraries.\n\
            Consider updating the following in your Cargo.toml:\n\
            1. Ensure all dependencies that use rustls have the 'aws-lc-rs' feature\n\
            2. Check transitive dependencies in reqwest, hyper-rustls, etc.\n\
            3. Update your dependencies to versions that support FIPS mode\n",
            deps = deps.join("\n"),
        );

        Err(error_msg)
    } else {
        println!(
            "cargo::warning=No {dependency_name} dependency found. FIPS compliance check passed for this dependency!"
        );
        Ok(())
    }
}
