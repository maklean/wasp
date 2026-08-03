use std::{collections::HashMap, path::Path, rc::Rc};

use wasp::{module::Module, runtime::{ExternVal, ModuleInstance, Val}};

use crate::common::spectest::SpecTestExports;

/// Returns whether the two collection of values are the same.
pub fn vals_match(a: &[Val], b: &[Val]) -> bool {
    if a.len() != b.len() { return false; }

    a.iter().zip(b).all(|(va, vb)| match (va, vb) {
        (Val::I32(va), Val::I32(vb)) => va == vb,
        (Val::I64(va), Val::I64(vb)) => va == vb,
        (Val::F32(va), Val::F32(vb)) => (va.is_nan() && vb.is_nan()) || va.to_bits() == vb.to_bits(),
        (Val::F64(va), Val::F64(vb)) => (va.is_nan() && vb.is_nan()) || va.to_bits() == vb.to_bits(),
        _ => false,
    })
}

/// Returns the imports required from any currently registered modules (incl. spectest).
pub fn resolve_imports(
    module: &Module,
    spectest_exports: &SpecTestExports,
    registered_modules: &HashMap<String, Rc<ModuleInstance>>,
    filename: &String,
    line: &u32
) -> Result<Vec<ExternVal>, String> {
    module.parsed.imports
        .iter()
        .map(|import| {
            if import.module == "spectest" {
                match import.name.as_str() {
                    "global_i32" => Ok(ExternVal::Global(spectest_exports.global_i32)),
                    "global_i64" => Ok(ExternVal::Global(spectest_exports.global_i64)),
                    "global_f32" => Ok(ExternVal::Global(spectest_exports.global_f32)),
                    "global_f64" => Ok(ExternVal::Global(spectest_exports.global_f64)),
                    "memory" => Ok(ExternVal::Mem(spectest_exports.memory)),
                    "table" => Ok(ExternVal::Table(spectest_exports.table)),
                    "print" => Ok(ExternVal::Func(spectest_exports.print)),
                    "print_i32" => Ok(ExternVal::Func(spectest_exports.print_i32)),
                    "print_i64" => Ok(ExternVal::Func(spectest_exports.print_i64)),
                    "print_f32" => Ok(ExternVal::Func(spectest_exports.print_f32)),
                    "print_f64" => Ok(ExternVal::Func(spectest_exports.print_f64)),
                    "print_i32_f32" => Ok(ExternVal::Func(spectest_exports.print_i32_f32)),
                    "print_f64_f64" => Ok(ExternVal::Func(spectest_exports.print_f64_f64)),
                    other => Err(format!("line {line}: unhandled spectest import: {other} in {filename}.")),
                }
            } else if let Some(registered_module) = registered_modules.get(&import.module) {
                registered_module.exports
                    .iter()
                    .find(|export| export.name == import.name)
                    .map(|export| export.value)
                    .ok_or_else(|| format!(
                        "line {line}: export '{}' not found in registered module '{}' ({filename}).",
                        import.name, import.module
                    ))
            } else {
                Err(format!("line {line}: unknown import module: {} in {filename}.", import.module))
            }
        })
        .collect() 
}

/// Looks up the module in the registered modules map, uses current instance as a fallback.
pub fn resolve_module(
    module: &Option<String>, 
    current_instance: &Option<Rc<ModuleInstance>>, 
    registered_modules: &HashMap<String, Rc<ModuleInstance>>, 
    line: u32,
    manifest_path: &Path,
) -> Rc<ModuleInstance> {
    let instance = match module {
        Some(module) => registered_modules.get(module)
            .unwrap_or_else(|| panic!("[{}] line {line}: module '{module}' is not registered.", manifest_path.display())),

        None => current_instance.as_ref()
            .unwrap_or_else(|| panic!("[{}] line {line}: no module loaded", manifest_path.display()))
    };

    Rc::clone(instance)
}