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
#[derive(Clone, PartialEq, Debug)]
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

    pub(crate) fn validate(&self) -> Result<(), ValidationError> {
        // At most 1 result is allowed in Wasm 1.0
        if self.results.len() > 1 {
            return Err(ValidationError::InvalidFuncTypeResultCount { result_count: self.results.len() });
        }

        Ok(())
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

    pub(crate) fn validate(&self, validator: &mut Validator) -> Result<(), ValidationError> {
        let func_type = validator.types
            .get(self.type_idx as usize)
            .ok_or(ValidationError::UndefinedType { index: self.type_idx as usize })?;

        // set locals and validate function body
        validator.locals = func_type.params
            .clone()
            .into_iter()
            .chain(
                self.locals.clone().into_iter()
            )
            .collect();
        
        self.body.validate(validator, func_type.results.clone())?;

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

    pub(crate) fn validate(&self) -> Result<(), ValidationError> {
        self.table_type.validate()
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
    /// The range which the limit must be valid within.
    const TABLE_MAX: u64 = 4294967296;

    fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        Ok(Self {
            elem_type: ElemType::decode(reader)?,
            limits: Limits::decode(reader)?,
        })
    }

    fn validate(&self) -> Result<(), ValidationError> {
        self.limits.validate(Self::TABLE_MAX)
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
    /// The maximum number of pages a linear memory is allowed to span.
    pub const MEMORY_MAX: u64 = 65536;

    pub(crate) fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        Ok(Self {
            mem_type: Limits::decode(reader)?
        })
    }

    pub(crate) fn validate(&self) -> Result<(), ValidationError> {
        self.mem_type.validate(Self::MEMORY_MAX)
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

    fn validate(&self, k: u64) -> Result<(), ValidationError> {
        // min shouldn't be larger than k
        if self.min as u64 > k {
            return Err(ValidationError::LimitsMinLargerThanK { min: self.min as usize, k: k as usize });
        }

        if let Some(max) = self.max {
            // max must not be larger than k
            if max as u64 > k {
                return Err(ValidationError::LimitsMaxLargerThanK { max: max as usize, k: k as usize })
            }

            // min must not be larger than max
            if max < self.min {
                return Err(ValidationError::LimitsMinLargerThanMax { min: self.min as usize, max: max as usize });
            }
        }

        Ok(())
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

    pub(crate) fn validate(&self, validator: &mut Validator) -> Result<(), ValidationError> {
        // global type is already valid.
        self.init.validate_const_expr(validator, Some(self.global_type.val_type))
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

    pub(crate) fn validate(&self, validator: &mut Validator) -> Result<(), ValidationError> {
        let table_type = validator.tables
            .get(self.table_idx as usize)
            .ok_or(ValidationError::UndefinedTable { index: self.table_idx as usize })?;

        table_type.validate()?;

        // offset result has to be an i32/index to start writing at in table
        self.offset.validate_const_expr(validator, Some(ValType::I32))?;

        // every function has to be defined
        for func_idx in &self.init {
            if validator.funcs.len() <= *func_idx as usize {
                return Err(ValidationError::UndefinedFunction { index: *func_idx as usize });
            }
        }

        Ok(())
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

    pub(crate) fn validate(&self, validator: &mut Validator) -> Result<(), ValidationError> {
        if validator.mems.len() <= self.mem_idx as usize {
            return Err(ValidationError::UndefinedLinearMemory { index: self.mem_idx as usize });
        }

        // offset result has to be an i32/index to start writing at in linear memory
        self.offset.validate_const_expr(validator, Some(ValType::I32))?;

        Ok(())
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

    pub(crate) fn validate(&self, validator: &mut Validator) -> Result<(), ValidationError> {
        self.desc.validate(validator)
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

    pub(crate) fn validate(&self, validator: &mut Validator) -> Result<(), ValidationError> {
        match self {
            Self::Func(type_idx) => {
                if validator.types.len() <= *type_idx as usize {
                    return Err(ValidationError::UndefinedType { index: *type_idx as usize });
                }
            },

            Self::Table(table_type) => table_type.validate()?,

            Self::Mem(mem_type) => mem_type.validate(Mem::MEMORY_MAX)?,

            // Global types are already valid.
            _ => ()
        }

        Ok(())
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

    pub(crate) fn validate(&self, validator: &mut Validator) -> Result<(), ValidationError> {
        self.desc.validate(validator)
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

    pub(crate) fn validate(&self, validator: &mut Validator) -> Result<(), ValidationError> {
        match self {
            Self::Func(func_idx) => {
                if validator.funcs.len() <= *func_idx as usize {
                    return Err(ValidationError::UndefinedFunction { index: *func_idx as usize });
                }
            }

            Self::Table(table_idx) => {
                if validator.tables.len() <= *table_idx as usize {
                    return Err(ValidationError::UndefinedTable { index: *table_idx as usize });
                }
            }

            Self::Mem(mem_idx) => {
                if validator.mems.len() <= *mem_idx as usize {
                    return Err(ValidationError::UndefinedLinearMemory { index: *mem_idx as usize });
                }
            }

            Self::Global(global_idx) => {
                if validator.globals.len() <= *global_idx as usize {
                    return Err(ValidationError::UndefinedGlobal { index: *global_idx as usize });
                }
            }
        }

        Ok(())
    }
}

/// Signature of Wasm structured control instructions (block/loop/if/else).
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BlockType {
    /// Returns nothing.
    Empty,

    /// Returns a single value.
    Val(ValType)
}

impl Into<Vec<ValType>> for BlockType {
    fn into(self) -> Vec<ValType> {
        match self {
            Self::Empty => vec![],
            Self::Val(v) => vec![v]
        }
    }
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

    /// Returns the arity of the block type.
    pub(crate) fn arity(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Val(_) => 1,
        }
    }
}

/// Wasm memory argument.
#[derive(Debug, PartialEq, Clone)]
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