use std::{fs, path::Path};

use crate::common::{Command, Manifest};

mod common;

/// Runs a specific Wasm 1.0 spec test from its manifest/.json file.
fn run_spec_test(manifest_path: &Path) {
    //let dir = manifest_path.parent().unwrap();
    let text = fs::read_to_string(manifest_path).unwrap();

    let manifest: Manifest = serde_json::from_str(&text).unwrap();
    
    for cmd in &manifest.commands {
        match cmd {
            Command::Module { filename, line } => {
                println!("now running {filename}...");
            },
            _ => {}
        }
    }

    println!();
}

/// Tests the entire WebAssembly 1.0 spec test suite.
#[test]
fn run_spec_test_suite() {
    common::convert_wasts();

    let dir = fs::read_dir(Path::new(common::SPEC_OUTPUT_DIR))
        .expect(&format!("{} should exist.", common::SPEC_OUTPUT_DIR));
    
    for entry in dir {
        let path = entry.unwrap().path();

        if path.extension().map_or(false, |e| e == "json") {
            println!("running {}", path.display());

            run_spec_test(&path);
        }
    }
}