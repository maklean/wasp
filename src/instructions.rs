use crate::{decoder::Decoder, definitions::ValType, errors::DecodeError};

/// Wasm expression outline.
pub type Expr = Vec<Instr>; // should be ended with 0x0B byte.

#[repr(u8)]
pub enum Instr {
    // Control Instructions
    Unreachable,
    Nop,
    Block(BlockType, Vec<Instr>),
    Loop(BlockType, Vec<Instr>),
    If(BlockType, Vec<Instr>, Vec<Instr>),
    Br(u32),
    BrIf(u32),
    BrTable(Vec<u32>, u32),
    Return,
    Call(u32),
    CallIndirect(u32),

    // Parametric Instructions
    Drop,
    Select,

    // Variable Instructions
    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),
    GlobalGet(u32),
    GlobalSet(u32),

    // Memory Instructions
    I32Load(MemArg),
    I64Load(MemArg),
    F32Load(MemArg),
    F64Load(MemArg),
    I32Load8S(MemArg),
    I32Load8U(MemArg),
    I32Load16S(MemArg),
    I32Load16U(MemArg),
    I64Load8S(MemArg),
    I64Load8U(MemArg),
    I64Load16S(MemArg),
    I64Load16U(MemArg),
    I64Load32S(MemArg),
    I64Load32U(MemArg),
    I32Store(MemArg),
    I64Store(MemArg),
    F32Store(MemArg),
    F64Store(MemArg),
    I32Store8(MemArg),
    I32Store16(MemArg),
    I64Store8(MemArg),
    I64Store16(MemArg),
    I64Store32(MemArg),
    MemorySize,
    MemoryGrow,
}

impl Instr {
    const CTRL_END_MARKER: u8 = 0x0B;

