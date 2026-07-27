use std::rc::Rc;

use wasp::executor::{Addr, ExternVal, FuncInstance, GlobalInstance, HostFunc, MemInstance, PAGE_SIZE, Store, TableInstance, Val};
use wasp::definitions::{FuncType, Mutability, ValType};
use wasp::module::Module;

/// Contains the addresses of the spectest module's exports.
pub struct SpecTestExports {
    pub global_i32: Addr,
    pub global_i64: Addr,
    pub global_f32: Addr,
    pub global_f64: Addr,
    pub memory: Addr,
    pub table: Addr,
    pub print: Addr,
    pub print_i32: Addr,
    pub print_i64: Addr,
    pub print_f32: Addr,
    pub print_f64: Addr,
    pub print_i32_f32: Addr,
    pub print_f64_f64: Addr,
}

/// Adds the 'spectest' module's exports to the given store. Returns a `SpecTestExports`
pub fn register_spectest(store: &mut Store) -> SpecTestExports {
    let global_i32 = store.globals.len();
    store.globals.push(GlobalInstance { value: Val::I32(666), mutability: Mutability::Const });

    let global_i64 = store.globals.len();
    store.globals.push(GlobalInstance { value: Val::I64(666), mutability: Mutability::Const });

    let global_f32 = store.globals.len();
    store.globals.push(GlobalInstance { value: Val::F32(666.6), mutability: Mutability::Const });

    let global_f64 = store.globals.len();
    store.globals.push(GlobalInstance { value: Val::F64(666.6), mutability: Mutability::Const });

    let memory = store.mems.len();
    store.mems.push(MemInstance { data: vec![0u8; PAGE_SIZE], max: Some(2) });

    let table = store.tables.len();
    store.tables.push(TableInstance { elem: vec![None; 10], max: Some(20) });

    // all print functions share the same implementation
    let print_host_func = Rc::new(HostFunc::new(|_store, _args| Ok(vec![])));

    let print = store.funcs.len();
    store.funcs.push(FuncInstance::Host { 
        func_type: Rc::new(FuncType {
            params: vec![],
            results: vec![],
        }), 
        code: Rc::clone(&print_host_func)
    });

    let print_i32 = store.funcs.len();
    store.funcs.push(FuncInstance::Host { 
        func_type: Rc::new(FuncType {
            params: vec![ValType::I32],
            results: vec![],
        }), 
        code: Rc::clone(&print_host_func)
    });

    let print_i64 = store.funcs.len();
    store.funcs.push(FuncInstance::Host { 
        func_type: Rc::new(FuncType {
            params: vec![ValType::I64],
            results: vec![],
        }), 
        code: Rc::clone(&print_host_func)
    });

    let print_f32 = store.funcs.len();
    store.funcs.push(FuncInstance::Host { 
        func_type: Rc::new(FuncType {
            params: vec![ValType::F32],
            results: vec![],
        }), 
        code: Rc::clone(&print_host_func)
    });

    let print_f64 = store.funcs.len();
    store.funcs.push(FuncInstance::Host { 
        func_type: Rc::new(FuncType {
            params: vec![ValType::F64],
            results: vec![],
        }), 
        code: Rc::clone(&print_host_func)
    });

    let print_i32_f32 = store.funcs.len();
    store.funcs.push(FuncInstance::Host { 
        func_type: Rc::new(FuncType {
            params: vec![ValType::I32, ValType::F32],
            results: vec![],
        }), 
        code: Rc::clone(&print_host_func)
    });

    let print_f64_f64 = store.funcs.len();
    store.funcs.push(FuncInstance::Host { 
        func_type: Rc::new(FuncType {
            params: vec![ValType::F64, ValType::F64],
            results: vec![],
        }), 
        code: Rc::clone(&print_host_func)
    });

    SpecTestExports { global_i32, global_i64, global_f32, global_f64, memory, table, print, print_i32, print_i64, print_f32, print_f64, print_i32_f32, print_f64_f64 }
}

/// Returns the required imports from the spectest module.
pub fn spectest_imports(module: &Module, store: &Store) -> Vec<ExternVal> {
    todo!()
}