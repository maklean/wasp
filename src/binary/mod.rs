use crate::{binary::reader::Reader, errors::DecodingError, structure::*};

pub(crate) mod reader;

/// Decodes a section that simply reserves space on its corresponding vector
/// and calls DecodeType::decode(reader).
macro_rules! decode_vec_section {
    ($self:ident, $reader:expr, $field: ident, $decode_type: ty, $decode_func: ident) => {{
        let count = $reader.read_u32()? as usize;
        $self.$field.reserve_exact(count);
        for _ in 0..count {
            $self.$field.push(<$decode_type>::$decode_func($reader)?);
        }
    }};
}


/// Parsed/Decoded Wasm module.
#[derive(Default)]
pub struct ParsedModule {
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

impl ParsedModule {
    /// Decodes a Wasm module from its bytes.
    pub fn decode_from_bytes(bytes: &[u8]) -> Result<Self, DecodingError> {
        let mut reader = Reader::new(bytes);

        reader.read_wasm_header()?;

        let mut this: ParsedModule = Default::default();

        this.decode_sections(&mut reader)?;

        Ok(this)
    }

    /// Decodes every defined section in the Wasm module.
    fn decode_sections(&mut self, reader: &mut Reader) -> Result<(), DecodingError> {
        let start = reader.pos();

        let mut last_section = ModuleSection::Custom;
        let mut seen_code_section = false; // to check for defined type section, but undefined code section.

        while !reader.eof() {
            let curr_section = ModuleSection::decode(reader)?;
            let section_size = reader.read_u32()?;

            let mut section_reader = Reader::new(reader.read_bytes(section_size as usize)?);

            // skip custom sections
            if curr_section == ModuleSection::Custom {
                let name_len = section_reader.read_u32()? as usize;
                let pos = section_reader.pos();
                let name_bytes = section_reader.read_bytes(name_len)?;

                std::str::from_utf8(name_bytes)
                    .map_err(|_| DecodingError::InvalidUTF8Name { pos })?;

                continue;
            }

            // excluding custom sections, section IDs have to appear in a monotonic non-decreasing order
            if last_section >= curr_section {
                return Err(DecodingError::InvalidSectionOrder { last: last_section, curr: curr_section });
            }

            match curr_section {
                ModuleSection::Type => decode_vec_section!(self, &mut section_reader, types, FuncType, decode),
                ModuleSection::Import => decode_vec_section!(self, &mut section_reader, imports, Import, decode),
                ModuleSection::Function => decode_vec_section!(self, &mut section_reader, funcs, Func, decode_type_idx),
                ModuleSection::Table => decode_vec_section!(self, &mut section_reader, tables, Table, decode),
                ModuleSection::Memory => decode_vec_section!(self, &mut section_reader, mems, Mem, decode),
                ModuleSection::Global => decode_vec_section!(self, &mut section_reader, globals, Global, decode),
                ModuleSection::Export => decode_vec_section!(self, &mut section_reader, exports, Export, decode),
                ModuleSection::Start => self.start = Some(section_reader.read_u32()?),
                ModuleSection::Element => decode_vec_section!(self, &mut section_reader, elem, Elem, decode),
                ModuleSection::Code => {
                    let num_func_bodies = section_reader.read_u32()? as usize;

                    // the number of functions and function bodies should be the same.
                    if num_func_bodies != self.funcs.len() {
                        return Err(DecodingError::FunctionCodeCountMismatch { func_count: self.funcs.len(), code_count: num_func_bodies });
                    }

                    for func_idx in 0..num_func_bodies {
                        let size = section_reader.read_u32()? as usize;
                        let start = section_reader.pos();

                        self.funcs[func_idx].decode_locals_body(&mut section_reader)?;

                        // the code size should be the declared size
                        let actual_size = section_reader.pos() - start;

                        if actual_size != size {
                            return Err(DecodingError::InvalidFuncCodeSize { expected: size, actual: actual_size })
                        }
                    }

                    seen_code_section = true;
                },
                ModuleSection::Data => decode_vec_section!(self, &mut section_reader, data, Data, decode),
                _ => unreachable!("All decodable sections have matching branches.")
            }

            // if we haven't read the entire section, there's a size mismatch
            if !section_reader.eof() {
                return Err(DecodingError::SectionSizeMismatch { expected: section_size as usize, actual: reader.pos() - start })
            }

            last_section = curr_section;
        }

        // check if we've declared functions, but haven't decoded any bodies
        if !self.funcs.is_empty() && !seen_code_section {
            return Err(DecodingError::FunctionCodeCountMismatch { func_count: self.funcs.len(), code_count: 0 });
        }

        Ok(())
    }
}

/// Wasm module section.
#[derive(PartialEq, PartialOrd, Debug)]
pub enum ModuleSection {
    Custom,
    Type,
    Import,
    Function,
    Table,
    Memory,
    Global,
    Export,
    Start,
    Element,
    Code, 
    Data
}

impl ModuleSection {
    fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        let actual = reader.read_byte()?;

        match actual {
            0x00 => Ok(Self::Custom),
            0x01 => Ok(Self::Type),
            0x02 => Ok(Self::Import),
            0x03 => Ok(Self::Function),
            0x04 => Ok(Self::Table),
            0x05 => Ok(Self::Memory),
            0x06 => Ok(Self::Global),
            0x07 => Ok(Self::Export),
            0x08 => Ok(Self::Start),
            0x09 => Ok(Self::Element),
            0x0A => Ok(Self::Code),
            0x0B => Ok(Self::Data),
            _ => Err(DecodingError::InvalidSectionId { actual }),
        }
    }
}