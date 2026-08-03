use std::rc::Rc;

use crate::{binary::reader::Reader, errors::{DecodingError, ExecutionError, TrapReason, ValidationError}, execution::Executor, runtime::{Addr, FuncInstance, ModuleInstance, Store, Val}, structure::{ImportDesc, Mem, Mutability, ValType, types::{BlockType, LabelKind, MemArg, OpenLabel, PatchSite}}, validation::Validator};

/// Wasm expression.
#[derive(Default, Debug, PartialEq, Clone)]
pub struct Expr {
    /// Sequence of instructions.
    pub instructions: Vec<Instr>,
}

impl Expr {
    pub fn decode(reader: &mut Reader) -> Result<Self, DecodingError> {
        Ok(Self {
            instructions: Instr::decode_sequence(reader)?
        })
    }

    /// Validates the expression as a function body.
    pub(crate) fn validate(&self, validator: &mut Validator, end_types: Vec<ValType>) -> Result<(), ValidationError> {
        validator.opds.clear();
        validator.ctrls.clear();

        validator.push_ctrl(end_types.clone(), end_types);

        for instr in &self.instructions {
            instr.validate(validator)?;
        }

        validator.pop_ctrl()?;

        Ok(())
    }

    /// Validates the expression as a constant expression.
    pub(crate) fn validate_const_expr(&self, validator: &mut Validator, end_type: Option<ValType>) -> Result<(), ValidationError> {
        validator.opds.clear();
        validator.ctrls.clear();

        let end_types: Vec<ValType> = end_type.into_iter().collect();

        validator.push_ctrl(end_types.clone(), end_types);

        for instr in &self.instructions {
            match instr {
                Instr::I32Const(_) | Instr::I64Const(_)
                | Instr::F32Const(_) | Instr::F64Const(_)
                => instr.validate(validator)?,
                
                Instr::GlobalGet(global_idx) => {
                    let global_idx = *global_idx as usize;

                    /*
                        the global used to initialize the const expr., must be an imported global.
                        imported globals come first in the list of globals, so any index in [0, num_imported_globals-1]
                        is fine.
                    */
                    let num_imported_globals = validator.module.imports
                        .iter()
                        .filter(|import| matches!(import.desc, ImportDesc::Global(_)))
                        .count();

                    if num_imported_globals <= global_idx {
                        return Err(ValidationError::InvalidNonImportedGlobal { index: global_idx });
                    }

                    let global = validator.globals
                        .get(global_idx)
                        .ok_or(ValidationError::UndefinedGlobal { index: global_idx })?; // shouldn't err

                    // ensure global is const
                    if global.mutability != Mutability::Const {
                        return Err(ValidationError::NonConstantInstruction { actual: instr.clone() });
                    }

                    instr.validate(validator)?
                }

                _ => Err(ValidationError::NonConstantInstruction { actual: instr.clone() })?
            }
        }

        validator.pop_ctrl()?;

        Ok(())
    }

    /// Executes the expression as a constant expression.
    pub(crate) fn execute_const_expr(&self, store: &Store, imported_global_addrs: &[Addr]) -> Val {
        match &self.instructions[0] {
            Instr::I32Const(v) => Val::I32(*v),
            Instr::I64Const(v) => Val::I64(*v),
            Instr::F32Const(v) => Val::F32(*v),
            Instr::F64Const(v) => Val::F64(*v),
            Instr::GlobalGet(global_idx) => store.globals[imported_global_addrs[*global_idx as usize]].value,
            _ => unreachable!("expression should be constant.")
        }
    }
}

