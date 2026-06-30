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

            // skip custom sections
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
                Section::Global => self.decode_global_section(&mut section)?,
                Section::Export => self.decode_export_section(&mut section)?,
                Section::Start => self.start = Some(section.read_u32()?),
                Section::Element => self.decode_element_section(&mut section)?,
                Section::Code => self.decode_code_section(&mut section)?,
                Section::Data => self.decode_data_section(&mut section)?,
                Section::Custom => (),
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

    /// Decodes the global section in the module.
    fn decode_global_section(&mut self, decoder: &mut Decoder) -> Result<(), DecodeError> {
        let num_globals = decoder.read_u32()? as usize;

        self.globals.reserve_exact(num_globals);

        for _ in 0..num_globals {
            self.globals.push(Global::decode(decoder)?);
        }

        Ok(())
    }

    /// Decodes the export section in the module.
    fn decode_export_section(&mut self, decoder: &mut Decoder) -> Result<(), DecodeError> {
        let num_exports = decoder.read_u32()? as usize;

        self.exports.reserve_exact(num_exports);

        for _ in 0..num_exports {
            self.exports.push(Export::decode(decoder)?);
        }

        Ok(())
    }

    /// Decodes the element section in the module.
    fn decode_element_section(&mut self, decoder: &mut Decoder) -> Result<(), DecodeError> {
        let num_elements = decoder.read_u32()? as usize;

        self.elem.reserve_exact(num_elements);

        for _ in 0..num_elements {
            self.elem.push(Elem::decode(decoder)?);
        }

        Ok(())
    }

    /// Decodes the code section in the module.
    fn decode_code_section(&mut self, decoder: &mut Decoder) -> Result<(), DecodeError> {
        let num_funcs = decoder.read_u32()? as usize;

        // there should be an exact match
        if num_funcs != self.funcs.len() {
            return Err(DecodeError::InvalidFunctionCount);
        }
        
        for func_idx in 0..num_funcs {
            let size = decoder.read_u32()? as usize;
            let start = decoder.pos();

            self.funcs[func_idx].decode_locals_body(decoder)?;

            // we should have an exact size
            if decoder.pos() - start != size {
                return Err(DecodeError::MalformedCodeSize);
            }
        }

        Ok(())
    }

    /// Decodes the data section in the module.
    fn decode_data_section(&mut self, decoder: &mut Decoder) -> Result<(), DecodeError> {
        let num_data = decoder.read_u32()? as usize;

        self.data.reserve_exact(num_data);

        for _ in 0..num_data {
            self.data.push(Data::decode(decoder)?);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::instructions::Instr;
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
        assert!(matches!(module.imports[0].desc, ImportDesc::Func(0)));
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
            ImportDesc::Global(g) => {
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
            .is_err_and(|e| e == DecodeError::InvalidImportDesc));
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

    #[test]
    fn test_decode_global_section() {
        let bytes: &[u8] = &[
            0x00, 0x61, 0x73, 0x6D, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x06, // global section
            0x06, // section size

            0x01, // number of globals

            0x7F, // i32
            0x00, // const
            0x41, 0x2A, // i32.const 42
            0x0B, // end
        ];

        let module = Module::decode(bytes).unwrap();

        assert_eq!(module.globals.len(), 1);

        assert!(matches!(module.globals[0].global_type.val_type, ValType::I32));
        assert!(matches!(module.globals[0].global_type.mutability, Mutability::Const));

        assert_eq!(module.globals[0].init.instructions.len(), 1);
        assert!(matches!(module.globals[0].init.instructions[0], Instr::I32Const(42)));
    }

    #[test]
    fn test_decode_multiple_globals() {
        let bytes: &[u8] = &[
            0x00, 0x61, 0x73, 0x6D, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x06, // global section
            0x0B, // section size

            0x02, // number of globals

            0x7F, // i32
            0x00, // const
            0x41, 0x01, // i32.const 1
            0x0B, // end

            0x7E, // i64
            0x01, // var
            0x42, 0x02, // i64.const 2
            0x0B, // end
        ];

        let module = Module::decode(bytes).unwrap();

        assert_eq!(module.globals.len(), 2);

        assert!(matches!(module.globals[0].global_type.val_type, ValType::I32));
        assert!(matches!(module.globals[0].global_type.mutability, Mutability::Const));
        assert!(matches!(module.globals[0].init.instructions[0], Instr::I32Const(1)));

        assert!(matches!(module.globals[1].global_type.val_type, ValType::I64));
        assert!(matches!(module.globals[1].global_type.mutability, Mutability::Var));
        assert!(matches!(module.globals[1].init.instructions[0], Instr::I64Const(2)));
    }

    #[test]
    fn test_decode_non_const_global_expr() {
        let bytes: &[u8] = &[
            0x00, 0x61, 0x73, 0x6D, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x06, // global section
            0x07, // section size

            0x01, // number of globals

            0x7F, // i32
            0x00, // const
            0x41, 0x01, // i32.const 1
            0x1A, // drop
            0x0B, // end
        ];

        assert!(matches!(
            Module::decode(bytes),
            Err(DecodeError::InvalidNonConstExpr)
        ));
    }

    #[test]
    fn decode_export_section_func() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x07,                   // export section id
            0x07,                   // section size

            0x01,                   // 1 export

            // export "add" (func index 0)
            0x03, 0x61, 0x64, 0x64, // export name len=3, "add"
            0x00,                   // desc: func
            0x00,                   // func index: 0
        ];

        let module = Module::decode(&bytes).unwrap();
        assert_eq!(module.exports.len(), 1);
        assert_eq!(module.exports[0].name, "add");
        assert!(matches!(module.exports[0].desc, ExportDesc::Func(0)));
    }

    #[test]
    fn test_decode_multiple_exports() {
        let bytes: &[u8] = &[
            0x00, 0x61, 0x73, 0x6D, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x07,                   // export section id
            0x11,                   // section size

            0x04,                   // 4 exports

            0x01, 0x61,             // export name len=1, "a"
            0x00,                   // desc: func
            0x00,                   // func index: 0

            0x01, 0x62,             // export name len=1, "b"
            0x01,                   // desc: table
            0x01,                   // table index: 1

            0x01, 0x63,             // export name len=1, "c"
            0x02,                   // desc: mem
            0x02,                   // mem index: 2

            0x01, 0x64,             // export name len=1, "d"
            0x03,                   // desc: global
            0x03,                   // global index: 3
        ];

        let module = Module::decode(bytes).unwrap();

        assert_eq!(module.exports.len(), 4);

        assert_eq!(module.exports[0].name, "a");
        assert!(matches!(module.exports[0].desc, ExportDesc::Func(0)));

        assert_eq!(module.exports[1].name, "b");
        assert!(matches!(module.exports[1].desc, ExportDesc::Table(1)));

        assert_eq!(module.exports[2].name, "c");
        assert!(matches!(module.exports[2].desc, ExportDesc::Mem(2)));

        assert_eq!(module.exports[3].name, "d");
        assert!(matches!(module.exports[3].desc, ExportDesc::Global(3)));
    }

    #[test]
    fn decode_export_section_invalid_desc() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x07,                   // export section id
            0x04,                   // section size

            0x01,                   // 1 export

            // export "x" (invalid desc)
            0x01, 0x78,             // export name len=1, "x"
            0x04,                   // desc: invalid
        ];

        assert!(Module::decode(&bytes)
            .is_err_and(|e| e == DecodeError::InvalidExportDesc));
    }

    #[test]
    fn decode_export_section_invalid_utf8() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x07,                   // export section id
            0x04,                   // section size

            0x01,                   // 1 export

            // invalid UTF-8 export name
            0x02, 0xFF, 0xFE,       // export name len=2, invalid UTF-8
        ];

        assert!(Module::decode(&bytes)
            .is_err_and(|e| e == DecodeError::InvalidUTF8Name));
    }

    #[test]
    fn decode_start_section() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x08,                   // start section id
            0x01,                   // section size

            0x02,                   // start func index 2
        ];

        let module = Module::decode(&bytes).unwrap();

        assert_eq!(module.start, Some(2));
    }

    #[test]
    fn decode_element_section() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x09,                   // element section id
            0x08,                   // section size

            0x01,                   // 1 element segment

            0x00,                   // table index: 0
            0x41, 0x05,             // i32.const 5
            0x0B,                   // end
            0x02,                   // 2 function indices
            0x03,                   // func index 3
            0x04,                   // func index 4
        ];

        let module = Module::decode(&bytes).unwrap();

        assert_eq!(module.elem.len(), 1);
        assert_eq!(module.elem[0].table_idx, 0);
        assert!(matches!(module.elem[0].offset.instructions[0], Instr::I32Const(5)));
        assert_eq!(module.elem[0].init.len(), 2);
        assert_eq!(module.elem[0].init[0], 3);
        assert_eq!(module.elem[0].init[1], 4);
    }

    #[test]
    fn decode_element_section_invalid_table_index() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x09,                   // element section id
            0x06,                   // section size

            0x01,                   // 1 element segment

            0x01,                   // invalid table index: 1
            0x41, 0x00,             // i32.const 0
            0x0B,                   // end
            0x00,                   // 0 function indices
        ];

        assert!(Module::decode(&bytes)
            .is_err_and(|e| e == DecodeError::InvalidTableIndex));
    }

    #[test]
    fn decode_element_section_non_const_expr() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x09,                   // element section id
            0x07,                   // section size

            0x01,                   // 1 element segment

            0x00,                   // table index: 0
            0x41, 0x01,             // i32.const 1
            0x1A,                   // drop (makes it non-const)
            0x0B,                   // end
            0x00,                   // 0 function indices
        ];

        assert!(Module::decode(&bytes)
            .is_err_and(|e| e == DecodeError::InvalidNonConstExpr));
    }

    #[test]
    fn decode_element_section_multiple() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x09,                   // element section id
            0x0C,                   // section size

            0x02,                   // 2 element segments

            // segment 1
            0x00,                   // table index: 0
            0x41, 0x00,             // i32.const 0
            0x0B,                   // end
            0x00,                   // 0 function indices

            // segment 2
            0x00,                   // table index: 0
            0x41, 0x0A,             // i32.const 10
            0x0B,                   // end
            0x01,                   // 1 function index
            0x0F,                   // func index 15
        ];

        let module = Module::decode(&bytes).unwrap();

        assert_eq!(module.elem.len(), 2);

        assert_eq!(module.elem[0].table_idx, 0);
        assert!(matches!(module.elem[0].offset.instructions[0], Instr::I32Const(0)));
        assert_eq!(module.elem[0].init.len(), 0);

        assert_eq!(module.elem[1].table_idx, 0);
        assert!(matches!(module.elem[1].offset.instructions[0], Instr::I32Const(10)));
        assert_eq!(module.elem[1].init.len(), 1);
        assert_eq!(module.elem[1].init[0], 15);
    }

    #[test]
    fn decode_code_section() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x03,                   // function section id
            0x02,                   // section size
            0x01,                   // 1 function declared
            0x00,                   // type index 0

            0x0A,                   // code section id
            0x06,                   // section size
            0x01,                   // 1 code entry

            0x04,                   // function size: 4 bytes
            0x01,                   // 1 local group
            0x02,                   // 2 locals in this group
            0x7F,                   // type: i32
            0x0B,                   // end instruction
        ];

        let module = Module::decode(&bytes).unwrap();

        assert_eq!(module.funcs.len(), 1);
        
        // Locals should be expanded from (count: 2, type: i32)
        assert_eq!(module.funcs[0].locals.len(), 2);
        assert!(matches!(module.funcs[0].locals[0], ValType::I32));
        assert!(matches!(module.funcs[0].locals[1], ValType::I32));
    }

    #[test]
    fn decode_code_section_malformed_size_too_small() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x03, 0x02, 0x01, 0x00, // function section: 1 func

            0x0A,                   // code section id
            0x06,                   // section size
            0x01,                   // 1 code entry

            0x02,                   // function size: 2 bytes (WRONG, actual is 4)
            0x01, 0x02, 0x7F, 0x0B, // 4 bytes of actual function data
        ];

        assert!(Module::decode(&bytes)
            .is_err_and(|e| e == DecodeError::MalformedCodeSize));
    }

    #[test]
    fn decode_code_section_malformed_size_too_large() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x03, 0x02, 0x01, 0x00, // function section: 1 func

            0x0A,                   // code section id
            0x06,                   // section size
            0x01,                   // 1 code entry

            0x07,                   // function size: 7 bytes (WRONG, actual is 4)
            0x01, 0x02, 0x7F, 0x0B, // 4 bytes of actual function data
        ];

        assert!(Module::decode(&bytes)
            .is_err_and(|e| e == DecodeError::MalformedCodeSize));
    }

    #[test]
    fn decode_code_section_invalid_function_count() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x03, 0x02, 0x01, 0x00, // function section: 1 func

            0x0A,                   // code section id
            0x0B,                   // section size
            0x02,                   // 2 code entries (WRONG, should match the 1 func above)

            // entry 1
            0x04, 0x01, 0x02, 0x7F, 0x0B, 
            // entry 2
            0x04, 0x01, 0x02, 0x7F, 0x0B, 
        ];

        assert!(Module::decode(&bytes)
            .is_err_and(|e| e == DecodeError::InvalidFunctionCount));
    }

    #[test]
    fn decode_data_section() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x0B,                   // data section id
            0x08,                   // section size

            0x01,                   // 1 data segment

            0x00,                   // memory index: 0
            0x41, 0x05,             // i32.const 5
            0x0B,                   // end
            0x02,                   // 2 bytes of data
            0xAA, 0xBB,             // init data
        ];

        let module = Module::decode(&bytes).unwrap();

        assert_eq!(module.data.len(), 1);
        assert_eq!(module.data[0].mem_idx, 0);
        assert!(matches!(module.data[0].offset.instructions[0], Instr::I32Const(5)));
        assert_eq!(module.data[0].init.len(), 2);
        assert_eq!(module.data[0].init, vec![0xAA, 0xBB]);
    }

    #[test]
    fn decode_data_section_invalid_memory_index() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x0B,                   // data section id
            0x06,                   // section size

            0x01,                   // 1 data segment

            0x01,                   // invalid memory index: 1
            0x41, 0x00,             // i32.const 0
            0x0B,                   // end
            0x00,                   // 0 bytes of data
        ];

        assert!(Module::decode(&bytes)
            .is_err_and(|e| e == DecodeError::InvalidMemoryIndex));
    }

    #[test]
    fn decode_data_section_non_const_expr() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x0B,                   // data section id
            0x07,                   // section size

            0x01,                   // 1 data segment

            0x00,                   // memory index: 0
            0x41, 0x01,             // i32.const 1
            0x1A,                   // drop (makes it non-const)
            0x0B,                   // end
            0x00,                   // 0 bytes of data
        ];

        assert!(Module::decode(&bytes)
            .is_err_and(|e| e == DecodeError::InvalidNonConstExpr));
    }

    #[test]
    fn decode_data_section_multiple() {
        let bytes = [
            0x00, 0x61, 0x73, 0x6d, // magic
            0x01, 0x00, 0x00, 0x00, // version

            0x0B,                   // data section id
            0x0E,                   // section size

            0x02,                   // 2 data segments

            // segment 1
            0x00,                   // memory index: 0
            0x41, 0x00,             // i32.const 0
            0x0B,                   // end
            0x01,                   // 1 byte of data
            0xFF,                   // init data

            // segment 2
            0x00,                   // memory index: 0
            0x41, 0x0A,             // i32.const 10
            0x0B,                   // end
            0x02,                   // 2 bytes of data
            0xAA, 0xBB,             // init data
        ];

        let module = Module::decode(&bytes).unwrap();

        assert_eq!(module.data.len(), 2);

        assert_eq!(module.data[0].mem_idx, 0);
        assert!(matches!(module.data[0].offset.instructions[0], Instr::I32Const(0)));
        assert_eq!(module.data[0].init, vec![0xFF]);

        assert_eq!(module.data[1].mem_idx, 0);
        assert!(matches!(module.data[1].offset.instructions[0], Instr::I32Const(10)));
        assert_eq!(module.data[1].init, vec![0xAA, 0xBB]);
    }
}