#![cfg(all(feature = "dynamic-manual", not(miri)))]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Barrier};
use std::time::Duration;

use libddwaf::{load_library, version, Builder, Config, LoadError, RunResult, RunnableContext};
use libddwaf::{waf_array, waf_map};

fn exercise_waf() {
    assert_eq!(version().to_bytes(), env!("CARGO_PKG_VERSION").as_bytes());
    let mut builder = Builder::new(Some(&Config::default())).unwrap();
    let rules = waf_map! {
        ("version", "2.1"),
        ("rules", waf_array! { waf_map! {
            ("id", "manual-loading"),
            ("name", "Manual loading smoke test"),
            ("tags", waf_map! { ("type", "test"), ("category", "test") }),
            ("conditions", waf_array! { waf_map! {
                ("operator", "match_regex"),
                ("parameters", waf_map! {
                    ("regex", "attack"),
                    ("inputs", waf_array! { waf_map! { ("address", "input") } }),
                }),
            } }),
            ("on_match", waf_array! { "block" }),
        } }),
    };
    assert!(builder.add_or_update_config("rules", &rules, None));
    let handle = builder.build().unwrap();
    let mut context = handle.new_context();
    assert!(matches!(
        context.run(waf_map! { ("input", "attack") }, Duration::from_secs(1)),
        Ok(RunResult::Match(_))
    ));
}

fn run_case(case: &str, library: &Path) {
    // Failed initialization must not make product startup terminate the process.
    assert!(Builder::new(None).is_none());
    match case {
        "explicit" => unsafe { load_library(Some(library)) }.unwrap(),
        "default" => unsafe { load_library(None) }.unwrap(),
        "retry" => {
            let missing = library.with_file_name("missing-libddwaf-library");
            let error = unsafe { load_library(Some(&missing)) }.unwrap_err();
            assert!(matches!(error, LoadError::Library(_)));
            assert!(Builder::new(None).is_none());
            unsafe { load_library(Some(library)) }.unwrap();
        }
        "concurrent" => {
            let barrier = Arc::new(Barrier::new(8));
            let threads: Vec<_> = (0..8)
                .map(|_| {
                    let barrier = Arc::clone(&barrier);
                    let path = library.to_owned();
                    std::thread::spawn(move || {
                        barrier.wait();
                        unsafe { load_library(Some(&path)) }
                    })
                })
                .collect();
            let mut successes = 0;
            for thread in threads {
                match thread.join().unwrap() {
                    Ok(()) => successes += 1,
                    Err(LoadError::AlreadyLoaded) => {}
                    result => panic!("unexpected concurrent loading result: {result:?}"),
                }
            }
            assert_eq!(successes, 1);
        }
        "version" | "null-version" | "symbols" => {
            let fixture = std::env::var_os("LIBDDWAF_TEST_FIXTURE").unwrap();
            let error = unsafe { load_library(Some(Path::new(&fixture))) }.unwrap_err();
            match case {
                "version" => assert!(matches!(error, LoadError::Version { found: Some(_), .. })),
                "null-version" => {
                    assert!(matches!(error, LoadError::Version { found: None, .. }));
                }
                _ => assert!(matches!(error, LoadError::Symbol(_))),
            }
            assert!(Builder::new(None).is_none());
            unsafe { load_library(Some(library)) }.unwrap();
        }
        _ => panic!("unknown case: {case}"),
    }
    exercise_waf();
    // Even an invalid new path cannot replace a successfully loaded library.
    assert!(matches!(
        unsafe { load_library(Some(Path::new("missing-replacement"))) },
        Err(LoadError::AlreadyLoaded)
    ));
    exercise_waf();
}

fn spawn_case(case: &str, library: &Path, fixture: Option<&Path>) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "manual_loading", "--nocapture"])
        .env("LIBDDWAF_TEST_CASE", case)
        .env("LIBDDWAF_TEST_LIBRARY", library);
    if let Some(fixture) = fixture {
        command.env("LIBDDWAF_TEST_FIXTURE", fixture);
    }
    // Set the resolver's search path before process startup, never concurrently.
    let search_var = if cfg!(target_os = "macos") {
        "DYLD_LIBRARY_PATH"
    } else if cfg!(target_os = "windows") {
        "PATH"
    } else {
        "LD_LIBRARY_PATH"
    };
    let mut paths = vec![library.parent().unwrap().to_owned()];
    paths.extend(std::env::split_paths(
        &std::env::var_os(search_var).unwrap_or_default(),
    ));
    command.env(search_var, std::env::join_paths(paths).unwrap());
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "case {case} failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
fn build_fixture(directory: &Path, name: &str, version: &str) -> PathBuf {
    let source = directory.join(format!("{name}.c"));
    let library = directory.join(format!("{name}{}", std::env::consts::DLL_SUFFIX));
    std::fs::write(
        &source,
        format!("const char *ddwaf_get_version(void) {{ return {version}; }}\n"),
    )
    .unwrap();
    let output = Command::new("cc")
        .arg(if cfg!(target_os = "macos") {
            "-dynamiclib"
        } else {
            "-shared"
        })
        .arg("-fPIC")
        .arg(&source)
        .arg("-o")
        .arg(&library)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    library
}

#[test]
fn manual_loading() {
    let library = PathBuf::from(std::env::var_os("LIBDDWAF_TEST_LIBRARY").expect(
        "set LIBDDWAF_TEST_LIBRARY to a trusted libddwaf shared library for this integration test",
    ));
    if let Ok(case) = std::env::var("LIBDDWAF_TEST_CASE") {
        run_case(&case, &library);
        return;
    }
    let library = std::fs::canonicalize(library).unwrap();
    for case in ["explicit", "default", "retry", "concurrent"] {
        spawn_case(case, &library, None);
    }
    #[cfg(unix)]
    {
        let directory =
            std::env::temp_dir().join(format!("libddwaf-loader-{}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        for (case, version) in [
            ("version", "\"1.0.0\"".to_owned()),
            ("null-version", "0".to_owned()),
            ("symbols", format!("\"{}\"", env!("CARGO_PKG_VERSION"))),
        ] {
            let fixture = build_fixture(&directory, case, &version);
            spawn_case(case, &library, Some(&fixture));
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
}
