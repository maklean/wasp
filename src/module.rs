use std::{fs, path::Path, rc::Rc};

use crate::{binary::ParsedModule, errors::{DecodingError, ExecutionError, ValidationError}, runtime::{ExternVal, ModuleInstance, Store}, validation::Validator};

pub struct Module {
    pub parsed: ParsedModule,
    pub(crate) valid: bool,
}

impl Module {
    /// Decodes the Wasm module at the given path.
    pub fn decode_from_file(path: impl AsRef<Path>) -> Result<Self, DecodingError> {
        let bytes = fs::read(path)
            .map_err(DecodingError::Io)?;

        Self::decode_from_bytes(&bytes)
    }

    /// Decodes a Wasm module from its bytes.
    pub fn decode_from_bytes(bytes: &[u8]) -> Result<Self, DecodingError> {
        Ok(Self { 
            parsed: ParsedModule::decode_from_bytes(bytes)?,
            valid: false
        })
    }

    /// Validates the current module.
    pub fn validate(&mut self) -> Result<(), ValidationError> {
        let result = Validator::validate(&self.parsed);

        self.valid = result.is_ok();

        result
    }

    /// Instantiates the current module. Returns the `ModuleInstance` if sucessful.
    pub fn instantiate(&self, store: &mut Store, imports: &[ExternVal]) -> Result<Rc<ModuleInstance>, ExecutionError> {
        ModuleInstance::instantiate(self, store, imports)
    }
}