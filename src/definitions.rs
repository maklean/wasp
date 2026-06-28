/// Types that Wasm code can use for its values.
pub enum ValType {
    I32,
    I64,
    F32,
    F64
}

/// Signature of functions; maps vector of parameters to vector of results (in Wasm 1.0, there's only at most 1 result returned).
pub struct FuncType {
    /// Function parameters.
    pub params: Vec<ValType>,

    /// Function results.
    pub results: Vec<ValType>,
}

/// Wasm module's function outline.
pub struct Func {
    /// Index to the function's signature in the module's `types` vector.
    pub type_idx: u32,

    /// Vector of mutable local variables and their types, function parameters are the first elements in the vector.
    pub locals: Vec<ValType>,

    // /// Instruction sequence for the function.
    // pub body: Vec<Instr>,
}

/// Wasm module's table outline.
pub struct Table {
    /// Table's details.
    pub table_type: TableType,
}

/// Wasm module's linear memory outline.
pub struct Mem {
    /// Min (initial) and max size of the memory.
    pub mem_type: Limits,
}

/// Wasm module's global variable outline.
pub struct Global {
    /// Global's details.
    pub global_type: GlobalType,

    // /// Constant expression that initializes the global's value.
    // pub init: ConstExpr,
}

/// A Wasm module element segment outline (initializes a subrange of a table).
pub struct Elem {
    /// Index of the table in the module (should always be 0 since only one table is allowed per module in Wasm 1.0).
    pub table_idx: u32,

    // /// Offset into the table to start writing at.
    // pub offset: ConstExpr,

    /// Function indices to write into the table slots from the offset.
    pub init: Vec<u32>,
}

/// A wasm module data segment outline (initializes a subrange of a linear memory).
pub struct Data {
    /// Index of the memory in the module.
    pub mem_idx: u32,

    // /// Offset into the memory to start writing at.
    // pub offset: ConstExpr,

    /// Bytes to write into the memory slots from the offset.
    pub init: Vec<u8>,
}

/// Wasm module import outline.
pub struct Import {
    /// Module name.
    pub module: String,

    /// Import name.
    pub name: String,

    /// Type/Descriptor of import.
    pub desc: Desc,
}

/// Wasm module export outline.
pub struct Export {
    /// Export name (unique).
    pub name: String,

    /// Type/Descriptor of export.
    pub desc: Desc,
}

/// Description/schema of a table.
pub struct TableType {
    /// Min (initial) and (optional) max size of the table.
    pub limits: Limits,

    /// Type of all elements in the table.
    pub elem_type: ElemType,
}

/// Details the minimum and (optional) maximum size of a definition (mainly for tables and linear memories).
pub struct Limits {
    pub min: u32,
    pub max: Option<u32>,
}

/// Types of elements in a table (In Wasm 1.0, the only ElemType is a function reference / index to a Func).
pub enum ElemType {
    FuncRef,
}

/// Description/schema of a global variable.
pub struct GlobalType {
    /// Type of the global's value.
    pub val_type: ValType,

    /// Mutability of the global.
    pub mutable: Mutability,
}

/// Details the possible mutabilities of data.
pub enum Mutability {
    /// Immutable.
    Const,

    /// Mutable.
    Var,
}

/// Types of imports/exports.
pub enum Desc {
    /// Function index.
    Func(u32),

    /// Table index.
    Table(u32),

    /// Memory index.
    Mem(u32),

    /// Global Index.
    Global(u32),
}