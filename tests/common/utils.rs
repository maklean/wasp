use std::{collections::HashMap, rc::Rc};

use wasp::{executor::{ModuleInstance, Val}};

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

/// Looks up the module in the registered modules map, uses current instance as a fallback.
pub fn resolve_module(
    module: &Option<String>, 
    current_instance: &Option<Rc<ModuleInstance>>, 
    registered_modules: &HashMap<String, Rc<ModuleInstance>>, 
    line: u32
) -> Rc<ModuleInstance> {
    let instance = match module {
        Some(module) => registered_modules.get(module)
            .unwrap_or_else(|| panic!("line {line}: module '{module}' is not registered.")),

        None => current_instance.as_ref()
            .unwrap_or_else(|| panic!("line {line}: no module loaded"))
    };

    Rc::clone(instance)
}