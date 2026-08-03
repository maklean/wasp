use std::{collections::HashMap, fs, path::Path, rc::Rc};

use wasp::{errors::{ExecutionError, TrapReason}, module::Module, runtime::{ExternVal, ModuleInstance, Store, Val}};

use crate::common::{Action, Command, Manifest, SPEC_OUTPUT_DIR, convert_wasts, register_spectest, resolve_imports, resolve_module, vals_match};

mod common;

/// Rust stack size for spec tests (in MB).
const STACK_SIZE_MB: usize = 256;

/// Runs a specific Wasm 1.0 spec test from its manifest (.json) file.
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
                // decode, validate, and instantiate the module with the given file name, then set it as the current instance.
                let mut module = Module::decode_from_file(dir.join(filename))
                    .unwrap_or_else(|e| panic!("line {line}: failed to decode {filename}: {e:?}"));
        
                module.validate()
                    .unwrap_or_else(|e| panic!("line {line}: failed to validate {filename}: {e:?}"));

                let imports = resolve_imports(&module, &spectest_exports, &registered_modules, filename, line)
                    .unwrap_or_else(|_| panic!("[{}] should be able to resolve module imports", manifest_path.display()));

                let instance = module.instantiate(&mut store, &imports)
                    .unwrap_or_else(|e| panic!("line {line}: failed to instantiate {filename}: {e:?}"));

                if !name.is_empty() {
                    registered_modules.insert(name.clone(), Rc::clone(&instance));
                }

                current_instance = Some(instance);
            },
            
            Command::Register { line, name, as_ } => {
                // register module instance with the name 'name' as 'as_', if name is empty use current instance instead.
                let instance = if name.is_empty() {
                    current_instance.as_ref()
                        .unwrap_or_else(|| panic!("[{}] line {line}: no current module to register", manifest_path.display()))
                } else {
                    registered_modules.get(name)
                        .unwrap_or_else(|| panic!("[{}] line {line}: module '{name}' should be registered already.", manifest_path.display()))
                };

                registered_modules.insert(as_.to_string(), Rc::clone(instance));
            },

            Command::Action { action, line } => {
                match action {
                    Action::Invoke { module, field, args } => {
                        // call the function exported as 'field' with the given arguments.
                        let instance = resolve_module(module, &current_instance, &registered_modules, *line, manifest_path);
                    
                        let args: Vec<Val> = args
                            .iter()
                            .map(|arg_val| arg_val.clone().try_into().unwrap())
                            .collect();

                        instance.invoke_exported_function(field, &args, &mut store)
                            .unwrap_or_else(|e| panic!("[{}] line {line}: expected success, got {e:?}", manifest_path.display()));
                    },

                    _ => unreachable!("Command::Action should only happen for function invocation.")
                }
            },

            Command::AssertReturn { action, expected, line } => {
                let expected_vals: Vec<Val> = expected
                    .iter()
                    .map(|arg_val| arg_val.clone().try_into().unwrap())
                    .collect();

                match action {
                    Action::Invoke { module, field, args } => {
                        // call the function exported as 'field' with the given arguments, test against expected results.
                        let instance = resolve_module(module, &current_instance, &registered_modules, *line, manifest_path);

                        let args: Vec<Val> = args
                            .iter()
                            .map(|arg_val| arg_val.clone().try_into().unwrap())
                            .collect();

                        let actual_vals = instance.invoke_exported_function(field, &args, &mut store)
                            .unwrap_or_else(|e| panic!("[{}] line {line}: expected success, got {e:?}", manifest_path.display()));

                        assert!(
                            vals_match(&actual_vals, &expected_vals),
                            "[{}] line {line}: expected {expected_vals:?}, got {actual_vals:?}", manifest_path.display()
                        )
                    },

                    Action::Get { module, field } => {
                        // get the global exported as 'field''s value, test against expected result.
                        let instance = resolve_module(module, &current_instance, &registered_modules, *line, manifest_path);

                        let export = instance.exports
                            .iter()
                            .find(|e| e.name == *field)
                            .unwrap_or_else(|| panic!("[{}] line {line}: export '{field}' not found", manifest_path.display()));

                        let ExternVal::Global(global_addr) = export.value else {
                            panic!("[{}] line {line}: export '{field}' is not a global", manifest_path.display());
                        };

                        let value = store.globals[global_addr].value;

                        assert!(
                            vals_match(&[value], &expected_vals),
                            "[{}] line {line}: expected {expected_vals:?}, got [{value:?}]", manifest_path.display()
                        );
                    }
                }
            },

            Command::AssertExhaustion { action, line } => {
                match action {
                    Action::Invoke { module, field, args } => {
                        // call function exported as 'field' with the given arguments, expect call stack exhaustion error.
                        let instance = resolve_module(module, &current_instance, &registered_modules, *line, manifest_path);

                        let args: Vec<Val> = args
                            .iter()
                            .map(|arg_val| arg_val.clone().try_into().unwrap())
                            .collect();

                        let result = instance.invoke_exported_function(field, &args, &mut store);

                        let Err(err) = result else {
                            panic!("[{}] line {line}: expected '{field}' to exhaust call stack.", manifest_path.display());
                        };

                        assert_eq!(
                            err, ExecutionError::Trapped(TrapReason::CallStackExhausted), 
                            "[{}] line {line}: expected error to be exhausted call stack error, got {err:?}", manifest_path.display()
                        );
                    },

                    _ => unreachable!("Command::AssertExhaustion should only happen for function invocation.")
                }
            },

            Command::AssertTrap { action, line } => {
                match action {
                    Action::Invoke { module, field, args } => {
                        // call function exported as 'field' with the given arguments, expect trap error.
                        let instance = resolve_module(module, &current_instance, &registered_modules, *line, manifest_path);

                        let args: Vec<Val> = args
                            .iter()
                            .map(|arg_val| arg_val.clone().try_into().unwrap())
                            .collect();

                        let result = instance.invoke_exported_function(field, &args, &mut store);

                        let Err(err) = result else {
                            panic!("[{}] line {line}: expected '{field}' to trap.", manifest_path.display());
                        };

                        match err {
                            ExecutionError::Trapped(_) => {},
                            _ => panic!("[{}] line {line}: expected error to be trap error, got {err:?}", manifest_path.display())
                        }
                    },

                    _ => unreachable!("Command::AssertTrap should only happen for function invocation.")
                }
            },

            Command::AssertUninstantiable { line, filename } => {
                // decode, validate, then try to instantiate module (should fail)
                let mut module = Module::decode_from_file(dir.join(filename))
                    .unwrap_or_else(|e| panic!("line {line}: failed to decode {filename}: {e:?}"));
        
                module.validate()
                    .unwrap_or_else(|e| panic!("line {line}: failed to validate {filename}: {e:?}"));

                let imports = resolve_imports(&module, &spectest_exports, &registered_modules, filename, line)
                    .unwrap_or_else(|_| panic!("[{}] should be able to resolve module imports", manifest_path.display()));

                let result = module.instantiate(&mut store, &imports);

                assert!(result.is_err(), "[{}] line {line}: expected module instantiation (uninstantiable) to fail, got success.", manifest_path.display());
            },

            Command::AssertMalformed { line, filename, module_type } => {
                // try to decode module (should fail)
                if module_type != "binary" { continue; }

                let result = Module::decode_from_file(dir.join(filename));

                assert!(result.is_err(), "[{}] line {line}: expected module decoding to fail, got success.", manifest_path.display());
            },

            Command::AssertInvalid { line, filename, module_type } => {
                // decode, then try to validate module (should fail)
                if module_type != "binary" { continue; }

                let mut module = Module::decode_from_file(dir.join(filename))
                    .unwrap_or_else(|e| panic!("line {line}: failed to decode {filename}: {e:?}"));
        
                let result = module.validate();

                assert!(result.is_err(), "[{}] line {line}: expected module validation to fail, got success.", manifest_path.display());
            },

            Command::AssertUnlinkable { line, filename, module_type } => {
                // decode, validate, then either the import lookup or module instantiation should fail.
                if module_type != "binary" { continue; }

                let mut module = Module::decode_from_file(dir.join(filename))
                    .unwrap_or_else(|e| panic!("line {line}: failed to decode {filename}: {e:?}"));
        
                module.validate()
                    .unwrap_or_else(|e| panic!("line {line}: failed to validate {filename}: {e:?}"));

                let result = resolve_imports(&module, &spectest_exports, &registered_modules, filename, line)
                    .and_then(|imports| {
                        module.instantiate(&mut store, &imports)
                            .map_err(|e| format!("{e:?}"))
                    });
                
                assert!(result.is_err(), "[{}] line {line}: expected module instantiation (unlinkable) to fail, got success.", manifest_path.display());
            }

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
            convert_wasts();

            let dir = fs::read_dir(Path::new(SPEC_OUTPUT_DIR))
                .expect(&format!("{} should exist.", SPEC_OUTPUT_DIR));
            
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