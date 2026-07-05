use crate::{decoder::Decoder, errors::{DecodeError, ValidateError}, instructions::Expr};

/// Types that Wasm code can use for its values.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ValType {
    I32,
    I64,
    F32,
    F64,
    Unknown
}

impl TryFrom<u8> for ValType {
    type Error = DecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x7f => Ok(Self::I32),
            0x7e => Ok(Self::I64),
            0x7d => Ok(Self::F32),
            0x7c => Ok(Self::F64),
            _ => Err(DecodeError::InvalidValType),
        }
    }
}

/// Signature of functions; maps vector of parameters to vector of results (in Wasm 1.0, there's only at most 1 result returned).
pub struct FuncType {
    /// Function parameters.
    pub params: Vec<ValType>,

    /// Function results.
    pub results: Vec<ValType>,
}

impl FuncType {
    const MARKER: u8 = 0x60;

    /// Decodes a single `FuncType` from a sequence of bytes and returns an instance if successful.
    pub fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        // Sequence should start with a 0x60 marker
        decoder.match_byte(FuncType::MARKER, DecodeError::InvalidFunctionType)?;

        // Get parameters
        let param_count = decoder.read_u32()? as usize;
        let params: Vec<ValType> = decoder.read_bytes(param_count)?
            .iter()
            .map(|&b| ValType::try_from(b))
            .collect::<Result<Vec<_>, _>>()?;

        let results_count = decoder.read_u32()? as usize;
        let results: Vec<ValType> = decoder.read_bytes(results_count)?
            .iter()
            .map(|&b| ValType::try_from(b))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { params, results })
    }

    /// Validates a `FuncType`.
    pub fn validate(&self) -> Result<(), ValidateError> {
        // FuncTypes can only have at most one result in Wasm 1.0
        if self.results.len() > 1 {
            return Err(ValidateError::FuncTypeHasMoreThanOneResult);
        }

        Ok(())
    }
}

#[derive(Default)]
/// Wasm module's function outline.
pub struct Func {
    /// Index to the function's signature in the module's `types` vector.
    pub type_idx: u32,

    /// Vector of mutable local variables and their types, function parameters are the first elements in the vector.
    pub locals: Vec<ValType>,

    /// Instruction sequence for the function.
    pub body: Expr,
}

impl Func {
    /// Returns a `Func` with only the type_idx being set/decoded.
    pub fn decode_type_idx(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        Ok(Self {
            type_idx: decoder.read_u32()?,
            ..Default::default()
        })
    }

    /// Decodes and sets the local variables and body of a `Func`.
    pub fn decode_locals_body(&mut self, decoder: &mut Decoder) -> Result<(), DecodeError> {
        let num_locals_group = decoder.read_u32()?  as usize;
        self.locals.reserve_exact(num_locals_group);

        for _ in 0..num_locals_group {
            let n = decoder.read_u32()? as usize;
            let val_type = ValType::try_from(decoder.read_byte()?)?;

            // add 'n' of this ValType
            for _ in 0..n {
                self.locals.push(val_type.clone());
            }
        }

        self.body = Expr::decode(decoder)?;

        Ok(())
    }
}

/// Wasm module's table outline.
pub struct Table {
    /// Table's details.
    pub table_type: TableType,
}

impl Table {
    /// Decodes a `Table` from a sequence of bytes.
    pub fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        let table_type = TableType::decode(decoder)?;

        Ok(Self { table_type })
    }

    /// Validates a `Table`.
    pub fn validate(&self) -> Result<(), ValidateError> {
        self.table_type.validate()
    }
}

/// Wasm module's linear memory outline.
pub struct Mem {
    /// Min (initial) and max size of the memory.
    pub mem_type: Limits,
}

impl Mem {
    const MEMORY_MAX: u64 = 65536;

    /// Decodes a linear memory from a sequence of bytes.
    pub fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        let mem_type = Limits::decode(decoder)?;

        Ok(Self { mem_type })
    }

    /// Validates a linear memory.
    pub fn validate(&self) -> Result<(), ValidateError> {
        self.mem_type.validate(Self::MEMORY_MAX)
    }
}

/// Wasm module's global variable outline.
pub struct Global {
    /// Global's details.
    pub global_type: GlobalType,

    /// Constant expression that initializes the global's value.
    pub init: Expr,
}

impl Global {
    /// Decodes a global variable.
    pub fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        let global_type = GlobalType::decode(decoder)?;
        let init = Expr::decode(decoder)?;

        if !init.is_const() {
            return Err(DecodeError::InvalidNonConstExpr);
        }

