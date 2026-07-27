use std::{fs, path::Path, rc::Rc};

use wasp::{executor::{ModuleInstance, Store}, module::Module, validator::Validator};

use crate::common::{Command, Manifest, register_spectest, spectest_imports};

mod common;

/// Runs a specific Wasm 1.0 spec test from its manifest/.json file.
fn run_spec_test(manifest_path: &Path) {
    let dir = manifest_path.parent().unwrap();
    let text = fs::read_to_string(manifest_path).unwrap();

    let manifest: Manifest = serde_json::from_str(&text).unwrap();

    let mut store = Store::new();
    let mut current_instance: Option<Rc<ModuleInstance>> = None;

    // add spectest module to store
    register_spectest(&mut store);
    
    for cmd in &manifest.commands {
        match cmd {
            Command::Module { filename, line } => {
                let bytes = fs::read(dir.join(filename))
                    .unwrap_or_else(|e| panic!("line {line}: failed to read {filename}: {e}"));
                
                let module = Module::decode(&bytes)
                    .unwrap_or_else(|e| panic!("line {line}: failed to decode {filename}: {e:?}"));

                for imp in &module.imports {
                    println!("{:?}", imp)
                }

                Validator::validate(&module)
                    .unwrap_or_else(|e| panic!("line {line}: failed to validate {filename}: {e:?}"));

                let instance = ModuleInstance::new(&module, &mut store, Vec::new())
                    .unwrap_or_else(|e| panic!("line {line}: failed to instantiate {filename}: {e:?}"));

                current_instance = Some(instance);
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