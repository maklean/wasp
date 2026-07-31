use crate::{binary::reader::Reader, errors::{DecodingError, ValidationError}, validation::Validator};

use super::instructions::Expr;

/// Wasm value types.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ValType {
    I32,
    I64,
    F32,
    F64,
    Unknown,
}

impl ValType {
    pub fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
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
#[derive(Default)]
pub struct Func {
    /// Index to the function's signature.
    pub type_idx: u32,

    /// Parameters + Local Variables.
    pub locals: Vec<ValType>,

    /// Instruction sequence.
    pub body: Expr,
}

impl Func {
    /// Maximum number of locals allowed per function body.
    const MAX_LOCALS_PER_FUNC: usize = u16::MAX as usize;

    /// Returns `Func` with only the `type_idx` decoded.
    pub(crate) fn decode_type_idx(reader: &mut Reader) -> Result<Self, DecodingError> {
        Ok(Self {
            type_idx: reader.read_u32()?,
            ..Default::default()
        })
    }

    /// Decodes/Sets the `locals` and `body` of an existing `Func`.
    pub(crate) fn decode_locals_body(&mut self, reader: &mut Reader) -> Result<(), DecodingError> {
        let num_local_groups = reader.read_u32()? as usize;

        for _ in 0..num_local_groups {
            let num_locals = reader.read_u32()? as usize;

            let new_len = self.locals.len()
                .checked_add(num_locals)
                .filter(|&len| len <= Self::MAX_LOCALS_PER_FUNC)
                .ok_or(DecodingError::TooManyLocals)?;

            // add locals with the group val type to function locals
            let local_val_type = ValType::decode(reader)?;
            self.locals.resize(new_len, local_val_type);     
        }

        self.body = Expr::decode(reader)?;

        Ok(())
    }
}

/// Wasm table.
pub struct Table {
    /// Table's type details.
    pub table_type: TableType
}

impl Table {
    pub(crate) fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        Ok(Self {
            table_type: TableType::decode(reader)?
        })
    }
}

/// Description/schema of a table.
#[derive(Debug, PartialEq)]
pub struct TableType {
    /// Min (initial) and (optional) max size of the table.
    pub limits: Limits,

    /// Type of all elements in the table.
    pub elem_type: ElemType,
}

impl TableType {
    fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        Ok(Self {
            elem_type: ElemType::decode(reader)?,
            limits: Limits::decode(reader)?,
        })
    }
}

/// Types of elements in a Wasm table.
#[derive(Debug, PartialEq)]
pub enum ElemType {
    /// Function reference/index.
    FuncRef,
}

impl ElemType {
    fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        let actual = reader.read_byte()?;

        match actual {
            0x70 => Ok(Self::FuncRef),
            _ => Err(DecodingError::InvalidElemType { actual })
        }
    }
}

/// Wasm linear memory.
pub struct Mem {
    /// Memory's size details.
    pub mem_type: Limits,
}

impl Mem {
    pub(crate) fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        Ok(Self {
            mem_type: Limits::decode(reader)?
        })
    }
}

/// Details the minimum and (optional) maximum size of a definition (mainly for tables and linear memories).
#[derive(Debug, PartialEq)]
pub struct Limits {
    /// Minimum size.
    pub min: u32,

    /// Maximum size (optional).
    pub max: Option<u32>,
}

impl Limits {
    /// Max is not defined.
    const FLAG_MAX_MISSING: u8 = 0x00;

    /// Max is defined.
    const FLAG_MAX_PRESENT: u8 = 0x01;

    fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        let actual = reader.read_byte()?;

        match actual {
            Self::FLAG_MAX_MISSING => Ok(Self { min: reader.read_u32()?, max: None }),
            Self::FLAG_MAX_PRESENT => Ok(Self { min: reader.read_u32()?, max: Some(reader.read_u32()?) }),
            _ => Err(DecodingError::InvalidLimits { actual })
        }
    }
}

/// Wasm global variable.
pub struct Global {
    /// Global's details.
    pub global_type: GlobalType,

    /// Constant initializer expression.
    pub init: Expr,
}

impl Global {
    pub(crate) fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        Ok(Self {
            global_type: GlobalType::decode(reader)?,
            init: Expr::decode(reader)?
        })
    }
}

/// Description/schema of a global variable.
#[derive(Debug, PartialEq)]
pub struct GlobalType {
    /// Type of the global's value.
    pub val_type: ValType,

    /// Mutability of the global.
    pub mutability: Mutability,
}

impl GlobalType {
    fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        Ok(Self {
            val_type: ValType::decode(reader)?,
            mutability: Mutability::decode(reader)?
        })
    }
}

/// Mutabilities of data.
#[derive(Debug, PartialEq)]
pub enum Mutability {
    /// Immutable.
    Const,

    /// Mutable.
    Var,
}

impl Mutability {
    fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        let actual = reader.read_byte()?;

