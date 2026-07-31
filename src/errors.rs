use std::io;

use crate::binary::ModuleSection;

/// Error from decoding a Wasm module.
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
    SectionSizeMismatch { expected: usize, actual: usize },
    FunctionCodeCountMismatch { func_count: usize, code_count: usize },
    InvalidFuncTypeMarker { pos: usize, actual: u8 },
    InvalidValType { actual: u8 },
    InvalidElemType { actual: u8 },
    InvalidLimits { actual: u8 },
    InvalidMutability { actual: u8 },
    InvalidImportDesc { actual: u8 },
    InvalidExportDesc { actual: u8 },
    TooManyLocals,
    InvalidFuncCodeSize { expected: usize, actual: usize },
    ExpectedEndOfInstrSeq { actual: u8 },
    InvalidInstr { actual: u8 },
    InvalidMemorySizeInstr { actual: u8 },
    InvalidMemoryGrowInstr { actual: u8 },
    InvalidCallIndirectInstr { actual: u8 },
    InvalidIfThenInstr { actual: u8 },
    Io(io::Error),
}

/// Error from validating a Wasm module.
pub enum ValidationError {

}

/// Error from executing and instantiating a Wasm module.
pub enum ExecutionError {

}