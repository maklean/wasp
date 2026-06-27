use crate::errors::DecodeError;

const MAGIC_HEADER: &[u8; 4] = b"\0asm";
const WASM_SPEC_VERSION: &[u8; 4] = &[1, 0, 0, 0];

pub struct Module {

}

impl Module {
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        // Check for magic header
        if bytes.len() < 4 || &bytes[0..4] != MAGIC_HEADER {
            return Err(DecodeError::InvalidMagicHeader);
        }

        // Check for version number
        if bytes.len() < 8 || &bytes[4..8] != WASM_SPEC_VERSION {
            return Err(DecodeError::InvalidSpecificationVersion);
        }

        Ok(Self {})
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