    pub fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        match decoder.read_byte()? {
            // Control Instructions
            0x00 => Ok(Self::Unreachable),
            0x01 => Ok(Self::Nop),
            0x02 => Ok(Self::Block(BlockType::decode(decoder)?, Self::decode_sequence(decoder)?)),
            0x03 => Ok(Self::Loop(BlockType::decode(decoder)?, Self::decode_sequence(decoder)?)),
            0x04 => Ok(Self::decode_if_instr(decoder)?),
            0x0C => Ok(Self::Br(decoder.read_u32()?)),
            0x0D => Ok(Self::BrIf(decoder.read_u32()?)),
            0x0E => {
                let num_labels = decoder.read_u32()? as usize;
                let mut labels: Vec<u32> = Vec::with_capacity(num_labels);

                for _ in 0..num_labels {
                    labels.push(decoder.read_u32()?);
                }

                Ok(Self::BrTable(labels, decoder.read_u32()?))
            },
            0x0F => Ok(Self::Return),
            0x10 => Ok(Self::Call(decoder.read_u32()?)),
            0x11 => {
                let type_idx = decoder.read_u32()?;

                decoder.match_byte(0x00, DecodeError::InvalidCallIndirectInstr)?;
                Ok(Self::CallIndirect(type_idx))
            }

            // Parametric Instructions
            0x1A => Ok(Self::Drop),
            0x1B => Ok(Self::Select),

            // Variable Instructions
            0x20 => Ok(Instr::LocalGet(decoder.read_u32()?)),
            0x21 => Ok(Instr::LocalSet(decoder.read_u32()?)),
            0x22 => Ok(Instr::LocalTee(decoder.read_u32()?)),
            0x23 => Ok(Instr::GlobalGet(decoder.read_u32()?)),
            0x24 => Ok(Instr::GlobalSet(decoder.read_u32()?)),

            // Memory Instructions
            0x28 => Ok(Instr::I32Load(MemArg::decode(decoder)?)),
            0x29 => Ok(Instr::I64Load(MemArg::decode(decoder)?)),
            0x2A => Ok(Instr::F32Load(MemArg::decode(decoder)?)),
            0x2B => Ok(Instr::F64Load(MemArg::decode(decoder)?)),
            0x2C => Ok(Instr::I32Load8S(MemArg::decode(decoder)?)),
            0x2D => Ok(Instr::I32Load8U(MemArg::decode(decoder)?)),
            0x2E => Ok(Instr::I32Load16S(MemArg::decode(decoder)?)),
            0x2F => Ok(Instr::I32Load16U(MemArg::decode(decoder)?)),
            0x30 => Ok(Instr::I64Load8S(MemArg::decode(decoder)?)),
            0x31 => Ok(Instr::I64Load8U(MemArg::decode(decoder)?)),
            0x32 => Ok(Instr::I64Load16S(MemArg::decode(decoder)?)),
            0x33 => Ok(Instr::I64Load16U(MemArg::decode(decoder)?)),
            0x34 => Ok(Instr::I64Load32S(MemArg::decode(decoder)?)),
            0x35 => Ok(Instr::I64Load32U(MemArg::decode(decoder)?)),
            0x36 => Ok(Instr::I32Store(MemArg::decode(decoder)?)),
            0x37 => Ok(Instr::I64Store(MemArg::decode(decoder)?)),
            0x38 => Ok(Instr::F32Store(MemArg::decode(decoder)?)),
            0x39 => Ok(Instr::F64Store(MemArg::decode(decoder)?)),
            0x3A => Ok(Instr::I32Store8(MemArg::decode(decoder)?)),
            0x3B => Ok(Instr::I32Store16(MemArg::decode(decoder)?)),
            0x3C => Ok(Instr::I64Store8(MemArg::decode(decoder)?)),
            0x3D => Ok(Instr::I64Store16(MemArg::decode(decoder)?)),
            0x3E => Ok(Instr::I64Store32(MemArg::decode(decoder)?)),
            0x3F => {
                decoder.match_byte(0x00, DecodeError::InvalidMemorySizeInstr)?;
                Ok(Self::MemorySize)
            },
            0x40 => {
                decoder.match_byte(0x00, DecodeError::InvalidMemoryGrowInstr)?;
                Ok(Self::MemoryGrow)
            }

            // Numeric Instructions

            _ => Err(DecodeError::InvalidInstr)
        }
    }

    /// Decodes a sequence of instructions (i.e.: (in: instr)* 0x0B)
    fn decode_sequence(decoder: &mut Decoder) -> Result<Vec<Instr>, DecodeError> {
        let mut instr: Vec<Instr> = Vec::new();

        while decoder.peek_byte()? != Self::CTRL_END_MARKER {
            instr.push(Self::decode(decoder)?);
        }

        // read 0x0B (CTRL_END_MARKER)
        decoder.match_byte(Self::CTRL_END_MARKER, DecodeError::ExpectedEndOfCtrlInstr)?;

        Ok(instr)
    }

    /// Decodes an if instruction.
    fn decode_if_instr(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        let block_type = BlockType::decode(decoder)?;

        let mut then_instr: Vec<Instr> = Vec::new();
        let mut else_instr: Vec<Instr> = Vec::new();

        // check for end marker or else opcode
        while !matches!(decoder.peek_byte()?, Self::CTRL_END_MARKER | 0x05) {
            then_instr.push(Self::decode(decoder)?);
        }

        // parse else block if there's an else opcode
        if decoder.peek_byte()? == 0x05 {
            while decoder.peek_byte()? != Self::CTRL_END_MARKER {
                else_instr.push(Self::decode(decoder)?);
            }
        }

        // read 0x0B (CTRL_END_MARKER)
        decoder.match_byte(Self::CTRL_END_MARKER, DecodeError::ExpectedEndOfCtrlInstr)?;

        Ok(Self::If(block_type, then_instr, else_instr))
    }
}

pub enum BlockType {
    Empty,
    Val(ValType)
}

impl BlockType {
    const EMPTY_MARKER: u8 = 0x40;

    /// Decodes a `BlockType`.
    pub fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        match decoder.read_byte()? {
            Self::EMPTY_MARKER => Ok(Self::Empty),
            b => Ok(Self::Val(ValType::try_from(b)?)),
        }
    }
}

pub struct MemArg {
    pub align: u32,
    pub offset: u32,
}

impl MemArg {
    pub fn decode(decoder: &mut Decoder) -> Result<Self, DecodeError> {
        let align = decoder.read_u32()?;
        let offset = decoder.read_u32()?;
        Ok(Self { align, offset })
    }
}