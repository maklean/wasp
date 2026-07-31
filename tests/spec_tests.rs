use std::{collections::HashMap, fs, path::Path, rc::Rc};

use wasp::{errors::ExecuteError, executor::{ExternVal, ModuleInstance, Store, Val}, module::Module};

use crate::common::{Action, Command, Manifest, register_spectest, resolve_module, spectest_imports, vals_match};

mod common;

/// Stack size for spec tests (in MB).
const STACK_SIZE_MB: usize = 256; 

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
                
                let module = Module::decode_from_bytes(&bytes)
                    .unwrap_or_else(|e| panic!("line {line}: failed to decode {filename}: {e:?}"));

                module.validate()
                    .unwrap_or_else(|e| panic!("line {line}: failed to validate {filename}: {e:?}"));

                let imports = spectest_imports(&module, &spectest_exports, &registered_modules, filename, line).unwrap();
                let instance = module.instantiate(&mut store, imports)
                    .unwrap_or_else(|e| panic!("line {line}: failed to instantiate {filename}: {e:?}"));

                if !name.is_empty() {
                    registered_modules.insert(name.clone(), Rc::clone(&instance));
                }

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

            Command::Action { action, line } => {
                match action {
                    Action::Invoke { module, field, args } => {
                        let instance = resolve_module(module, &current_instance, &registered_modules, *line);
                    
                        let args: Vec<Val> = args
                            .iter()
                            .map(|arg_val| arg_val.clone().try_into().unwrap())
                            .collect();

                        instance.invoke_export(&mut store, field, args)
                            .unwrap_or_else(|e| panic!("line {line}: expected success, got {e:?}"));
                    },

                    _ => unreachable!()
                }
            },

            Command::AssertReturn { action, expected, line } => {
                let expected_vals: Vec<Val> = expected
                    .iter()
                    .map(|arg_val| arg_val.clone().try_into().unwrap())
                    .collect();

                match action {
                    Action::Invoke { module, field, args } => {
                        let instance = resolve_module(module, &current_instance, &registered_modules, *line);
                    
                        let args: Vec<Val> = args
                            .iter()
                            .map(|arg_val| arg_val.clone().try_into().unwrap())
                            .collect();

                        let actual_vals = instance.invoke_export(&mut store, field, args)
                            .unwrap_or_else(|e| panic!("line {line}: expected success, got {e:?}"));

                        assert!(
                            vals_match(&actual_vals, &expected_vals),
                            "line {line}: expected {expected_vals:?}, got {actual_vals:?}"
                        )
                    },

                    Action::Get { module, field } => {
                        let instance = resolve_module(module, &current_instance, &registered_modules, *line);

                        let export = instance.exports
                            .iter()
                            .find(|e| e.name == *field)
                            .unwrap_or_else(|| panic!("line {line}: export '{field}' not found"));

                        let ExternVal::Global(global_addr) = export.value else {
                            panic!("line {line}: export '{field}' is not a global");
                        };

                        let value = store.globals[global_addr].value;

                        assert!(
                            vals_match(&[value], &expected_vals),
                            "line {line}: expected {expected_vals:?}, got [{value:?}]"
                        );
                    },
                }
            },

            Command::AssertExhaustion { action, line } => {
                match action {
                    Action::Invoke { module, field, args } => {
                        let instance = resolve_module(module, &current_instance, &registered_modules, *line);

                        let args: Vec<Val> = args
                            .iter()
                            .map(|arg_val| arg_val.clone().try_into().unwrap())
                            .collect();

                        let result = instance.invoke_export(&mut store, field, args);

                        let Err(err) = result else {
                            panic!("line {line}: expected '{field}' to exhaust call stack.");
                        };

                        assert_eq!(
                            err, ExecuteError::CallStackExhausted, 
                            "line {line}: expected error to be exhausted call stack error, got {err:?}"
                        );
                    },

                    // there shouldn't be any assert_exhaustion tests on Action::Get
                    _ => unreachable!()
                }
            },

            Command::AssertTrap { action, line } => {
                match action {
                    Action::Invoke { module, field, args } => {
                        let instance = resolve_module(module, &current_instance, &registered_modules, *line);

                        let args: Vec<Val> = args
                            .iter()
                            .map(|arg_val| arg_val.clone().try_into().unwrap())
                            .collect();

                        let result = instance.invoke_export(&mut store, field, args);

                        let Err(err) = result else {
                            panic!("line {line}: expected '{field}' to trap.");
                        };

                        assert_eq!(
                            err, ExecuteError::Trapped, 
                            "line {line}: expected error to be trap error, got {err:?}"
                        );
                    },

                    _ => unreachable!()
                }
            },

            Command::AssertUninstantiable { line, filename } => {
                let bytes = fs::read(dir.join(filename))
                    .unwrap_or_else(|e| panic!("line {line}: failed to read {filename}: {e}"));
                
                let module = Module::decode_from_bytes(&bytes)
                    .unwrap_or_else(|e| panic!("line {line}: failed to decode {filename}: {e:?}"));

                module.validate()
                    .unwrap_or_else(|e| panic!("line {line}: failed to validate {filename}: {e:?}"));

                let imports = spectest_imports(&module, &spectest_exports, &registered_modules, filename, line).unwrap();
                let result = module.instantiate(&mut store, imports);

                assert!(result.is_err(), "line {line}: expected module instantiation (uninstantiable) to fail, got success.");
            },

            Command::AssertMalformed { line, filename, module_type } => {
                if module_type != "binary" { continue; }

                let bytes = fs::read(dir.join(filename))
                    .unwrap_or_else(|e| panic!("line {line}: failed to read {filename}: {e}"));
                
                let result = Module::decode_from_bytes(&bytes);
                
                assert!(result.is_err(), "line {line}: expected module decoding to fail, got success.");
            },

            Command::AssertInvalid { line, filename, module_type } => {
                if module_type != "binary" { continue; }

                let bytes = fs::read(dir.join(filename))
                    .unwrap_or_else(|e| panic!("line {line}: failed to read {filename}: {e}"));
                
                let module = Module::decode_from_bytes(&bytes)
                    .unwrap_or_else(|e| panic!("line {line}: failed to decode {filename}: {e:?}"));

                let result = module.validate();

                assert!(result.is_err(), "line {line}: expected module validation to fail, got success.");
            },

            Command::AssertUnlinkable { line, filename, module_type } => {
                if module_type != "binary" { continue; }

                let bytes = fs::read(dir.join(filename))
                    .unwrap_or_else(|e| panic!("line {line}: failed to read {filename}: {e}"));
                
                let module = Module::decode_from_bytes(&bytes)
                    .unwrap_or_else(|e| panic!("line {line}: failed to decode {filename}: {e:?}"));

                module.validate()
                    .unwrap_or_else(|e| panic!("line {line}: failed to validate {filename}: {e:?}"));

                // either the import lookup or module instantiation should fail
                let result = spectest_imports(&module, &spectest_exports, &registered_modules, filename, line)
                    .and_then(|imports| {
                        module.instantiate(&mut store, imports)
                            .map_err(|e| format!("{e:?}"))
                    });

                assert!(result.is_err(), "line {line}: expected module instantiation (unlinkable) to fail, got success.");
            },

            _ => {}
        }
    }
}

/// Tests the entire WebAssembly 1.0 spec test suite.
#[test]
fn run_spec_test_suite() {
    let child = std::thread::Builder::new()
        .stack_size(STACK_SIZE_MB * 1024 * 1024)
        .spawn(|| {
            common::convert_wasts();

            let dir = fs::read_dir(Path::new(common::SPEC_OUTPUT_DIR))
                .expect(&format!("{} should exist.", common::SPEC_OUTPUT_DIR));
            
            for entry in dir {
                let path = entry.unwrap().path();

                if path.extension().map_or(false, |e| e == "json") {
                    run_spec_test(&path);
                }
            }
        })
        .unwrap();
    
    child.join().unwrap();
}