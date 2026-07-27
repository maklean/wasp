use std::{collections::HashMap, fs, path::Path, rc::Rc};

use wasp::{executor::{ModuleInstance, Store, Val}, module::Module, validator::Validator};

use crate::common::{Command, Manifest, register_spectest, spectest_imports, vals_match};

mod common;

/// Runs a specific Wasm 1.0 spec test from its manifest/.json file.
fn run_spec_test(manifest_path: &Path) {
    let dir = manifest_path.parent().unwrap();
    let text = fs::read_to_string(manifest_path).unwrap();

    let manifest: Manifest = serde_json::from_str(&text).unwrap();

    let mut store = Store::new();
    let mut current_instance: Option<Rc<ModuleInstance>> = None;

    // add spectest module to store
    let spectest_exports = register_spectest(&mut store);

    // map of all the seen modules throughout the spec test
    let mut registered_modules: HashMap<String, Rc<ModuleInstance>> = HashMap::new();
    
    for cmd in &manifest.commands {
        match cmd {
            Command::Module { filename, line, name } => {
                let bytes = fs::read(dir.join(filename))
                    .unwrap_or_else(|e| panic!("line {line}: failed to read {filename}: {e}"));
                
                let module = Module::decode(&bytes)
                    .unwrap_or_else(|e| panic!("line {line}: failed to decode {filename}: {e:?}"));

                Validator::validate(&module)
                    .unwrap_or_else(|e| panic!("line {line}: failed to validate {filename}: {e:?}"));

                let instance = ModuleInstance::new(&module, &mut store, spectest_imports(&module, &spectest_exports, &registered_modules, filename, line))
                    .unwrap_or_else(|e| panic!("line {line}: failed to instantiate {filename}: {e:?}"));

                if !name.is_empty() {
                    registered_modules.insert(name.clone(), Rc::clone(&instance));
                }

                println!("loaded {filename} (name={name:?}), exports: {:?}", instance.exports.iter().map(|e| (&e.name, &e.value)).collect::<Vec<_>>());

                current_instance = Some(instance);
            },

            Command::Register { line, name, as_ } => {
                let instance = if name.is_empty() {
                    current_instance.as_ref()
                        .unwrap_or_else(|| panic!("line {line}: no current module to register"))
                } else {
                    registered_modules.get(name)
                        .expect(&format!("line {line}: module '{name}' should be registered already."))
                };

                registered_modules.insert(as_.to_string(), Rc::clone(instance));
            },

            Command::AssertReturn { action, expected, line } => {
                let instance = match &action.module {
                    Some(module) => registered_modules.get(module)
                        .unwrap_or_else(|| panic!("line {line}: module '{module}' is not registered.")),

                    None => current_instance.as_ref()
                        .unwrap_or_else(|| panic!("line {line}: no module loaded"))
                };
                
                let args: Vec<Val> = action.args
                    .iter()
                    .map(|arg_val| arg_val.clone().try_into().unwrap())
                    .collect();

                let actual_vals = instance.invoke_export(&mut store, &action.field, args)
                    .unwrap_or_else(|e| panic!("line {line}: expected success, got {e:?}"));

                let expected_vals: Vec<Val> = expected
                    .iter()
                    .map(|arg_val| arg_val.clone().try_into().unwrap())
                    .collect();

                assert!(
                    vals_match(&actual_vals, &expected_vals),
                    "line {line}: expected {expected_vals:?}, got {actual_vals:?}"
                )
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