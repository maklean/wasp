use std::io;

use crate::{binary::ModuleSection, structure::{FuncType, Instr, Mutability, ValType}};

/// Errors from decoding a Wasm module.
#[derive(Debug)]
pub enum DecodingError {
    UnexpectedEof { pos: usize },
    MalformedInteger { pos: usize },
    MalformedFloatingPoint { pos: usize },
    InvalidMagicNumber,
    InvalidSpecificationVersion,
    InvalidSectionId { actual: u8 },
    InvalidUTF8Name { pos: usize },
    InvalidSectionOrder { last: ModuleSection, curr: ModuleSection },
    SectionSizeMismatch { expect: usize, actual: usize },
    FunctionCodeCountMismatch { func_count: usize, code_count: usize },
    InvalidFuncTypeMarker { pos: usize, actual: u8 },
    InvalidValType { actual: u8 },
    InvalidElemType { actual: u8 },
    InvalidLimits { actual: u8 },
    InvalidMutability { actual: u8 },
    InvalidImportDesc { actual: u8 },
    InvalidExportDesc { actual: u8 },
    TooManyLocals,
    InvalidFuncCodeSize { expect: usize, actual: usize },
    ExpectedEndOfInstrSeq { actual: u8 },
    InvalidInstr { actual: u8 },
    InvalidMemorySizeInstr { actual: u8 },
    InvalidMemoryGrowInstr { actual: u8 },
    InvalidCallIndirectInstr { actual: u8 },
    InvalidIfThenInstr { actual: u8 },
    OpenLabelStackUnderflow,
    Io(io::Error),
}

/// Errors from validating a Wasm module.
#[derive(Debug)]
pub enum ValidationError {
    ExpectedControlFrame,
    PoppingOutsideControlFrame { frame_height: usize },
    ExpectedOperandInOpdStack,
    OperandMismatch { expect: ValType, actual: ValType },
    StackHeightMismatch { expect: usize, actual: usize },
    UndefinedLocal { index: usize },
    LocalSetTypeMismatch { expect: ValType, actual: ValType },
    UndefinedGlobal { index: usize },
    GlobalSetTypeMismatch { expect: ValType, actual: ValType },
    GlobalMustBeMutable { index: usize },
    NoLinearMemoryDefined,
    NoTableDefined,
    AlignmentLargerThanBitWidth { alignment: usize, bit_width: usize },
    InvalidLabelIndex { ctrl_frame_count: usize, index: usize },
    ExpectedMatchingLabelTypes { expect: Vec<ValType>, actual: Vec<ValType> },
    UndefinedFunction { index: usize },
    UndefinedType { index: usize },
    NonConstantInstruction { actual: Instr },
    InvalidNonImportedGlobal { index: usize },
    InvalidFuncTypeResultCount { result_count: usize },
    LimitsMinLargerThanK { min: usize, k: usize },
    LimitsMaxLargerThanK { max: usize, k: usize },
    LimitsMinLargerThanMax { min: usize, max: usize },
    UndefinedTable { index: usize },
    UndefinedLinearMemory { index: usize },
    InvalidStartFunction { params: Vec<ValType>, results: Vec<ValType> },
    DuplicateExportName { name: String },
    TooManyMems { count: usize },
    TooManyTables { count: usize },
}

/// Errors from executing and instantiating a Wasm module.
#[derive(Debug, PartialEq)]
pub enum ExecutionError {
    UnexpectedStackUnderflow(RuntimeStack),
    Trapped(TrapReason),
    EmbedderImportCountMismatch { module_count: usize, embedder_count: usize },
    EmbedderImportTypeMismatch { module: String, name: String, reason: EmbedderImportMismatchReason },
}

/// Types of runtime stacks.
#[derive(Debug, PartialEq)]
pub enum RuntimeStack {
    /// Operand stack.
    Operand,

    /// Function call frame stack.
    Frame,

    /// Control construct stack.
    Block,
}

/// Reasons for trapping.
#[derive(Debug, PartialEq)]
pub enum TrapReason {
    Unreachable,
    CallStackExhausted,
    UndefinedElement { index: usize },
    UninitializedElement { index: usize },
    IndirectCallTypeMismatch { expect: FuncType, actual: FuncType },
    OutOfBoundsMemoryAccess { addr: usize, len: usize, mem_size: usize },
    OutOfBoundsElementAccess { offset: usize, len: usize, table_size: usize },
    DivideByZero,
    IntegerOverflow,
    InvalidConversionToInteger,
}

/// Reasons an embedder-supplied import can fail to match a module's declared import.
#[derive(Debug, PartialEq)]
pub enum EmbedderImportMismatchReason {
    Kind,
    FuncSignature { expected: FuncType, actual: FuncType },
    TableTooSmall { expected_min: u32, actual_size: u32 },
    TableMaxTooLarge { expected_max: u32, actual_max: Option<u32> },
    MemTooSmall { expected_min: u32, actual_pages: u32 },
    MemMaxTooLarge { expected_max: u32, actual_max: Option<u32> },
    GlobalTypeMismatch { expected: ValType, actual: ValType },
    GlobalMutabilityMismatch { expected: Mutability, actual: Mutability },
}