use std::io;

use crate::{binary::ModuleSection, structure::ValType};

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
    UndefinedType { index: usize }
}

/// Errors from executing and instantiating a Wasm module.
#[derive(Debug)]
pub enum ExecutionError {

}