use std::rc::Rc;
use crate::{definitions::{Func, FuncType, Mutability}, instructions::Instr};

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

/// Stack.
pub struct Stack {
    /// Operands of instructions.
    pub values: Vec<Val>,

    /// Active structured control instructions.
    pub labels: Vec<Label>,

    /// Call frames of active function calls.
    pub activations: Vec<Activation>,
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

/// Runtime representation of a structure control construct label.
pub struct Label {
    /// Argument arity - number of values passed to the instruction sequence.
    pub arity: u32,

    /// Instruction sequence - instructions to execute when this label is branched to.
    pub instructions: Vec<Instr>,
}

/// Runtime activation of a function call.
pub struct Activation {
    /// Return arity of the function (at most one in Wasm 1.0).
    pub arity: u32,

    /// Function call frame.
    pub frame: Frame,
}

/// Runtime function call frame.
pub struct Frame {
    /// Function arguments and local variables.
    pub locals: Vec<Val>,

    /// Reference to module instance.
    pub module: Rc<ModuleInstance>,
}

/// Host function (leaving this for later)
pub struct HostFunc;