/// Wasm instructions.
#[derive(Debug, Clone, PartialEq)]
pub enum Instr {
    Unreachable,
    Nop,
    Block(BlockType),
    Loop(BlockType),
    If(BlockType, u32),
    Else(u32),
    End,
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
    /// Decodes a sequence of instructions until it reaches the final end instruction.
    pub fn decode_sequence(reader: &mut Reader) -> Result<Vec<Instr>, DecodingError> {
        // code sequence
        let mut code: Vec<Instr> = Vec::new();

        // currently open labels that need to be resolved.
        let mut labels: Vec<OpenLabel> = Vec::new();

        loop {
            match reader.read_byte()? {
                0x02 => {
                    let block_type = BlockType::decode(reader)?;

                    labels.push(OpenLabel::new(LabelKind::Block, code.len(), None));
                    code.push(Self::Block(block_type));
                },
                
                0x03 => {
                    let block_type = BlockType::decode(reader)?;

                    labels.push(OpenLabel::new(LabelKind::Loop, code.len(), None));
                    code.push(Self::Loop(block_type));
                },

                0x04 => {
                    let block_type = BlockType::decode(reader)?;

                    labels.push(OpenLabel::new(LabelKind::If, code.len(), None));
                    code.push(Self::If(block_type, u32::MAX)); // for now set the 'else' target to a placeholder.
                },

                0x05 => {
                    let open = labels.last_mut().ok_or(DecodingError::OpenLabelStackUnderflow)?;

                    // if the label isn't an 'if', the module is malformed
                    if !matches!(open.kind, LabelKind::If) {
                        return Err(DecodingError::InvalidIfThenInstr { actual: 0x05 });
                    }

                    let else_body_start = code.len() as u32 + 1;

                    // set the If's else target to the start of the else's body
                    if let Self::If(_, target) = &mut code[open.start_pc] {
                        *target = else_body_start;
                    }

                    open.else_pc = Some(code.len());
                    code.push(Self::Else(u32::MAX)); // set end of if-then-else target to placeholder
                },

                0x0B => {
                    // end instruction
                    let open = labels.pop();

                    code.push(Self::End);
                    let exit_pc = code.len() as u32;

                    match open {
                        Some(open) => {
                            // resolve branch target for label
                            let branch_target = match open.kind {
                                // branches to loops go to the start of the loop
                                LabelKind::Loop => open.start_pc as u32,

                                // branches to everything else go to the end
                                _ => exit_pc,
                            };

                            // resolve all branches to this label
                            for site in open.pending_br {
                                Self::patch_branch(&mut code, site, branch_target);
                            }

                            if matches!(open.kind, LabelKind::If) {
                                if open.else_pc.is_some() {
                                    // if there's an 'else', set its end target to this index
                                    if let Self::Else(target) = &mut code[open.else_pc.unwrap()] {
                                        *target = exit_pc;
                                    }
                                } else if let Self::If(_, target) = &mut code[open.start_pc] {
                                    // if there's no else-block, the if should skip to the end instr if the condition is false.
                                    *target = exit_pc;
                                }
                            }
                        },

                        // instruction sequence is done if there's no labels left.
                        None => break,
                    }
                }

                0x0C => {
                    let depth = reader.read_u32()? as usize;
                    let br_pc = code.len();

                    code.push(Self::Br(u32::MAX));

                    // invalid depths are validation checks, so we just return early.
                    if depth >= labels.len() {
                        continue;
                    }

                    // add Br to list of pending br's for target label
                    let label_index = labels.len() - 1 - depth;
                    labels[label_index].pending_br.push(PatchSite::Br(br_pc));
                },

                0x0D => {
                    let depth = reader.read_u32()? as usize;
                    let br_if_pc = code.len();

                    code.push(Self::BrIf(u32::MAX));

                    if depth >= labels.len() {
                        continue;
                    }

                    let label_index = labels.len() - 1 - depth;
                    labels[label_index].pending_br.push(PatchSite::BrIf(br_if_pc));
                },

                0x0E => {
                    let num_labels = reader.read_u32()? as usize;
                    let depths: Vec<u32> = (0..num_labels)
                        .map(|_| reader.read_u32())
                        .collect::<Result<Vec<_>, _>>()?;

                    let fallback_depth = reader.read_u32()? as usize;
                    let br_table_pc = code.len();

                    code.push(Self::BrTable(vec![u32::MAX; num_labels], u32::MAX)); // push Instr::BrTable with placeholders for the label indices

                    for (entry_index, depth) in depths.iter().enumerate() {
                        if *depth as usize >= labels.len() {
                            continue;
                        }

                        let label_index = labels.len() - 1 - *depth as usize;
                        labels[label_index].pending_br.push(PatchSite::BrTableEntry(br_table_pc, entry_index));
                    }

                    if fallback_depth >= labels.len() {
                        continue;
                    }

                    let default_index = labels.len() - 1 - fallback_depth;
                    labels[default_index].pending_br.push(PatchSite::BrTableDefault(br_table_pc));
                }

                byte => code.push(Self::decode_ncc(byte, reader)?)
            }
        }

        Ok(code)
    }

