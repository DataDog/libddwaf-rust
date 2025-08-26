use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, BufRead};
use std::path::PathBuf;
use std::process::{Command, exit};

use flate2::read::GzDecoder;
use reqwest::blocking::get;
use tar::Archive;

fn main() {
    if std::env::var("CARGO_FEATURE_FIPS").is_ok() {
        println!("cargo:warning=FIPS feature is enabled, checking for forbidden dependencies...");

        // List of dependencies that are not FIPS compliant
        let forbidden_dependencies = vec!["ring", "openssl", "boringssl"];

        // Check each forbidden dependency
        for dependency in &forbidden_dependencies {
            if let Err(error_msg) = check_forbidden_dependency(dependency) {
                println!("cargo:error={error_msg}");
                exit(-1);
            }
        }
        println!("cargo:warning=All dependency checks passed. No forbidden dependencies found!");
    }

    // Ensure reqwest is able to use a crypto provider (no default is set so it's easier to maintain FIPS compliance)
    rustls::crypto::CryptoProvider::install_default(rustls::crypto::aws_lc_rs::default_provider())
        .expect("Failed to set rustls default crypto provider");

    // Read the Rust crate version from the environment variable set by Cargo
    let version =
        env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION environment variable not set");

    // Target triple for the current build
    let target = env::var("TARGET").expect("TARGET environment variable not set");

    // Output directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let download_dir = out_dir.join("download").join(&target);
    let include_dir = download_dir.join("include");
    let lib_dir = download_dir.join("lib");

    // Check if the `dynamic` feature is enabled.
    let feature_dynamic = env::var("CARGO_FEATURE_DYNAMIC").is_ok();

    // Download the archive
    let (archive, soname, is_override) = {
        // Base URL for downloading the library
        let base_url = "https://github.com/DataDog/libddwaf/releases/download";

        // Map the target triple to the correct library archive
        let (archive_name, soname) = match target.as_str() {
            "x86_64-unknown-linux-gnu" => (
                format!("libddwaf-{version}-x86_64-linux-musl.tar.gz"),
                "libddwaf.so",
            ),
            "x86_64-unknown-linux-musl" => (
                format!("libddwaf-{version}-x86_64-linux-musl.tar.gz"),
                "libddwaf.so",
            ),
            "aarch64-unknown-linux-gnu" => (
                format!("libddwaf-{version}-aarch64-linux-musl.tar.gz"),
                "libddwaf.so",
            ),
            "aarch64-unknown-linux-musl" => (
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

    // Add library search path and link directive
    println!(
        "cargo:rustc-link-search=native={}",
        lib_dir.to_str().unwrap()
    );
    if !feature_dynamic {
        println!("cargo:rustc-link-lib=static=ddwaf");
    }

    // macOS has libc++ only as a dynamic library, so it's not bundled in libddwaf.a/.so.
    #[cfg(target_os = "macos")]
    println!("cargo::rustc-link-lib=c++");

    // if we want to disable this in final binaries, see maybe
    // https://github.com/rust-lang/cargo/issues/4789#issuecomment-2308131243
    println!(
        "cargo:rustc-link-arg=-Wl,-rpath,{}",
        lib_dir.to_str().unwrap()
    );

    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");

    // Generate bindings with bindgen
    let builder = bindgen::Builder::default()
        .header(include_dir.join("ddwaf.h").to_str().unwrap())
        .clang_arg(format!("-I{}", include_dir.to_str().unwrap()))
        .default_visibility(bindgen::FieldVisibilityKind::Public)
        .derive_default(true)
        .prepend_enum_name(false)
        // Specifically allow-list supported/useful functions to avoid bloat.
        .allowlist_function("^ddwaf_(get_version|set_log_cb)$")
        .allowlist_function("^ddwaf_(destroy|known_(addresses|actions))$")
        .allowlist_function("^ddwaf_(context_(init|destroy)|run)$")
        .allowlist_function("^ddwaf_builder_(init|add_or_update_config|remove_config|build_instance|get_config_paths|destroy)$")
        .allowlist_function(
            "^ddwaf_object_(array|bool|float|free|from_json|invalid|map|null|signed|stringl|unsigned)$",
        );
    let builder = if feature_dynamic {
        let filename = download_dir.join(format!("{soname}.zst"));
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

    println!("cargo:rerun-if-changed=build.rs");
}

/// Checks if a specific dependency is present in the dependency tree when FIPS is enabled.
fn check_forbidden_dependency(dependency_name: &str) -> Result<(), String> {
    println!("cargo:warning=Checking for {dependency_name} dependency...");

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
            "cargo:warning=No {dependency_name} dependency found. FIPS compliance check passed for this dependency!"
        );
        Ok(())
    }
}
