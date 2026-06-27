#[derive(PartialEq, Eq)]
pub enum DecodeError {
    InvalidMagicHeader,
    InvalidSpecificationVersion,
}