    /// Decodes non-control-construct related instructions.
    fn decode_ncc(byte: u8, reader: &mut Reader) -> Result<Self, DecodingError> {
        match byte {
            // Control Instructions
                0x00 => Ok(Self::Unreachable),
                0x01 => Ok(Self::Nop),
                0x10 => Ok(Self::Call(reader.read_u32()?)),
                0x11 => {
                    let type_idx = reader.read_u32()?;

                    reader.match_byte(0x00, DecodingError::InvalidCallIndirectInstr { actual: reader.peek_byte()? })?;
                    Ok(Self::CallIndirect(type_idx))
                }

            // Parametric Instructions
                0x1A => Ok(Self::Drop),
                0x1B => Ok(Self::Select),

            // Variable Instructions
                0x20 => Ok(Self::LocalGet(reader.read_u32()?)),
                0x21 => Ok(Self::LocalSet(reader.read_u32()?)),
                0x22 => Ok(Self::LocalTee(reader.read_u32()?)),
                0x23 => Ok(Self::GlobalGet(reader.read_u32()?)),
                0x24 => Ok(Self::GlobalSet(reader.read_u32()?)),

            // Memory Instructions
                0x28 => Ok(Self::I32Load(MemArg::decode(reader)?)),
                0x29 => Ok(Self::I64Load(MemArg::decode(reader)?)),
                0x2A => Ok(Self::F32Load(MemArg::decode(reader)?)),
                0x2B => Ok(Self::F64Load(MemArg::decode(reader)?)),
                0x2C => Ok(Self::I32Load8S(MemArg::decode(reader)?)),
                0x2D => Ok(Self::I32Load8U(MemArg::decode(reader)?)),
                0x2E => Ok(Self::I32Load16S(MemArg::decode(reader)?)),
                0x2F => Ok(Self::I32Load16U(MemArg::decode(reader)?)),
                0x30 => Ok(Self::I64Load8S(MemArg::decode(reader)?)),
                0x31 => Ok(Self::I64Load8U(MemArg::decode(reader)?)),
                0x32 => Ok(Self::I64Load16S(MemArg::decode(reader)?)),
                0x33 => Ok(Self::I64Load16U(MemArg::decode(reader)?)),
                0x34 => Ok(Self::I64Load32S(MemArg::decode(reader)?)),
                0x35 => Ok(Self::I64Load32U(MemArg::decode(reader)?)),
                0x36 => Ok(Self::I32Store(MemArg::decode(reader)?)),
                0x37 => Ok(Self::I64Store(MemArg::decode(reader)?)),
                0x38 => Ok(Self::F32Store(MemArg::decode(reader)?)),
                0x39 => Ok(Self::F64Store(MemArg::decode(reader)?)),
                0x3A => Ok(Self::I32Store8(MemArg::decode(reader)?)),
                0x3B => Ok(Self::I32Store16(MemArg::decode(reader)?)),
                0x3C => Ok(Self::I64Store8(MemArg::decode(reader)?)),
                0x3D => Ok(Self::I64Store16(MemArg::decode(reader)?)),
                0x3E => Ok(Self::I64Store32(MemArg::decode(reader)?)),
                0x3F => {
                    reader.match_byte(0x00, DecodingError::InvalidMemorySizeInstr { actual: reader.peek_byte()? })?;
                    Ok(Self::MemorySize)
                },
                0x40 => {
                    reader.match_byte(0x00, DecodingError::InvalidMemoryGrowInstr { actual: reader.peek_byte()? })?;
                    Ok(Self::MemoryGrow)
                }

            // Numeric Instructions
                0x41 => Ok(Self::I32Const(reader.read_i32()?)),
                0x42 => Ok(Self::I64Const(reader.read_i64()?)),
                0x43 => Ok(Self::F32Const(reader.read_f32()?)),
                0x44 => Ok(Self::F64Const(reader.read_f64()?)),
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

            actual => Err(DecodingError::InvalidInstr { actual })
        }
    }

    /// Patches the branch with the given patch site to go to 'target'.
    fn patch_branch(code: &mut [Instr], site: PatchSite, target: u32) {
        match site {
            PatchSite::Br(pc) => if let Self::Br(t) = &mut code[pc] { *t = target; },
            PatchSite::BrIf(pc) => if let Self::BrIf(t) = &mut code[pc] { *t = target; },
            PatchSite::BrTableEntry(pc, entry) => if let Self::BrTable(labels, _) = &mut code[pc] { labels[entry] = target; },
            PatchSite::BrTableDefault(pc) => if let Self::BrTable(_, default) = &mut code[pc] { *default = target; },
        }
    }

}