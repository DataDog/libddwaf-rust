use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Read the Rust crate version from the environment variable set by Cargo
    let version =
        env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION environment variable not set");

    // Target triple for the current build
    let target = env::var("TARGET").expect("TARGET environment variable not set");

    // Base URL for downloading the library
    let base_url = "https://github.com/DataDog/libddwaf/releases/download";

    // Map the target triple to the correct library archive
    let archive_name = match target.as_str() {
        "x86_64-unknown-linux-gnu" => format!("libddwaf-{}-x86_64-linux-musl.tar.gz", version),
        "x86_64-unknown-linux-musl" => format!("libddwaf-{}-x86_64-linux-musl.tar.gz", version),
        "aarch64-unknown-linux-gnu" => format!("libddwaf-{}-aarch64-linux-musl.tar.gz", version),
        "aarch64-unknown-linux-musl" => format!("libddwaf-{}-aarch64-linux-musl.tar.gz", version),
        "armv7-unknown-linux-musleabihf" => format!("libddwaf-{}-armv7-linux-musl.tar.gz", version),
        "aarch64-apple-darwin" => format!("libddwaf-{}-darwin-arm64.tar.gz", version),
        "x86_64-apple-darwin" => format!("libddwaf-{}-darwin-x86_64.tar.gz", version),
        _ => panic!("Unsupported target platform: {}", target),
    };

    // Construct the download URL
    let archive_url = format!("{}/{}/{}", base_url, version, archive_name);

    // Output directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let download_dir = out_dir.join("libddwaf_download");
    let extract_path = download_dir.join("extracted");
    let include_dir = extract_path.join("include");
    let lib_dir = extract_path.join("lib");

    // Ensure the download directory exists
    fs::create_dir_all(&download_dir).expect("Failed to create download directory");

    // Path to the downloaded archive
    let archive_path = download_dir.join(&archive_name);

    // Download the archive
    if !archive_path.exists() {
        println!("Downloading {}", archive_url);
        let status = Command::new("curl")
            .args(&["-Lf", "-o", archive_path.to_str().unwrap(), &archive_url])
            .status()
            .expect("Failed to execute curl");
        assert!(status.success(), "Failed to download {}", archive_url);
    }

    // Extract the archive
    if !extract_path.exists() {
        println!("Extracting {:?}", archive_path);
        fs::create_dir_all(&extract_path).expect("Failed to create extraction directory");
        let status = Command::new("tar")
            .args(&[
                "--strip-components=1",
                "-xzf",
                archive_path.to_str().unwrap(),
                "-C",
                extract_path.to_str().unwrap(),
            ])
            .status()
            .expect("Failed to execute tar");
        assert!(status.success(), "Failed to extract {}", archive_name);
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
    println!("cargo:rustc-link-lib=ddwaf");
    println!("cargo:rerun-if-changed=build.rs");

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
    let bindings = bindgen::Builder::default()
        .header(include_dir.join("ddwaf.h").to_str().unwrap())
        .blocklist_type(".*pthread.*")
        .clang_arg(format!("-I{}", include_dir.to_str().unwrap()))
        .generate()
        .expect("Failed to generate bindings");

    // Write the bindings to the output directory
    let bindings_out_path = out_dir.join("bindings.rs");
    bindings
        .write_to_file(&bindings_out_path)
        .expect("Failed to write bindings.rs");

    println!(
        "cargo:rerun-if-changed={}",
        include_dir.join("ddwaf.h").to_str().unwrap()
    );
}
