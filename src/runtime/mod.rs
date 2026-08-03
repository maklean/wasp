use std::rc::Rc;

use crate::{binary::ParsedModule, errors::{EmbedderImportMismatchReason, ExecutionError, TrapReason}, execution::Executor, module::Module, structure::{ExportDesc, Func, FuncType, ImportDesc, Mem, ValType}};

mod types;

pub use types::{
    Addr, Val, FuncElem,
    FuncInstance, HostFunc,
    TableInstance, MemInstance, GlobalInstance,
    ExportInstance, ExternVal,
};

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

impl ModuleInstance {
    /// Instantiates a valid module.
    pub(crate) fn instantiate(
        module: &Module,
        store: &mut Store,
        imports: &[ExternVal]
    ) -> Result<Rc<Self>, ExecutionError> {
        if !module.valid {
            panic!("cannot call ModuleInstance::instantiate() on an invalid module.");
        }

        let module = &module.parsed;

        Self::verify_embedder_imports(module, store, &imports)?;

        // imported funcs + module-defined function addresses
        let func_addrs: Vec<Addr> = imports
            .iter()
            .filter_map(|import| {
                match import {
                    ExternVal::Func(addr) => Some(*addr),
                    _ => None
                }
            })
            .chain(
                store.funcs.len()..store.funcs.len() + module.funcs.len()
            )
            .collect();

        // imported tables + module-defined table addresses
        let table_addrs: Vec<Addr> = imports
            .iter()
            .filter_map(|import| {
                match import {
                    ExternVal::Table(addr) => Some(*addr),
                    _ => None
                }
            })
            .chain(
                store.tables.len()..store.tables.len() + module.tables.len()
            )
            .collect();

        // imported mems + module-defined mem addresses
        let mem_addrs: Vec<Addr> = imports
            .iter()
            .filter_map(|import| {
                match import {
                    ExternVal::Mem(addr) => Some(*addr),
                    _ => None
                }
            })
            .chain(
                store.mems.len()..store.mems.len() + module.mems.len()
            )
            .collect();

        // imported globals + module-defined global addresses
        let imported_global_addrs: Vec<Addr> = imports
            .iter()
            .filter_map(|import| {
                match import {
                    ExternVal::Global(addr) => Some(*addr),
                    _ => None
                }
            })
            .collect();

        let global_addrs: Vec<Addr> = imported_global_addrs
            .iter()
            .copied()
            .chain(
                store.globals.len()..store.globals.len() + module.globals.len()
            )
            .collect();
            
        let exports: Vec<ExportInstance> = module.exports
            .iter()
            .map(|export| {
                let value = match export.desc {
                    ExportDesc::Func(func_idx) => ExternVal::Func(func_addrs[func_idx as usize]),
                    ExportDesc::Table(table_idx) => ExternVal::Table(table_addrs[table_idx as usize]),
                    ExportDesc::Mem(mem_idx) => ExternVal::Mem(mem_addrs[mem_idx as usize]),
                    ExportDesc::Global(global_idx) => ExternVal::Global(global_addrs[global_idx as usize]),
                };

                ExportInstance { name: export.name.clone(), value }
            })
            .collect();
        
        let this = Rc::new(Self {
            types: module.types.clone(),
            func_addrs,
            table_addrs,
            mem_addrs,
            global_addrs,
            exports
        });

        // add module-defined function instances to the store
        for func in &module.funcs {
            store.funcs.push(FuncInstance::Wasm {
                func_type: Rc::new(this.types[func.type_idx as usize].clone()),
                module: Rc::clone(&this),
                code: Rc::new(Func {
                    type_idx: func.type_idx,
                    locals: func.locals.clone(),
                    body: func.body.clone()
                })
            });
        }

        // NOTE: technically there's only at most one table or memory, doing a loop in-case I decide to update the runtime to Wasm 2.0

        // add module-defined tables to the store
        for table in &module.tables {
            store.tables.push(TableInstance { 
                elem: vec![None; table.table_type.limits.min as usize], 
                max: table.table_type.limits.max
            });
        }

        // add module-defined memories to the store
        for mem in &module.mems {
            store.mems.push(MemInstance { 
                data: vec![0u8; mem.mem_type.min as usize * Mem::PAGE_SIZE],
                max: mem.mem_type.max
            });
        }

        // add module-defined globals to the store
        for global in &module.globals {
            store.globals.push(GlobalInstance {
                value: global.init.execute_const_expr(store, &imported_global_addrs),
                mutability: global.global_type.mutability,
            });
        }

        // check that element and memory segments fit in the table/mem before adding any
        for elem in &module.elem {
            let offset = elem.offset.execute_const_expr(store, &imported_global_addrs).as_i32();
            let offset = usize::try_from(offset).map_err(|_| ExecutionError::Trapped(TrapReason::UndefinedElement { index: elem.table_idx as usize }))?;

            let table = &mut store.tables[this.table_addrs[elem.table_idx as usize]];

            // check if segment fits in table
            let end = offset.checked_add(elem.init.len())
                .ok_or(ExecutionError::Trapped(TrapReason::UndefinedElement { index: elem.table_idx as usize }))?;

            if end > table.elem.len() {
                return Err(ExecutionError::Trapped(TrapReason::UndefinedElement { index: elem.table_idx as usize }));
            }
        }

        for data in &module.data {
            let offset = data.offset.execute_const_expr(store, &imported_global_addrs).as_i32();
            let mem_addr = this.mem_addrs[data.mem_idx as usize];
            let mem = &mut store.mems[mem_addr];

            let offset = usize::try_from(offset)
                .map_err(|_| ExecutionError::Trapped(TrapReason::OutOfBoundsMemoryAccess {
                    addr: 0, len: data.init.len(), mem_size: mem.data.len()
                }))?;

            // check if bytes fit in memory
            let end = offset.checked_add(data.init.len())
                .ok_or(ExecutionError::Trapped(TrapReason::OutOfBoundsMemoryAccess {
                    addr: offset, len: data.init.len(), mem_size: mem.data.len()
                }))?;

            if end > mem.data.len() {
                return Err(ExecutionError::Trapped(TrapReason::OutOfBoundsMemoryAccess {
                    addr: offset, len: data.init.len(), mem_size: mem.data.len()
                }));
            }
        }

        // initialize table/mem with element and data segments
        for elem in &module.elem {
            let offset = elem.offset.execute_const_expr(store, &imported_global_addrs).as_i32() as usize;
            let table = &mut store.tables[this.table_addrs[elem.table_idx as usize]];

            // write indices into table
            for (i, func_idx) in elem.init.iter().enumerate() {
                table.elem[offset+i] = Some(this.func_addrs[*func_idx as usize]);
            }
        }

        for data in &module.data {
            let offset = data.offset.execute_const_expr(store, &imported_global_addrs).as_i32() as usize;
            let mem = &mut store.mems[this.mem_addrs[data.mem_idx as usize]];

            // write bytes to memory
            let end = offset + data.init.len();
            mem.data[offset..end].copy_from_slice(&data.init);
        }

        // run start function if present
        if let Some(start_idx) = module.start {
            let mut executor = Executor::with_args(vec![]);
            executor.execute_function(this.func_addrs[start_idx as usize], store)?;
        }

        Ok(this)
    }

