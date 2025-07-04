use std::env;
use std::fs::{self, File};
use std::io;
use std::path::PathBuf;

use flate2::read::GzDecoder;
use reqwest::blocking::get;
use tar::Archive;

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
    let include_dir = download_dir.join("include");
    let lib_dir = download_dir.join("lib");

    // Download the archive
    let archive = get(&archive_url).expect("Failed to download archive");

    // Extract the archive
    if !include_dir.exists() || !lib_dir.exists() {
        println!("Extracting {:?}", &archive_url);
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
    if cfg!(not(feature = "shared_lib")) {
        println!("cargo:rustc-link-lib=static=ddwaf");
    } else {
        println!("cargo:rustc-link-lib=dylib=ddwaf");
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
    }
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(target_os = "macos")]
    println!("cargo::rustc-link-lib=c++");

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
        .write_to_file(bindings_out_path)
        .expect("Failed to write bindings.rs");

    println!(
        "cargo:rerun-if-changed={}",
        include_dir.join("ddwaf.h").to_str().unwrap()
    );
}
