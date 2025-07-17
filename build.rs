use std::env;
use std::fs::{self, File};
use std::io;
use std::path::PathBuf;
use std::process::{exit, Command};

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
    let manifest_dir =
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR environment variable not set");

    // Target triple for the current build
    let target = env::var("TARGET").expect("TARGET environment variable not set");

    // Output directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let download_dir = out_dir.join("download").join(&target);
    let include_dir = download_dir.join("include");
    let lib_dir = download_dir.join("lib");

    // Download the archive
    let override_archive = PathBuf::from(&manifest_dir)
        .join("target")
        .join("libddwaf.tar.gz");
    let archive: Box<dyn io::Read> = if fs::exists(&override_archive).unwrap_or_default() {
        println!("cargo:warning=Using local archive at {override_archive:?}");
        Box::new(fs::File::open(&override_archive).unwrap())
    } else {
        // Base URL for downloading the library
        let base_url = "https://github.com/DataDog/libddwaf/releases/download";

        // Map the target triple to the correct library archive
        let archive_name = match target.as_str() {
            "x86_64-unknown-linux-gnu" => format!("libddwaf-{version}-x86_64-linux-musl.tar.gz"),
            "x86_64-unknown-linux-musl" => format!("libddwaf-{version}-x86_64-linux-musl.tar.gz"),
            "aarch64-unknown-linux-gnu" => format!("libddwaf-{version}-aarch64-linux-musl.tar.gz"),
            "aarch64-unknown-linux-musl" => format!("libddwaf-{version}-aarch64-linux-musl.tar.gz"),
            "armv7-unknown-linux-musleabihf" => {
                format!("libddwaf-{version}-armv7-linux-musl.tar.gz")
            }
            "aarch64-apple-darwin" => format!("libddwaf-{version}-darwin-arm64.tar.gz"),
            "x86_64-apple-darwin" => format!("libddwaf-{version}-darwin-x86_64.tar.gz"),
            _ => panic!("Unsupported target platform: {target}"),
        };

        // Construct the download URL
        let archive_url = format!("{base_url}/{version}/{archive_name}");

        Box::new(get(&archive_url).expect("Failed to download archive"))
    };
    println!("cargo:rerun-if-changed={override_archive:?}");

    // Extract the archive
    if !include_dir.exists() || !lib_dir.exists() {
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
            let mut file = File::create(out_path).expect("Failed to create file for archive entry");
            io::copy(&mut entry, &mut file)
                .expect("Failed to write archive entry contents to file");
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
    println!("cargo:rustc-link-lib=dylib=ddwaf");

    // macOS has libc++ only as a dynamic library, so it's not bundled in libddwaf.a
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
    println!("cargo:rerun-if-changed=build.rs");

    // Generate bindings with bindgen
    let bindings = bindgen::Builder::default()
        .header(include_dir.join("ddwaf.h").to_str().unwrap())
        .allowlist_item("^_?ddwaf_.+")
        .blocklist_function("^ddwaf_init$") // This will eventually disappear from libddwaf
        .clang_arg(format!("-I{}", include_dir.to_str().unwrap()))
        .default_visibility(bindgen::FieldVisibilityKind::PublicCrate)
        .derive_default(true)
        .prepend_enum_name(false)
        .generate()
        .expect("Failed to generate bindings");

    // Write the bindings to the output directory
    let bindings_out_path = out_dir.join("bindings.rs");
    bindings
        .write_to_file(bindings_out_path)
        .expect("Failed to write bindings.rs");

    println!(
        "cargo:rerun-if-changed={}",
        include_dir.join("ddwaf.h").to_str().unwrap()
    );
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
        println!("cargo:warning=No {dependency_name} dependency found. FIPS compliance check passed for this dependency!");
        Ok(())
    }
}
