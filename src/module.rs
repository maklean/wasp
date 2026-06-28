use crate::{decoder::Decoder, definitions::*, errors::DecodeError};

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

        let mut this = Self {
            ..Default::default()
        };

        // decode sections if we haven't reached EOF
        if !decoder.eof() {
            this.decode_sections(&mut decoder)?;
        }

        Ok(this)
    }

    /// Decodes each section in the module.
    fn decode_sections(&mut self, decoder: &mut Decoder) -> Result<(), DecodeError> {
        loop {
            let section_id: Section = Section::try_from(decoder.read_byte()?)?;
            let section_size = decoder.read_u32()?;
            let mut section = Decoder::new(decoder.read_bytes(section_size as usize)?);

            // skip custom section
            if section_id == Section::Custom {
                continue;
            }

            match section_id {
                Section::Type => self.decode_type_section(&mut section)?,
                _ => todo!()
            }

            if decoder.eof() {
                break;
            }
        }

        Ok(())
    }

    /// Decodes the type section in the module.
    fn decode_type_section(&mut self, decoder: &mut Decoder) -> Result<(), DecodeError> {
        let n = decoder.read_u32()?;

        for _ in 0..n {
            decoder.match_byte(0x60, DecodeError::InvalidFunctionType)?;

            // Get parameters
            let param_count = decoder.read_u32()? as usize;
            let params: Vec<ValType> = decoder.read_bytes(param_count)?
                .iter()
                .map(|&b| ValType::try_from(b))
                .collect::<Result<Vec<_>, _>>()?;

            // Get function result (there should only be at most one)
            let results_count = decoder.read_u32()? as usize;
            if results_count > 1 {
                return Err(DecodeError::InvalidFunctionTypeResultCount);
            }

            let results: Vec<ValType> = decoder.read_bytes(results_count)?
                .iter()
                .map(|&b| ValType::try_from(b))
                .collect::<Result<Vec<_>, _>>()?;

            // Add function type to module types
            self.types.push(FuncType { params, results });
        }

        Ok(())
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

    #[test]
    fn decode_type_section_multiple() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x01,                   // type section id
            0x10,                   // section size

            0x03,                   // 3 function types

            // (i32) -> i32
            0x60,                   // type 1: functype marker
            0x01, 0x7f,             // type 1: 1 param
            0x01, 0x7f,             // type 1: 1 result

            // (i32, i64, i32) -> f64
            0x60,                   // type 2: functype marker
            0x03, 0x7f, 0x7e, 0x7f, // type 2: 3 params
            0x01, 0x7c,             // type 2: 1 result

            // () -> ()
            0x60,                   // type 3: functype marker
            0x00,                   // type 3: 0 params
            0x00,                   // type 3: 0 results
        ];

        let module = Module::decode(&bytes).unwrap();
        assert_eq!(module.types.len(), 3);

        // (i32) -> i32
        assert_eq!(module.types[0].params.len(), 1);
        assert!(matches!(module.types[0].params[0], ValType::I32));

        assert_eq!(module.types[0].results.len(), 1);
        assert!(matches!(module.types[0].results[0], ValType::I32));

        // (i32, i64, i32) -> f64
        assert_eq!(module.types[1].params.len(), 3);
        assert!(matches!(module.types[1].params[0], ValType::I32));
        assert!(matches!(module.types[1].params[1], ValType::I64));
        assert!(matches!(module.types[1].params[2], ValType::I32));
        
        assert_eq!(module.types[1].results.len(), 1);
        assert!(matches!(module.types[1].results[0], ValType::F64));

        // () -> ()
        assert_eq!(module.types[2].params.len(), 0);
        assert_eq!(module.types[2].results.len(), 0);

    }

    #[test]
    fn decode_type_section_invalid_marker() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x01,                   // type section id
            0x0b,                   // section size

            0x02,                   // 2 function types
            
            // (i32, i32) -> f64
            0x60,                   // type 1: functype marker
            0x02, 0x7f, 0x7f,       // type 1: 2 params
            0x01, 0x7c,             // type 1: 1 result
            
            // () -> i32
            0x61,                   // type 2: functype marker <-- shouild err here (expected 0x60)
            0x00,                   // type 2: 0 params
            0x01, 0x7f,             // type 2: 1 result
        ];

        assert!(Module::decode(&bytes).is_err_and(|e| e == DecodeError::InvalidFunctionType));
    }
}