        Ok(Self { global_type, init })
    }
}

/// A Wasm module element segment outline (initializes a subrange of a table).
pub struct Elem {
    /// Index of the table in the module (should always be 0 since only one table is allowed per module in Wasm 1.0).
    pub table_idx: u32,

    /// Offset into the table to start writing at.
    pub offset: Expr,

    /// Function indices to write into the table slots from the offset.
    pub init: Vec<u32>,
}

impl Elem {
    pub fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        let table_idx = decoder.read_u32()?;

        // only one table is allowed in Wasm 1.0
        if table_idx != 0 {
            return Err(DecodeError::InvalidTableIndex);
        }

        let offset = Expr::decode(decoder)?;

        // the offset expression has to be a constant expression
        if !offset.is_const() {
            return Err(DecodeError::InvalidNonConstExpr);
        }

        // get func indexes.
        let num_funcs = decoder.read_u32()? as usize;
        let mut init: Vec<u32> = Vec::with_capacity(num_funcs);

        for _ in 0..num_funcs {
            init.push(decoder.read_u32()?);
        }

        Ok(Self { table_idx, offset, init })
    }
}

/// A wasm module data segment outline (initializes a subrange of a linear memory).
pub struct Data {
    /// Index of the memory in the module.
    pub mem_idx: u32,

    /// Offset into the memory to start writing at.
    pub offset: Expr,

    /// Bytes to write into the memory slots from the offset.
    pub init: Vec<u8>,
}

impl Data {
    /// Decodes a data segment from a sequence of bytes.
    pub fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        let mem_idx = decoder.read_u32()?;

        // only one linear memory is allowed in Wasm 1.0
        if mem_idx != 0 {
            return Err(DecodeError::InvalidMemoryIndex);
        }

        let offset = Expr::decode(decoder)?;

        // the offset expression has to be a constant expression
        if !offset.is_const() {
            return Err(DecodeError::InvalidNonConstExpr);
        }

        let num_bytes = decoder.read_u32()? as usize;
        let mut init: Vec<u8> = Vec::with_capacity(num_bytes);

        for _ in 0..num_bytes {
            init.push(decoder.read_byte()?);
        }

        Ok(Self { mem_idx, offset, init })
    }
}

/// Wasm module import outline.
pub struct Import {
    /// Module name.
    pub module: String,

    /// Import name.
    pub name: String,

    /// Type/Descriptor of import.
    pub desc: ImportDesc,
}

impl Import {
    /// Decodes an `Import` from a sequence of bytes.
    pub fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        // get module
        let module_len = decoder.read_u32()? as usize;
        let module = std::str::from_utf8(decoder.read_bytes(module_len)?)
            .map_err(|_| DecodeError::InvalidUTF8Name)?
            .to_string();

        // get name
        let name_len = decoder.read_u32()? as usize;
        let name = std::str::from_utf8(decoder.read_bytes(name_len)?)
            .map_err(|_| DecodeError::InvalidUTF8Name)?
            .to_string();

        // get desc
        let desc = ImportDesc::decode(decoder)?;

        Ok(Self { module, name, desc })
    }
}

/// Wasm module export outline.
pub struct Export {
    /// Export name (unique).
    pub name: String,

    /// Type/Descriptor of export.
    pub desc: ExportDesc,
}

impl Export {
    /// Decodes an `Export` from a sequence of bytes.
    pub fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        // get name
        let name_len = decoder.read_u32()? as usize;
        let name = std::str::from_utf8(decoder.read_bytes(name_len)?)
            .map_err(|_| DecodeError::InvalidUTF8Name)?
            .to_string();

        // get desc
        let desc = ExportDesc::decode(decoder)?;

        Ok(Self { name, desc })
    }
}

/// Description/schema of a table.
pub struct TableType {
    /// Min (initial) and (optional) max size of the table.
    pub limits: Limits,

    /// Type of all elements in the table.
    pub elem_type: ElemType,
}

impl TableType {
    /// The range which the limit must be valid within.
    const TABLE_MAX: u64 = 4294967296;

    /// Decodes a `TableType` from a sequence of bytes.
    fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        let elem_type = ElemType::try_from(decoder.read_byte()?)?;
        let limits = Limits::decode(decoder)?;

        Ok(Self { limits, elem_type })
    }

    /// Validates a `TableType`.
    fn validate(&self) -> Result<(), ValidateError> {
        self.limits.validate(Self::TABLE_MAX)
    }
}

/// Details the minimum and (optional) maximum size of a definition (mainly for tables and linear memories).
pub struct Limits {
    pub min: u32,
    pub max: Option<u32>,
}

