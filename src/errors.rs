use crate::definitions::ValType;

#[derive(PartialEq, Eq, Debug)]
pub enum DecodeError {
    InvalidMagicHeader,
    InvalidSpecificationVersion,
    UnexpectedEof,
    MalformedInteger,
    MalformedFloatingPoint,
    InvalidSectionId,
    InvalidFunctionType,
    InvalidValType,
    InvalidSectionOrder,
    InvalidUTF8Name,
    InvalidImportDesc,
    InvalidExportDesc,
    InvalidElemType,
    InvalidLimitsFlag,
    InvalidMutability,
    InvalidTableCount,
    InvalidMemoryCount,
    InvalidIfThenInstr,
    ExpectedEndOfInstrSeq,
    InvalidCallIndirectInstr,
    InvalidMemorySizeInstr,
    InvalidMemoryGrowInstr,
    InvalidInstr,
    InvalidNonConstExpr,
    InvalidTableIndex,
    InvalidMemoryIndex,
    InvalidFunctionCount,
    MalformedCodeSize,
}

#[derive(PartialEq, Eq, Debug)]
pub enum ValidateError {
    LimitsMinLargerThanK(u64),
    LimitsMaxLargerThanK,
    LimitsMinLargerThanMax,
    FuncTypeHasMoreThanOneResult,
    ExpectedAtLeastOneControlFrame,
    PoppingOutsideOfControlFrame,
    ExpectedOperandInOpdStack { expect: ValType, actual: ValType },
    StackHeightMismatchAtEnd { expect: usize, actual: usize }
}