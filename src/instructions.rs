use crate::{decoder::Decoder, definitions::ValType, errors::DecodeError};

/// Wasm expression outline.
pub type Expr = Vec<Instr>; // should be ended with 0x0B byte.

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

    // Numeric Instructions
    I32Const(i32),
    I64Const(i64),
    F32Const(f32),
    F64Const(f64),
    I32Eqz,
    I32Eq,
    I32Ne,
    I32LtS,
    I32LtU,
    I32GtS,
    I32GtU,
    I32LeS,
    I32LeU,
    I32GeS,
    I32GeU,
    I64Eqz,
    I64Eq,
    I64Ne,
    I64LtS,
    I64LtU,
    I64GtS,
    I64GtU,
    I64LeS,
    I64LeU,
    I64GeS,
    I64GeU,
    F32Eq,
    F32Ne,
    F32Lt,
    F32Gt,
    F32Le,
    F32Ge,
    F64Eq,
    F64Ne,
    F64Lt,
    F64Gt,
    F64Le,
    F64Ge,
    I32Clz,
    I32Ctz,
    I32Popcnt,
    I32Add,
    I32Sub,
    I32Mul,
    I32DivS,
    I32DivU,
    I32RemS,
    I32RemU,
    I32And,
    I32Or,
    I32Xor,
    I32Shl,
    I32ShrS,
    I32ShrU,
    I32Rotl,
    I32Rotr,
    I64Clz,
    I64Ctz,
    I64Popcnt,
    I64Add,
    I64Sub,
    I64Mul,
    I64DivS,
    I64DivU,
    I64RemS,
    I64RemU,
    I64And,
    I64Or,
    I64Xor,
    I64Shl,
    I64ShrS,
    I64ShrU,
    I64Rotl,
    I64Rotr,
    F32Abs,
    F32Neg,
    F32Ceil,
    F32Floor,
    F32Trunc,
    F32Nearest,
    F32Sqrt,
    F32Add,
    F32Sub,
    F32Mul,
    F32Div,
    F32Min,
    F32Max,
    F32Copysign,
    F64Abs,
    F64Neg,
    F64Ceil,
    F64Floor,
    F64Trunc,
    F64Nearest,
    F64Sqrt,
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    F64Min,
    F64Max,
    F64Copysign,
    I32WrapI64,
    I32TruncF32S,
    I32TruncF32U,
    I32TruncF64S,
    I32TruncF64U,
    I64ExtendI32S,
    I64ExtendI32U,
    I64TruncF32S,
    I64TruncF32U,
    I64TruncF64S,
    I64TruncF64U,
    F32ConvertI32S,
    F32ConvertI32U,
    F32ConvertI64S,
    F32ConvertI64U,
    F32DemoteF64,
    F64ConvertI32S,
    F64ConvertI32U,
    F64ConvertI64S,
    F64ConvertI64U,
    F64PromoteF32,
    I32ReinterpretF32,
    I64ReinterpretF64,
    F32ReinterpretI32,
    F64ReinterpretI64,
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
            0x41 => Ok(Self::I32Const(decoder.read_i32()?)),
            0x42 => Ok(Self::I64Const(decoder.read_i64()?)),
            0x43 => Ok(Self::F32Const(decoder.read_f32()?)),
            0x44 => Ok(Self::F64Const(decoder.read_f64()?)),
            0x45 => Ok(Self::I32Eqz),
            0x46 => Ok(Self::I32Eq),
            0x47 => Ok(Self::I32Ne),
            0x48 => Ok(Self::I32LtS),
            0x49 => Ok(Self::I32LtU),
            0x4A => Ok(Self::I32GtS),
            0x4B => Ok(Self::I32GtU),
            0x4C => Ok(Self::I32LeS),
            0x4D => Ok(Self::I32LeU),
            0x4E => Ok(Self::I32GeS),
            0x4F => Ok(Self::I32GeU),
            0x50 => Ok(Self::I64Eqz),
            0x51 => Ok(Self::I64Eq),
            0x52 => Ok(Self::I64Ne),
            0x53 => Ok(Self::I64LtS),
            0x54 => Ok(Self::I64LtU),
            0x55 => Ok(Self::I64GtS),
            0x56 => Ok(Self::I64GtU),
            0x57 => Ok(Self::I64LeS),
            0x58 => Ok(Self::I64LeU),
            0x59 => Ok(Self::I64GeS),
            0x5A => Ok(Self::I64GeU),
            0x5B => Ok(Self::F32Eq),
            0x5C => Ok(Self::F32Ne),
            0x5D => Ok(Self::F32Lt),
            0x5E => Ok(Self::F32Gt),
            0x5F => Ok(Self::F32Le),
            0x60 => Ok(Self::F32Ge),
            0x61 => Ok(Self::F64Eq),
            0x62 => Ok(Self::F64Ne),
            0x63 => Ok(Self::F64Lt),
            0x64 => Ok(Self::F64Gt),
            0x65 => Ok(Self::F64Le),
            0x66 => Ok(Self::F64Ge),
            0x67 => Ok(Self::I32Clz),
            0x68 => Ok(Self::I32Ctz),
            0x69 => Ok(Self::I32Popcnt),
            0x6A => Ok(Self::I32Add),
            0x6B => Ok(Self::I32Sub),
            0x6C => Ok(Self::I32Mul),
            0x6D => Ok(Self::I32DivS),
            0x6E => Ok(Self::I32DivU),
            0x6F => Ok(Self::I32RemS),
            0x70 => Ok(Self::I32RemU),
            0x71 => Ok(Self::I32And),
            0x72 => Ok(Self::I32Or),
            0x73 => Ok(Self::I32Xor),
            0x74 => Ok(Self::I32Shl),
            0x75 => Ok(Self::I32ShrS),
            0x76 => Ok(Self::I32ShrU),
            0x77 => Ok(Self::I32Rotl),
            0x78 => Ok(Self::I32Rotr),
            0x79 => Ok(Self::I64Clz),
            0x7A => Ok(Self::I64Ctz),
            0x7B => Ok(Self::I64Popcnt),
            0x7C => Ok(Self::I64Add),
            0x7D => Ok(Self::I64Sub),
            0x7E => Ok(Self::I64Mul),
            0x7F => Ok(Self::I64DivS),
            0x80 => Ok(Self::I64DivU),
            0x81 => Ok(Self::I64RemS),
            0x82 => Ok(Self::I64RemU),
            0x83 => Ok(Self::I64And),
            0x84 => Ok(Self::I64Or),
            0x85 => Ok(Self::I64Xor),
            0x86 => Ok(Self::I64Shl),
            0x87 => Ok(Self::I64ShrS),
            0x88 => Ok(Self::I64ShrU),
            0x89 => Ok(Self::I64Rotl),
            0x8A => Ok(Self::I64Rotr),
            0x8B => Ok(Self::F32Abs),
            0x8C => Ok(Self::F32Neg),
            0x8D => Ok(Self::F32Ceil),
            0x8E => Ok(Self::F32Floor),
            0x8F => Ok(Self::F32Trunc),
            0x90 => Ok(Self::F32Nearest),
            0x91 => Ok(Self::F32Sqrt),
            0x92 => Ok(Self::F32Add),
            0x93 => Ok(Self::F32Sub),
            0x94 => Ok(Self::F32Mul),
            0x95 => Ok(Self::F32Div),
            0x96 => Ok(Self::F32Min),
            0x97 => Ok(Self::F32Max),
            0x98 => Ok(Self::F32Copysign),
            0x99 => Ok(Self::F64Abs),
            0x9A => Ok(Self::F64Neg),
            0x9B => Ok(Self::F64Ceil),
            0x9C => Ok(Self::F64Floor),
            0x9D => Ok(Self::F64Trunc),
            0x9E => Ok(Self::F64Nearest),
            0x9F => Ok(Self::F64Sqrt),
            0xA0 => Ok(Self::F64Add),
            0xA1 => Ok(Self::F64Sub),
            0xA2 => Ok(Self::F64Mul),
            0xA3 => Ok(Self::F64Div),
            0xA4 => Ok(Self::F64Min),
            0xA5 => Ok(Self::F64Max),
            0xA6 => Ok(Self::F64Copysign),
            0xA7 => Ok(Self::I32WrapI64),
            0xA8 => Ok(Self::I32TruncF32S),
            0xA9 => Ok(Self::I32TruncF32U),
            0xAA => Ok(Self::I32TruncF64S),
            0xAB => Ok(Self::I32TruncF64U),
            0xAC => Ok(Self::I64ExtendI32S),
            0xAD => Ok(Self::I64ExtendI32U),
            0xAE => Ok(Self::I64TruncF32S),
            0xAF => Ok(Self::I64TruncF32U),
            0xB0 => Ok(Self::I64TruncF64S),
            0xB1 => Ok(Self::I64TruncF64U),
            0xB2 => Ok(Self::F32ConvertI32S),
            0xB3 => Ok(Self::F32ConvertI32U),
            0xB4 => Ok(Self::F32ConvertI64S),
            0xB5 => Ok(Self::F32ConvertI64U),
            0xB6 => Ok(Self::F32DemoteF64),
            0xB7 => Ok(Self::F64ConvertI32S),
            0xB8 => Ok(Self::F64ConvertI32U),
            0xB9 => Ok(Self::F64ConvertI64S),
            0xBA => Ok(Self::F64ConvertI64U),
            0xBB => Ok(Self::F64PromoteF32),
            0xBC => Ok(Self::I32ReinterpretF32),
            0xBD => Ok(Self::I64ReinterpretF64),
            0xBE => Ok(Self::F32ReinterpretI32),
            0xBF => Ok(Self::F64ReinterpretI64),

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
            // read 0x05 (else opcode)
            decoder.match_byte(0x05, DecodeError::InvalidIfThenInstr)?;

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