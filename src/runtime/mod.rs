use crate::{runtime::types::{Addr, ExportInstance, FuncInstance, GlobalInstance, MemInstance, TableInstance}, structure::FuncType};

mod types;

/// Runtime representation of a Wasm module.
pub struct ModuleInstance {
    /// Module-defined function signatures.
    pub types: Vec<FuncType>,

    /// Addresses of imported + module-defined functions.
    pub func_addrs: Vec<Addr>,

    /// Addresses of imported + module-defined tables.
    pub table_addrs: Vec<Addr>,

    /// Addresses of imported + module-defined linear memories.
    pub mem_addrs: Vec<Addr>,

    /// Addresses of imported + module-defined global variables.
    pub global_addrs: Vec<Addr>,

    /// Module's exports.
    pub exports: Vec<ExportInstance>,
}

/// Global state of a Wasm program.
pub struct Store {
    /// Function instances.
    pub funcs: Vec<FuncInstance>,

    /// Table instances.
    pub tables: Vec<TableInstance>,

    /// Memory instances.
    pub mems: Vec<MemInstance>,

    /// Global variable instances.
    pub globals: Vec<GlobalInstance>,
}

impl Store {
    pub fn new() -> Self {
        Self { 
            funcs: Vec::new(), 
            tables: Vec::new(), 
            mems: Vec::new(), 
            globals: Vec::new() 
        }
    }
}