    /// Verifies the imports passed by the embedder against the module's imports.
    fn verify_embedder_imports(module: &ParsedModule, store: &Store, imports: &[ExternVal]) -> Result<(), ExecutionError> {
        if module.imports.len() != imports.len() {
            return Err(ExecutionError::EmbedderImportCountMismatch { 
                module_count: module.imports.len(), 
                embedder_count: imports.len() 
            });
        }

        for (module_import, embedder_import) in module.imports.iter().zip(imports.iter()) {
            let mismatch = |reason: EmbedderImportMismatchReason| ExecutionError::EmbedderImportTypeMismatch {
                module: module_import.module.clone(),
                name: module_import.name.clone(),
                reason,
            };

            match (&module_import.desc, embedder_import) {
                (ImportDesc::Func(type_idx), ExternVal::Func(addr)) => {
                    // check for func signature mismatch
                    let expected_sig = &module.types[*type_idx as usize];

                    let actual_sig = match &store.funcs[*addr] {
                        FuncInstance::Host { func_type, .. } => func_type,
                        FuncInstance::Wasm { func_type, .. } => func_type,
                    };

                    if expected_sig.params != actual_sig.params || expected_sig.results != actual_sig.results {
                        return Err(mismatch(EmbedderImportMismatchReason::FuncSignature {
                            expected: expected_sig.clone(),
                            actual: (**actual_sig).clone(),
                        }));
                    }
                },

                (ImportDesc::Table(expected_table), ExternVal::Table(table_addr)) => {
                    let actual_table = &store.tables[*table_addr];

                    // actual table size must be at least the declared min
                    if expected_table.limits.min > actual_table.elem.len() as u32 {
                        return Err(mismatch(EmbedderImportMismatchReason::TableTooSmall {
                            expected_min: expected_table.limits.min,
                            actual_size: actual_table.elem.len() as u32,
                        }));
                    }

                    // if there's a declared max, the actual table max should be at most the declared max
                    if let Some(expected_max) = expected_table.limits.max {
                        if actual_table.max.is_none_or(|actual_max| actual_max > expected_max) {
                            return Err(mismatch(EmbedderImportMismatchReason::TableMaxTooLarge {
                                expected_max,
                                actual_max: actual_table.max,
                            }));
                        }
                    }
                },

                (ImportDesc::Mem(expected_mem), ExternVal::Mem(mem_addr)) => {
                    let actual_mem = &store.mems[*mem_addr];
                    let actual_pages = (actual_mem.data.len() / Mem::PAGE_SIZE) as u32;

                    // actual mem size must be at least the declared min
                    if expected_mem.min > actual_pages {
                        return Err(mismatch(EmbedderImportMismatchReason::MemTooSmall {
                            expected_min: expected_mem.min,
                            actual_pages,
                        }));
                    }

                    // if there's a declared max, the actual mem max should be at most the declared max
                    if let Some(expected_max) = expected_mem.max {
                        if actual_mem.max.is_none_or(|actual_max| actual_max > expected_max) {
                            return Err(mismatch(EmbedderImportMismatchReason::MemMaxTooLarge {
                                expected_max,
                                actual_max: actual_mem.max,
                            }));
                        }
                    }
                },

                (ImportDesc::Global(expected_global), ExternVal::Global(global_addr)) => {
                    let actual_global = &store.globals[*global_addr];
                    let actual_val_type: ValType = actual_global.value.into();

                    // should be the same val type and mutability
                    if expected_global.val_type != actual_val_type {
                        return Err(mismatch(EmbedderImportMismatchReason::GlobalTypeMismatch {
                            expected: expected_global.val_type,
                            actual: actual_val_type,
                        }));
                    }

                    if expected_global.mutability != actual_global.mutability {
                        return Err(mismatch(EmbedderImportMismatchReason::GlobalMutabilityMismatch {
                            expected: expected_global.mutability,
                            actual: actual_global.mutability,
                        }));
                    }
                },

                _ => return Err(mismatch(EmbedderImportMismatchReason::Kind)),
            }
        }

        Ok(())
    }

    /// Calls the given exported function by name on the current module instance.
    pub fn invoke_exported_function(&self, name: &str, args: &[Val], store: &mut Store) -> Result<Vec<Val>, ExecutionError> {
        // find exported function from module exports
        let export = self.exports
            .iter()
            .find(|export| export.name == name)
            .unwrap_or_else(|| panic!("export '{}' not found in module.", name));

        let ExternVal::Func(func_addr) = export.value else {
            panic!("export '{}' is not a function.", name);
        };

        let mut executor = Executor::with_args(args.to_vec());
        executor.execute_function(func_addr, store)?;

        // return function call results
        Ok(executor.values)
    }
}

/// Global state of a Wasm program.
#[derive(Default)]
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
        Default::default()
    }
}