use std::path::Path;

use crate::{decoder::Decoder, definitions::*, errors::{DecodeError, ValidateError}, validator::Validator};

/// Wasm module representation.
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
    const MAGIC_HEADER: &[u8; 4] = b"\0asm";
    const WASM_1_0_SPEC_VERSION: &[u8; 4] = &[1, 0, 0, 0];

    /// Decodes the Wasm module at the given path.
    pub fn decode_from_file(path: impl AsRef<Path>) -> Result<Self, DecodeError> {
        let bytes = std::fs::read(path)
            .map_err(|_| DecodeError::Io)?;

        Self::decode_from_bytes(&bytes)
    }

    /// Decodes a Wasm module from a slice of bytes.
    pub fn decode_from_bytes(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut decoder = Decoder::new(bytes);

        // read magic header
        if decoder.read_bytes(Self::MAGIC_HEADER.len())? != Self::MAGIC_HEADER {
            return Err(DecodeError::InvalidMagicHeader);
        }

        // read wasm specification version
        if decoder.read_bytes(Self::WASM_1_0_SPEC_VERSION.len())? != Self::WASM_1_0_SPEC_VERSION {
            return Err(DecodeError::InvalidSpecificationVersion);
        }

        let mut this: Module = Default::default();

        // decode sections
        this.decode_sections(&mut decoder)?;

        Ok(this)
    }

    /// Validates the current module.
    pub fn validate(&self) -> Result<(), ValidateError> { Validator::validate(self) }
}

// Specific decoding functions.
impl Module {
    /// Decodes each section in the module.
    fn decode_sections(&mut self, decoder: &mut Decoder) -> Result<(), DecodeError> {
        let mut last_section_id = Section::Custom;
        let mut seen_code_section = false;

        while !decoder.eof() {
            let section_id: Section = Section::try_from(decoder.read_byte()?)?;
            let section_size = decoder.read_u32()?;
            let mut section = Decoder::new(decoder.read_bytes(section_size as usize)?);

            // skip custom sections
            if section_id == Section::Custom {
                let name_len = section.read_u32()? as usize;
                let name_bytes = section.read_bytes(name_len)?;

                std::str::from_utf8(name_bytes)
                    .map_err(|_| DecodeError::InvalidUTF8Name)?;

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
                Section::Code => { self.decode_code_section(&mut section)?; seen_code_section = true; },
                Section::Data => self.decode_data_section(&mut section)?,
                Section::Custom => (),
            }

            // if we haven't read the entire section, there's a size mismatch
            if !section.eof() {
                return Err(DecodeError::SectionSizeMismatch);
            }

            last_section_id = section_id;
        }

        // check if we've declared functions, but haven't decoded any bodies
        if !self.funcs.is_empty() && !seen_code_section {
            return Err(DecodeError::InvalidFunctionCount);
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

        self.tables.reserve_exact(num_tables);

        for _ in 0..num_tables {
            self.tables.push(Table::decode(decoder)?);
        }

        Ok(())
    }

    /// Decodes the memory section in the module.
    fn decode_memory_section(&mut self, decoder: &mut Decoder) -> Result<(), DecodeError> {
        let num_memories = decoder.read_u32()? as usize;

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