impl Limits {
    const FLAG_MAX_MISSING: u8 = 0x00;
    const FLAG_MAX_PRESENT: u8 = 0x01;

    /// Decodes a `Limits` from a sequence of bytes, returns the correct variant based on if the maximum is present or not.
    fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        // return Limits instance based on flag
        match decoder.read_byte()? {
            Self::FLAG_MAX_MISSING => Ok(Self { min: decoder.read_u32()?, max: None }),
            Self::FLAG_MAX_PRESENT => Ok(Self { min: decoder.read_u32()?, max: Some(decoder.read_u32()?) }),
            _ => Err(DecodeError::InvalidLimitsFlag)
        }
    }

    /// Validaes a `Limits` instance.
    fn validate(&self, k: u64) -> Result<(), ValidateError> {
        // min shouldn't be larger than k
        if self.min as u64 > k {
            return Err(ValidateError::LimitsMinLargerThanK(k));   
        }

        if let Some(max) = self.max {
            // maximum must not be larger than k
            if max as u64 > k {
                return Err(ValidateError::LimitsMaxLargerThanK);
            }

            // max must not be smaller than min
            if max < self.min {
                return Err(ValidateError::LimitsMinLargerThanMax);
            }
        }

        Ok(())
    }
}

/// Types of elements in a table (In Wasm 1.0, the only ElemType is a function reference / index to a Func).
pub enum ElemType {
    FuncRef,
}

impl TryFrom<u8> for ElemType {
    type Error = DecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x70 => Ok(Self::FuncRef),
            _ => Err(DecodeError::InvalidElemType),
        }
    }
}

/// Description/schema of a global variable.
pub struct GlobalType {
    /// Type of the global's value.
    pub val_type: ValType,

    /// Mutability of the global.
    pub mutability: Mutability,
}

impl GlobalType {
    /// Decodes a `GlobalType` from a sequence of bytes.
    fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        let val_type = ValType::try_from(decoder.read_byte()?)?;
        let mutability = Mutability::try_from(decoder.read_byte()?)?;

        Ok(Self { val_type, mutability })
    }

    /// Validates a `GlobalType`.
    fn validate(&self) -> Result<(), ValidateError> {
        Ok(())
    }
}

/// Details the possible mutabilities of data.
pub enum Mutability {
    /// Immutable.
    Const,

    /// Mutable.
    Var,
}

impl TryFrom<u8> for Mutability {
    type Error = DecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::Const),
            0x01 => Ok(Self::Var),
            _ => Err(DecodeError::InvalidMutability)
        }
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

impl ImportDesc {
    /// Decodes an `ImportDesc` from a sequence of bytes.
    fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        match decoder.read_byte()? {
            0x00 => Ok(Self::Func(decoder.read_u32()?)),
            0x01 => Ok(Self::Table(TableType::decode(decoder)?)),
            0x02 => Ok(Self::Mem(Limits::decode(decoder)?)),
            0x03 => Ok(Self::Global(GlobalType::decode(decoder)?)),
            _ => Err(DecodeError::InvalidImportDesc),
        }
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
    /// Decodes an `ExportDesc` from a sequence of bytes.
    fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        match decoder.read_byte()? {
            0x00 => Ok(Self::Func(decoder.read_u32()?)),
            0x01 => Ok(Self::Table(decoder.read_u32()?)),
            0x02 => Ok(Self::Mem(decoder.read_u32()?)),
            0x03 => Ok(Self::Global(decoder.read_u32()?)),
            _ => Err(DecodeError::InvalidExportDesc),
        }
    }
}

/// Wasm module section
#[repr(u8)]
#[derive(PartialEq, PartialOrd)]
pub enum Section {
    Custom,
    Type,
    Import,
    Function,
    Table,
    Memory,
    Global,
    Export,
    Start,
    Element,
    Code, 
    Data
}

impl TryFrom<u8> for Section {
    type Error = DecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Section::Custom),
            0x01 => Ok(Section::Type),
            0x02 => Ok(Section::Import),
            0x03 => Ok(Section::Function),
            0x04 => Ok(Section::Table),
            0x05 => Ok(Section::Memory),
            0x06 => Ok(Section::Global),
            0x07 => Ok(Section::Export),
            0x08 => Ok(Section::Start),
            0x09 => Ok(Section::Element),
            0x0A => Ok(Section::Code),
            0x0B => Ok(Section::Data),
            _ => Err(DecodeError::InvalidSectionId),
        }
    }
}