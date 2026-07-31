mod instructions;
mod types;

pub use instructions::{Expr, Instr};

pub use types::{
    ValType, FuncType, Func,
    Table, TableType, ElemType,
    Mem, Limits,
    Global, GlobalType, Mutability,
    Elem, Data,
    Import, ImportDesc,
    Export, ExportDesc,
};