        match actual {
            0x00 => Ok(Self::Const),
            0x01 => Ok(Self::Var),
            _ => Err(DecodingError::InvalidMutability { actual })
        }
    }
}

/// Wasm element segment.
#[derive(Debug, PartialEq)]
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

impl Elem {
    pub(crate) fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        let table_idx = reader.read_u32()?;

        let offset = Expr::decode(reader)?;
        
        let num_funcs = reader.read_u32()? as usize;
        let init: Vec<u32> = (0..num_funcs).into_iter()
            .map(|_| reader.read_u32())
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            table_idx,
            offset,
            init
        })
    }
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

impl Data {
    pub(crate) fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        let mem_idx = reader.read_u32()?;

        let offset = Expr::decode(reader)?;

        let num_bytes = reader.read_u32()?;
        let init: Vec<u8> = (0..num_bytes).into_iter()
            .map(|_| reader.read_byte())
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            mem_idx,
            offset,
            init
        })
    }
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
        // get module name
        let module_len = reader.read_u32()? as usize;
        let mut pos = reader.pos();
        let module = std::str::from_utf8(reader.read_bytes(module_len)?)
            .map_err(|_| DecodingError::InvalidUTF8Name { pos })?
            .to_string();

        // get import name
        let name_len = reader.read_u32()? as usize;
        pos = reader.pos();
        let name = std::str::from_utf8(reader.read_bytes(name_len)?)
            .map_err(|_| DecodingError::InvalidUTF8Name { pos })?
            .to_string();

        Ok(Self { 
            module, 
            name, 
            desc: ImportDesc::decode(reader)?
        })
    }
}

/// Types of imports.
#[derive(Debug, PartialEq)]
pub enum ImportDesc {
    /// Function type index.
    Func(u32),

    /// Table type.
    Table(TableType),

    /// Memory type.
    Mem(Limits),

    /// Global type.
    Global(GlobalType),
}

impl ImportDesc {
    fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        let actual = reader.read_byte()?;

        match actual {
            0x00 => Ok(Self::Func(reader.read_u32()?)),
            0x01 => Ok(Self::Table(TableType::decode(reader)?)),
            0x02 => Ok(Self::Mem(Limits::decode(reader)?)),
            0x03 => Ok(Self::Global(GlobalType::decode(reader)?)),
            _ => Err(DecodingError::InvalidImportDesc { actual })
        }
    }
}

/// Wasm export.
pub struct Export {
    /// Name of the export.
    pub name: String,

    /// Type/Descriptor of the export.
    pub desc: ExportDesc,
}

impl Export {
    pub(crate) fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        // get export name
        let name_len = reader.read_u32()? as usize;
        let pos = reader.pos();
        let name = std::str::from_utf8(reader.read_bytes(name_len)?)
            .map_err(|_| DecodingError::InvalidUTF8Name { pos })?
            .to_string();

        Ok(Self { 
            name, 
            desc: ExportDesc::decode(reader)?,
        })
    }
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

impl ExportDesc {
    fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        let actual = reader.read_byte()?;

        match actual {
            0x00 => Ok(Self::Func(reader.read_u32()?)),
            0x01 => Ok(Self::Table(reader.read_u32()?)),
            0x02 => Ok(Self::Mem(reader.read_u32()?)),
            0x03 => Ok(Self::Global(reader.read_u32()?)),
            _ => Err(DecodingError::InvalidExportDesc { actual })
        }
    }
}

/// Signature of Wasm structured control instructions (block/loop/if/else).
#[derive(Debug, PartialEq)]
pub enum BlockType {
    /// Returns nothing.
    Empty,

    /// Returns a single value.
    Val(ValType)
}

impl BlockType {
    pub(crate) fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        if reader.peek_byte()? == 0x40 {
            reader.read_byte()?;
            Ok(Self::Empty)
        } else {
            Ok(Self::Val(ValType::decode(reader)?))
        }
    }
}

/// Wasm memory argument.
#[derive(Debug, PartialEq)]
pub struct MemArg {
    /// Alignment of address.
    pub align: u32,

    /// Offset from address.
    pub offset: u32,
}

impl MemArg {
    pub(crate) fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        Ok(Self {
            align: reader.read_u32()?,
            offset: reader.read_u32()?
        })
    }

    pub(crate) fn validate(&self, validator: &mut Validator, bit_width: usize) -> Result<(), ValidationError> {
        if validator.mems.is_empty() {
            return Err(ValidationError::NoLinearMemoryDefined);
        }

        // alignment must not be larger than bit width divided by 8 (num of bytes)
        let alignment = 2u64.pow(self.align);
        let num_bytes = (bit_width / 8) as u64;

        if alignment > num_bytes {
            return Err(ValidationError::AlignmentLargerThanBitWidth { alignment: alignment as usize, bit_width });
        }

        Ok(())
    }
}