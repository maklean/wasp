#[derive(PartialEq, Eq, Debug)]
pub enum DecodeError {
    InvalidMagicHeader,
    InvalidSpecificationVersion,
    UnexpectedEof,
    MalformedInteger,
    InvalidSectionId,
    InvalidFunctionType,
    InvalidValType,
    InvalidFunctionTypeResultCount,
    InvalidSectionOrder,
    InvalidUTF8Name,
    InvalidDesc,
    InvalidElemType,
    InvalidLimitsFlag,
    InvalidMutability,
}