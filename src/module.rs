use crate::{decoder::Decoder, errors::DecodeError};
use crate::definitions::*;

const MAGIC_HEADER: &[u8; 4] = b"\0asm";
const WASM_1_0_SPEC_VERSION: &[u8; 4] = &[1, 0, 0, 0];

/// Standalone module representation.
#[derive(Default)]
pub struct Module {
    /// Types of the functions in the module.
    pub types: Vec<FuncType>,

    /// Functions in the module.
    pub funcs: Vec<Func>,

    /// Tables in the module.
    pub tables: Vec<Table>,

    /// Linear memories in the module.
    pub mems: Vec<Mem>,

    /// Global variables in the module.
    pub globals: Vec<Global>,

    /// Element segments in the module.
    pub elem: Vec<Elem>,

    /// Data segments in the module.
    pub data: Vec<Data>,

    /// Index of the start function (in `funcs`) in the module.
    pub start: Option<u32>,

    /// Imported definitions required for the module's instantiation.
    pub imports: Vec<Import>,

    /// Exported definitions in the module.
    pub exports: Vec<Export>,
}

impl Module {
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut decoder = Decoder::new(bytes);

        // read magic header
        if decoder.read_bytes(MAGIC_HEADER.len())? != MAGIC_HEADER {
            return Err(DecodeError::InvalidMagicHeader);
        }

        // read wasm specification version
        if decoder.read_bytes(WASM_1_0_SPEC_VERSION.len())? != WASM_1_0_SPEC_VERSION {
            return Err(DecodeError::InvalidSpecificationVersion);
        }

        let this = Self {
            ..Default::default()
        };

        Ok(this)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_empty_module() {
        let bytes = [0x0, 0x61, 0x73, 0x6d, 0x01, 0x0, 0x0, 0x0];
        assert!(Module::decode(&bytes).is_ok());
    }

    #[test]
    fn decode_invalid_magic_header() {
        let bytes = [0x0, 0x60, 0x73, 0x6d, 0x01, 0x0, 0x0, 0x0];
        assert!(Module::decode(&bytes).is_err_and(|e| e == DecodeError::InvalidMagicHeader));
    }

    #[test]
    fn decode_invalid_version() {
        let bytes = [0x0, 0x61, 0x73, 0x6d, 0x02, 0x0, 0x0, 0x0];
        assert!(Module::decode(&bytes).is_err_and(|e| e == DecodeError::InvalidSpecificationVersion));
    }
}