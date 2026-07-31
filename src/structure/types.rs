use crate::{binary::reader::Reader, errors::DecodingError};

use super::instructions::Expr;

/// Wasm value types.
pub enum ValType {
    I32,
    I64,
    F32,
    F64
}

impl ValType {
    pub(crate) fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        let actual = reader.read_byte()?;

        match actual {
            0x7f => Ok(Self::I32),
            0x7e => Ok(Self::I64),
            0x7d => Ok(Self::F32),
            0x7c => Ok(Self::F64),
            _ => Err(DecodingError::InvalidValType { actual }),
        }
    }
}

/// Wasm function signature.
pub struct FuncType {
    /// Function parameter types.
    pub params: Vec<ValType>,

    /// Function result types.
    pub results: Vec<ValType>
}

impl FuncType {
    /// Marker each `FuncType` should start with when decoding.
    const MARKER: u8 = 0x60;

    pub(crate) fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        // Sequence should start with a 0x60 marker
        reader.match_byte(Self::MARKER, DecodingError::InvalidFuncTypeMarker { pos: reader.pos(), actual: reader.peek_byte()? })?;

        // decode parameter and result types
        let param_count = reader.read_u32()? as usize;
        let params: Vec<ValType> = (0..param_count).into_iter()
            .map(|_| ValType::decode(reader))
            .collect::<Result<Vec<_>, _>>()?;

        let results_count = reader.read_u32()? as usize;
        let results: Vec<ValType> = (0..results_count).into_iter()
            .map(|_| ValType::decode(reader))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { params, results })
    }
}

/// Wasm module function.
pub struct Func {
    /// Index to the function's signature.
    pub type_idx: u32,

    /// Parameters + Local Variables.
    pub locals: Vec<ValType>,

    /// Instruction sequence.
    pub body: Expr,
}

/// Wasm table.
pub struct Table {
    /// Table's type details.
    pub table_type: TableType
}

/// Description/schema of a table.
pub struct TableType {
    /// Min (initial) and (optional) max size of the table.
    pub limits: Limits,

    /// Type of all elements in the table.
    pub elem_type: ElemType,
}

/// Types of elements in a Wasm table.
pub enum ElemType {
    /// Function reference/index.
    FuncRef,
}

/// Wasm linear memory.
pub struct Mem {
    /// Memory's size details.
    pub mem_type: Limits,
}

/// Details the minimum and (optional) maximum size of a definition (mainly for tables and linear memories).
pub struct Limits {
    /// Minimum size.
    pub min: u32,

    /// Maximum size (optional).
    pub max: Option<u32>,
}

/// Wasm global variable.
pub struct Global {
    /// Global's details.
    pub global_type: GlobalType,

    /// Constant initializer expression.
    pub init: Expr,
}

/// Description/schema of a global variable.
pub struct GlobalType {
    /// Type of the global's value.
    pub val_type: ValType,

    /// Mutability of the global.
    pub mutability: Mutability,
}

/// Mutabilities of data.
pub enum Mutability {
    /// Immutable.
    Const,

    /// Mutable.
    Var,
}

/// Wasm element segment.
pub struct Elem {
    /// Index of the targetted Wasm table.
    pub table_idx: u32, 

    /// Const. expression that evaluates to the offset
    /// into the targetted Wasm table to start writing
    /// at.
    pub offset: Expr,

    /// Indices to the Wasm functions to write into the
    /// targetted Wasm table.
    pub init: Vec<u32>
}

/// Wasm data segment.
pub struct Data {
    /// Index of the targetted Wasm memory.
    pub mem_idx: u32,

    /// Const. expression that evaluates to the offset
    /// into the targetted Wasm memory to start writing
    /// at.
    pub offset: Expr,

    /// Bytes to write into the targetted Wasm memory.
    pub init: Vec<u8>,
}

/// Wasm import.
pub struct Import {
    /// Name of the module the import is coming from.
    pub module: String,

    /// Name of the import.
    pub name: String,

    /// Type/Descriptor of the import.
    pub desc: ImportDesc,
}

impl Import {
    pub(crate) fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        // get module
        let module_len = reader.read_u32()? as usize;
        let mut pos = reader.pos();
        let module = std::str::from_utf8(reader.read_bytes(module_len)?)
            .map_err(|_| DecodingError::InvalidUTF8Name { pos })?
            .to_string();

        // get name
        let name_len = reader.read_u32()? as usize;
        pos = reader.pos();
        let name = std::str::from_utf8(reader.read_bytes(name_len)?)
            .map_err(|_| DecodingError::InvalidUTF8Name { pos })?
            .to_string();

        // get desc
        let desc = ImportDesc::decode(reader)?;

        Ok(Self { module, name, desc })
    }
}

/// Types of imports.
pub enum ImportDesc {
    /// Function index.
    Func(u32),

    /// Table type.
    Table(TableType),

    /// Memory type.
    Mem(Limits),

    /// Global type.
    Global(GlobalType),
}

/// Wasm export.
pub struct Export {
    /// Name of the export.
    pub name: String,

    /// Type/Descriptor of the export.
    pub desc: ExportDesc,
}

/// Types of exports
pub enum ExportDesc {
    /// Function index.
    Func(u32),

    /// Table index.
    Table(u32),

    /// Memory index.
    Mem(u32),

    /// Global index.
    Global(u32)
}