use std::{fs, path::Path};

use crate::{binary::ParsedModule, errors::DecodingError};

pub struct Module {
    pub parsed: ParsedModule
}

impl Module {
    /// Decodes the Wasm module at the given path.
    pub fn decode_from_file(path: impl AsRef<Path>) -> Result<Self, DecodingError> {
        let bytes = fs::read(path)
            .map_err(|e| DecodingError::Io(e))?;

        Self::decode_from_bytes(&bytes)
    }

    /// Decodes a Wasm module from its bytes.
    pub fn decode_from_bytes(bytes: &[u8]) -> Result<Self, DecodingError> {
        Ok(Self { 
            parsed: ParsedModule::decode_from_bytes(bytes)?
        })
    }
}