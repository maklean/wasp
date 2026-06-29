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

        let mut this: Module = Default::default();

        // decode sections
        this.decode_sections(&mut decoder)?;

        Ok(this)
    }

    /// Decodes each section in the module.
    fn decode_sections(&mut self, decoder: &mut Decoder) -> Result<(), DecodeError> {
        let mut last_section_id = Section::Custom;

        while !decoder.eof() {
            let section_id: Section = Section::try_from(decoder.read_byte()?)?;
            let section_size = decoder.read_u32()?;
            let mut section = Decoder::new(decoder.read_bytes(section_size as usize)?);

            // skip custom section
            if section_id == Section::Custom {
                continue;
            }

            // excluding custom sections, section IDs have to appear in a monotonic non-decreasing order
            if last_section_id >= section_id {
                return Err(DecodeError::InvalidSectionOrder);
            }

            match section_id {
                Section::Type => self.decode_type_section(&mut section)?,
                Section::Import => self.decode_import_section(&mut section)?,
                Section::Function => self.decode_function_section(&mut section)?,
                Section::Table => self.decode_table_section(&mut section)?,
                Section::Memory => self.decode_memory_section(&mut section)?,
                _ => todo!()
            }

            last_section_id = section_id;
        }

        Ok(())
    }

    /// Decodes the type section in the module.
    fn decode_type_section(&mut self, decoder: &mut Decoder) -> Result<(), DecodeError> {
        let num_func_types = decoder.read_u32()? as usize;

        self.types.reserve_exact(num_func_types);

        for _ in 0..num_func_types {
            self.types.push(FuncType::decode(decoder)?);
        }

        Ok(())
    }

    /// Decodes the import section in the module.
    fn decode_import_section(&mut self, decoder: &mut Decoder) -> Result<(), DecodeError> {
        let num_imports = decoder.read_u32()? as usize;

        self.imports.reserve_exact(num_imports);

        for _ in 0..num_imports {
            self.imports.push(Import::decode(decoder)?);
        }

        Ok(())
    }

    /// Decodes the function section in the module.
    fn decode_function_section(&mut self, decoder: &mut Decoder) -> Result<(), DecodeError> {
        let num_funcs = decoder.read_u32()? as usize;

        self.funcs.reserve_exact(num_funcs);

        for _ in 0..num_funcs {
            self.funcs.push(Func::decode_type_idx(decoder)?);
        }

        Ok(())
    }

    /// Decodes the table section in the module.
    fn decode_table_section(&mut self, decoder: &mut Decoder) -> Result<(), DecodeError> {
        let num_tables = decoder.read_u32()? as usize;

        // there's only at most one table allowed in Wasm 1.0
        if num_tables > 1 {
            return Err(DecodeError::InvalidTableCount);
        }

        self.tables.reserve_exact(num_tables);

        for _ in 0..num_tables {
            self.tables.push(Table::decode(decoder)?);
        }

        Ok(())
    }

    /// Decodes the memory section in the module.
    fn decode_memory_section(&mut self, decoder: &mut Decoder) -> Result<(), DecodeError> {
        let num_memories = decoder.read_u32()? as usize;

        // there's only at most one linear memory allowed in Wasm 1.0
        if num_memories > 1 {
            return Err(DecodeError::InvalidMemoryCount);
        }

        self.mems.reserve_exact(num_memories);

        for _ in 0..num_memories {
            self.mems.push(Mem::decode(decoder)?);
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

    #[test]
    fn decode_import_section_func() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x02,                   // import section id
            0x0B,                   // section size

            0x01,                   // 1 import

            // import "env"."add" (func type index 0)
            0x03, 0x65, 0x6e, 0x76, // module name len=3, "env"
            0x03, 0x61, 0x64, 0x64, // import name len=3, "add"
            0x00,                   // desc: func
            0x00,                   // type index: 0
        ];

        let module = Module::decode(&bytes).unwrap();
        assert_eq!(module.imports.len(), 1);
        assert_eq!(module.imports[0].module, "env");
        assert_eq!(module.imports[0].name, "add");
        assert!(matches!(module.imports[0].desc, Desc::Func(0)));
    }

    #[test]
    fn decode_import_section_global() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x02,                   // import section id
            0x0A,                   // section size

            0x01,                   // 1 import

            // import "env"."g" (global i32 const)
            0x03, 0x65, 0x6e, 0x76, // module name len=3, "env"
            0x01, 0x67,             // import name len=1, "g"
            0x03,                   // desc: global
            0x7f,                   // val type: i32
            0x00,                   // mutability: const
        ];

        let module = Module::decode(&bytes).unwrap();
        assert_eq!(module.imports.len(), 1);
        assert_eq!(module.imports[0].module, "env");
        assert_eq!(module.imports[0].name, "g");

        match &module.imports[0].desc {
            Desc::Global(g) => {
                assert!(matches!(g.val_type, ValType::I32));
                assert!(matches!(g.mutability, Mutability::Const));
            }
            _ => panic!("expected global desc"),
        }
    }

    #[test]
    fn decode_import_section_invalid_desc() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x02,                   // import section id
            0x08,                   // section size

            0x01,                   // 1 import

            // import "env"."x" (invalid desc)
            0x03, 0x65, 0x6e, 0x76, // module name len=3, "env"
            0x01, 0x78,             // import name len=1, "x"
            0x04,                   // desc: invalid
        ];

        assert!(Module::decode(&bytes)
            .is_err_and(|e| e == DecodeError::InvalidDesc));
    }

    #[test]
    fn decode_import_section_invalid_utf8() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x02,                   // import section id
            0x05,                   // section size

            0x01,                   // 1 import

            // invalid UTF-8 module name
            0x02, 0xFF, 0xFE,       // module name len=2, invalid UTF-8
            0x01, 0x78,             // import name len=1, "x"
        ];

        assert!(Module::decode(&bytes)
            .is_err_and(|e| e == DecodeError::InvalidUTF8Name));
    }

    #[test]
    fn decode_table_section() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x04,                   // table section id
            0x04,                   // section size

            0x01,                   // 1 table

            0x70,                   // elem type: funcref
            0x00,                   // max absent
            0x0A,                   // min = 10
        ];

        let module = Module::decode(&bytes).unwrap();

        assert_eq!(module.tables.len(), 1);
        assert!(matches!(module.tables[0].table_type.elem_type, ElemType::FuncRef));
        assert_eq!(module.tables[0].table_type.limits.min, 10);
        assert_eq!(module.tables[0].table_type.limits.max, None);
    }

    #[test]
    fn decode_table_section_with_max() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x04,                   // table section id
            0x05,                   // section size

            0x01,                   // 1 table

            0x70,                   // elem type: funcref
            0x01,                   // max present
            0x05,                   // min = 5
            0x0A,                   // max = 10
        ];

        let module = Module::decode(&bytes).unwrap();

        assert_eq!(module.tables.len(), 1);
        assert!(matches!(module.tables[0].table_type.elem_type, ElemType::FuncRef));
        assert_eq!(module.tables[0].table_type.limits.min, 5);
        assert_eq!(module.tables[0].table_type.limits.max, Some(10));
    }

    #[test]
    fn decode_table_section_invalid_elem_type() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x04,                   // table section id
            0x04,                   // section size

            0x01,                   // 1 table

            0x71,                   // invalid elem type
            0x00,                   // max absent
            0x01,                   // min
        ];

        assert!(Module::decode(&bytes)
            .is_err_and(|e| e == DecodeError::InvalidElemType));
    }

    #[test]
    fn decode_table_section_invalid_limits_flag() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x04,                   // table section id
            0x04,                   // section size

            0x01,                   // 1 table

            0x70,                   // elem type: funcref
            0x02,                   // invalid limits flag
            0x01,                   // min
        ];

        assert!(Module::decode(&bytes)
            .is_err_and(|e| e == DecodeError::InvalidLimitsFlag));
    }

    #[test]
    fn decode_table_section_multiple() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x04,                   // table section id
            0x07,                   // section size

            0x02,                   // 2 tables

            // table 1
            0x70,                   // elem type: funcref
            0x00,                   // max absent
            0x01,                   // min = 1

            // table 2
            0x70,                   // elem type: funcref
            0x00,                   // max absent
            0x02,                   // min = 2
        ];

        assert!(Module::decode(&bytes)
            .is_err_and(|e| e == DecodeError::InvalidTableCount));
    }

    #[test]
    fn decode_memory_section() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x05,                   // memory section id
            0x03,                   // section size

            0x01,                   // 1 memory

            0x00,                   // max absent
            0x0A,                   // min = 10
        ];

        let module = Module::decode(&bytes).unwrap();

        assert_eq!(module.mems.len(), 1);
        assert_eq!(module.mems[0].mem_type.min, 10);
        assert_eq!(module.mems[0].mem_type.max, None);
    }

    #[test]
    fn decode_memory_section_with_max() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x05,                   // memory section id
            0x04,                   // section size

            0x01,                   // 1 memory

            0x01,                   // max present
            0x05,                   // min = 5
            0x0A,                   // max = 10
        ];

        let module = Module::decode(&bytes).unwrap();

        assert_eq!(module.mems.len(), 1);
        assert_eq!(module.mems[0].mem_type.min, 5);
        assert_eq!(module.mems[0].mem_type.max, Some(10));
    }

    #[test]
    fn decode_memory_section_invalid_limits_flag() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x05,                   // memory section id
            0x03,                   // section size

            0x01,                   // 1 memory

            0x02,                   // invalid limits flag
            0x01,                   // min
        ];

        assert!(Module::decode(&bytes)
            .is_err_and(|e| e == DecodeError::InvalidLimitsFlag));
    }

    #[test]
    fn decode_memory_section_multiple() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x05,                   // memory section id
            0x05,                   // section size

            0x02,                   // 2 memories

            // memory 1
            0x00,                   // max absent
            0x01,                   // min = 1

            // memory 2
            0x00,                   // max absent
            0x02,                   // min = 2
        ];

        assert!(Module::decode(&bytes)
            .is_err_and(|e| e == DecodeError::InvalidMemoryCount));
    }
}