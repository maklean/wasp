use std::rc::Rc;
use crate::definitions::{Func, FuncType, Mutability};

/// Runtime representation of a Wasm value.
pub enum Val {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64)
}

/// Outcome of a computation.
pub enum ExecutionResult {
    Val(Vec<Val>),
    Trap
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

/// Runtime representation of a Wasm function.
pub enum FuncInstance {
    /// A function defined inside a Wasm module.
    Wasm {
        /// Signature of the function.
        func_type: FuncType,

        /// Originating module (used to resolve references to other definitions).
        module: Rc<ModuleInstance>,

        /// Function definition.
        code: Func,
    },

    /// A function provided by the host (import).
    Host {
        /// Signature of the function.
        func_type: FuncType,

        /// Host function.
        code: HostFunc,
    }
}

/// Runtime representation of a Wasm table.
pub struct TableInstance {
    /// Function elements.
    pub elem: Vec<FuncElem>,

    /// Maximum size.
    pub max: Option<u32>,
}

/// Runtime representation of a Wasm linear memory.
pub struct MemInstance {
    /// Bytes.
    pub data: Vec<u8>,

    /// Maximum size.
    pub max: Option<u32>,
}

/// Runtime representation of a Wasm global variable.
pub struct GlobalInstance {
    /// Runtime value.
    pub value: Val,

    /// Mutability.
    pub mutability: Mutability,
}

/// Runtime representation of a module.
pub struct ModuleInstance {
    /// Signature of functions.
    pub types: Vec<FuncType>,

    /// Addresses of functions.
    pub func_addrs: Vec<Addr>,

    /// Addresses of tables.
    pub table_addrs: Vec<Addr>,

    /// Addresses of linear memories.
    pub mem_addrs: Vec<Addr>,

    /// Addresses of global variables.
    pub global_addrs: Vec<Addr>,

    /// Module's exports.
    pub exports: Vec<ExportInstance>,
}

/// Runtime representation of a Wasm export.
pub struct ExportInstance {
    /// Name of export.
    pub name: String,

    /// Value of export.
    pub value: ExternVal,
}

/// Optional address to a function.
pub type FuncElem = Option<Addr>;

/// Wasm address (basically an index).
pub type Addr = usize;

/// Wasm module export value.
pub enum ExternVal {
    Func(Addr),
    Table(Addr),
    Mem(Addr),
    Global(Addr),
}

/// Runtime function call frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct Frame {
    /// Number of values the function returns.
    pub arity: usize,

    /// Where the function's locals begin in the `locals` stack.
    pub locals_start: usize,

    /// Where the function's values begin in the `values` stack.
    pub values_start: usize,
}

/// Runtime structured control construct (block/loop).
#[derive(Debug, Default, Clone, Copy)]
pub struct Block {
    /// Number of values this block/loop produces.
    pub arity: usize,

    /// `values` stack height once we entered this block/loop.
    pub values_start: usize,
}

/// Host function (leaving this for later)
pub struct HostFunc;

pub struct Executor {
    pub values: Vec<Val>,
    pub locals: Vec<Val>,
    pub current_frame: Frame,
    pub current_block: Block,
}