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
}