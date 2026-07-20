use std::rc::Rc;
use crate::{definitions::{Func, FuncType, Mutability, ValType}, errors::ExecuteError, instructions::{BlockType, Instr, MemArg}};

pub const PAGE_SIZE: usize = 65536;

/// Runtime representation of a Wasm value.
#[derive(Debug, Clone, Copy)]
pub enum Val {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64)
}

impl Val {
    pub fn as_i32(&self) -> i32 { 
        match self {
            Self::I32(v) => *v,
            _ => unreachable!()
        }
    }

    pub fn as_i64(&self) -> i64 { 
        match self {
            Self::I64(v) => *v,
            _ => unreachable!()
        }
    }

    pub fn as_f32(&self) -> f32 { 
        match self {
            Self::F32(v) => *v,
            _ => unreachable!()
        }
    }

    pub fn as_f64(&self) -> f64 { 
        match self {
            Self::F64(v) => *v,
            _ => unreachable!()
        }
    }

    pub fn zero(val_type: ValType) -> Self {
        match val_type {
            ValType::I32 => Self::I32(0),
            ValType::I64 => Self::I64(0),
            ValType::F32 => Self::F32(0.0),
            ValType::F64 => Self::F64(0.0),
            _ => unreachable!()
        }
    }
}

impl TryInto<ValType> for Val {
    type Error = ExecuteError;

    fn try_into(self) -> Result<ValType, Self::Error> {
        match self {
            Self::I32(_) => Ok(ValType::I32),
            Self::I64(_) => Ok(ValType::I64),
            Self::F32(_) => Ok(ValType::F32),
            Self::F64(_) => Ok(ValType::F64),
        }
    }
}

/// Outcome of a computation.
pub enum ExecutionResult {
    Val(Vec<Val>),
    Trap
}

/// Global state of a Wasm program.
pub struct Store {
    /// Function instances.
    pub funcs: Vec<FuncInstance>,

    /// Table instances.
    pub tables: Vec<TableInstance>,

    /// Memory instances.
    pub mems: Vec<MemInstance>,

    /// Global variable instances.
    pub globals: Vec<GlobalInstance>,
}

/// Runtime representation of a Wasm function.
pub enum FuncInstance {
    /// A function defined inside a Wasm module.
    Wasm {
        /// Signature of the function.
        func_type: Rc<FuncType>,

        /// Originating module.
        module: Rc<ModuleInstance>,

        /// Function definition.
        code: Rc<Func>,
    },

    /// A function provided by the host (import).
    Host {
        /// Signature of the function.
        func_type: Rc<FuncType>,

        /// Host function.
        code: Rc<HostFunc>,
    }
}

/// Runtime representation of a Wasm table.
pub struct TableInstance {
    /// Function elements.
    pub elem: Vec<FuncElem>,

    /// Maximum size.
    pub max: Option<u32>,
}

/// Runtime representation of a Wasm linear memory.
pub struct MemInstance {
    /// Bytes.
    pub data: Vec<u8>,

    /// Maximum size.
    pub max: Option<u32>,
}

/// Runtime representation of a Wasm global variable.
pub struct GlobalInstance {
    /// Runtime value.
    pub value: Val,

    /// Mutability.
    pub mutability: Mutability,
}

/// Runtime representation of a module.
pub struct ModuleInstance {
    /// Signature of functions.
    pub types: Vec<FuncType>,

    /// Addresses of functions.
    pub func_addrs: Vec<Addr>,

    /// Addresses of tables.
    pub table_addrs: Vec<Addr>,

    /// Addresses of linear memories.
    pub mem_addrs: Vec<Addr>,

    /// Addresses of global variables.
    pub global_addrs: Vec<Addr>,

    /// Module's exports.
    pub exports: Vec<ExportInstance>,
}

/// Runtime representation of a Wasm export.
pub struct ExportInstance {
    /// Name of export.
    pub name: String,

    /// Value of export.
    pub value: ExternVal,
}

/// Optional address to a function.
pub type FuncElem = Option<Addr>;

/// Wasm address (basically an index).
pub type Addr = usize;

/// Wasm module export value.
pub enum ExternVal {
    Func(Addr),
    Table(Addr),
    Mem(Addr),
    Global(Addr),
}

/// Runtime function call frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct Frame {
    /// Number of values the function returns.
    pub arity: usize,

    /// Where the function's locals begin in the `locals` stack.
    pub locals_start: usize,

    /// Where the function's values begin in the `values` stack.
    pub values_start: usize,
}

/// Runtime structured control construct (block/loop/if).
#[derive(Debug, Default, Clone, Copy)]
pub struct Block {
    /// Number of values this block/loop/if produces.
    pub arity: usize,

    /// `values` stack height once we entered this block/loop/if.
    pub values_start: usize,
}

/// Host function (leaving this for later)
pub struct HostFunc;

pub struct Executor {
    pub values: Vec<Val>,
    pub locals: Vec<Val>,
    pub current_frame: Frame,
    pub current_block: Block,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            locals: Vec::new(),
            current_frame: Frame::default(),
            current_block: Block::default(),
        }
    }
    
    pub fn execute_instructions(
        &mut self, 
        instrs: &[Instr], 
        level: usize, 
        store: &mut Store, 
        module: &Rc<ModuleInstance>
    ) -> Result<Option<usize>, ExecuteError> {
        for instr in instrs {
            match instr {
                // Control Instructions
                Instr::Unreachable => return Err(ExecuteError::Trapped),
                Instr::Nop => {},

                Instr::Block(block_type, body) => {
                    let arity = Self::block_arity(*block_type);
                    let prev = self.enter_block(arity);

                    // execute instructions in block with nested level
                    let branch = self.execute_instructions(body, level + 1, store, module)?;
                
                    // check if we're unwinding to a branch that's further up
                    let unwinding = branch.is_some_and(|target| target <= level);
                    self.exit_block(prev, unwinding);

                    /*
                        if we exited the block through a br/br_if/return 
                        and we're unwinding, we should exit early.
                    */
                    if unwinding {
                        return Ok(branch);
                    }
                },

                Instr::Loop(_block_type, body) => {
                    // arity for loops is always 0
                    let prev = self.enter_block(0);
                    let current_level = level + 1;

                    loop {
                        let return_level = self.execute_instructions(body, level + 1, store, module)?;

                        if return_level == Some(current_level) {
                            continue;
                        }

                        let unwinding = return_level.is_some_and(|target| target <= level);
                        self.exit_block(prev, unwinding);

                        // looping
                        if unwinding {
                            return Ok(return_level);
                        }

                        break;
                    }
                },

                Instr::If(block_type, then_block, else_block) => {
                    let condition = self.pop_value()?.as_i32();
                    let prev = self.enter_block(Self::block_arity(*block_type));

                    // execute block based on condition
                    let return_level = if condition != 0 {
                        self.execute_instructions(then_block, level + 1, store, module)?
                    } else {
                        self.execute_instructions(else_block, level + 1, store, module)?
                    };

                    let unwinding = return_level.is_some_and(|target| target <= level);
                    self.exit_block(prev, unwinding);

                    if unwinding {
                        return Ok(return_level);
                    }
                }

                Instr::Br(label) => {
                    return Ok(Some(level - *label as usize));
                }

                Instr::BrIf(label) => {
                    let condition = self.pop_value()?.as_i32();

                    if condition != 0 {
                        return Ok(Some(level - *label as usize));
                    }
                },

                Instr::BrTable(frame_indices, fallback) => {
                    let label_idx = self.pop_value()?.as_i32() as usize;

                    if label_idx <= frame_indices.len() {
                        let label = frame_indices
                            .get(label_idx)
                            .expect("label index should exist.");

                        return Ok(Some(level - *label as usize));
                    }

                    return Ok(Some(level - *fallback as usize));
                }

                Instr::Return => return Ok(Some(0)),

                Instr::Call(func_idx) => {
                    let addr = module.func_addrs
                        .get(*func_idx as usize)
                        .ok_or(ExecuteError::InvalidFuncIndex)?;

                    self.call_function(*addr, store)?;
                },
                
                Instr::CallIndirect(type_idx) => {
                    let expect_type = module.types
                        .get(*type_idx as usize)
                        .ok_or(ExecuteError::InvalidTypeIndex)?;
                    
                    let i = self.pop_value()?.as_i32() as usize;

                    let table_idx = *module.table_addrs
                        .get(0)
                        .expect("There should be one table defined.");

                    let table = store.tables
                        .get(table_idx)
                        .unwrap();

                    // get function element index
                    let func_addr = table.elem
                        .get(i)
                        .ok_or(ExecuteError::Trapped)? // invalid function address index 
                        .ok_or(ExecuteError::Trapped)?; // invalid function address

                    let func = store.funcs
                        .get(func_addr)
                        .expect("Table element should point to a valid function.");
                    
                    let actual_type = match func {
                        FuncInstance::Wasm { func_type, module: _, code: _ } => func_type,
                        FuncInstance::Host { func_type, code: _ } => func_type
                    };

                    // types should match
                    if *expect_type != **actual_type {
                        return Err(ExecuteError::Trapped);
                    }

                    self.call_function(func_addr, store)?;
                },

                // Parametric Instructions
                Instr::Drop => { self.pop_value()?; },

                Instr::Select => {
                    let c = self.pop_value()?.as_i32();

                    let val_2 = self.pop_value()?;
                    let val_1 = self.pop_value()?;

                    if c != 0 {
                        self.push_value(val_1);
                    } else {
                        self.push_value(val_2);
                    }
                },

                // Variable Instructions
                Instr::LocalGet(local_idx) => {
                    let local_idx: usize = *local_idx as usize;
                    
                    let local = self.locals
                        .get(self.current_frame.locals_start + local_idx)
                        .expect("Local should exist.");
                    
                    self.push_value(*local);
                },

                Instr::LocalSet(local_idx) => {
                    let local_idx: usize = *local_idx as usize;

                    let val = self.pop_value()?;

                    let local = self.locals
                        .get_mut(self.current_frame.locals_start + local_idx)
                        .expect("Local should exist.");

                    *local = val;
                },

                Instr::LocalTee(local_idx) => {
                    let local_idx: usize = *local_idx as usize;

                    let val = self.pop_value()?;

                    self.push_value(val);

                    let local = self.locals
                        .get_mut(self.current_frame.locals_start + local_idx)
                        .expect("Local should exist.");

                    *local = val;
                },

                Instr::GlobalGet(global_idx) => {
                    let global_idx: usize = *global_idx as usize;

                    let global_addr = *module.global_addrs
                        .get(global_idx)
                        .ok_or(ExecuteError::InvalidGlobalIndex)?;

                    let global = store.globals
                        .get(global_addr)
                        .expect("Global should exist.");

                    self.push_value(global.value);
                },

                Instr::GlobalSet(global_idx) => {
                    let global_idx: usize = *global_idx as usize;

                    let val = self.pop_value()?;

                    let global_addr = *module.global_addrs
                        .get(global_idx)
                        .ok_or(ExecuteError::InvalidGlobalIndex)?;
                    
                    let global = store.globals
                        .get_mut(global_addr)
                        .expect("Global should exist.");

                    global.value = val;
                }

                // Memory Instructions
                Instr::I32Load(arg) => {
                    let bytes = self.load_bytes(32, arg, module, store)?;
                    self.push_value(Val::I32(i32::from_le_bytes(bytes[..4].try_into().unwrap())));
                },

                Instr::I64Load(arg) => {
                    let bytes = self.load_bytes(64, arg, module, store)?;
                    self.push_value(Val::I64(i64::from_le_bytes(bytes)));
                },

                Instr::F32Load(arg) => {
                    let bytes = self.load_bytes(32, arg, module, store)?;
                    self.push_value(Val::F32(f32::from_le_bytes(bytes[..4].try_into().unwrap())));
                },

                Instr::F64Load(arg) => {
                    let bytes = self.load_bytes(64, arg, module, store)?;
                    self.push_value(Val::F64(f64::from_le_bytes(bytes)));
                },

                Instr::I32Load8S(arg) => {
                    let bytes = self.load_bytes(8, arg, module, store)?;
                    self.push_value(Val::I32(bytes[0] as i8 as i32));
                }

                Instr::I32Load8U(arg) => {
                    let bytes = self.load_bytes(8, arg, module, store)?;
                    self.push_value(Val::I32(bytes[0] as u8 as i32));
                },

                Instr::I32Load16S(arg) => {
                    let bytes = self.load_bytes(16, arg, module, store)?;
                    self.push_value(Val::I32(i16::from_le_bytes(bytes[..2].try_into().unwrap()) as i32));
                }

                Instr::I32Load16U(arg) => {
                    let bytes = self.load_bytes(16, arg, module, store)?;
                    self.push_value(Val::I32(u16::from_le_bytes(bytes[..2].try_into().unwrap()) as i32));
                },

                Instr::I64Load8S(arg) => {
                    let bytes = self.load_bytes(8, arg, module, store)?;
                    self.push_value(Val::I64(bytes[0] as i8 as i64));
                }

                Instr::I64Load8U(arg) => {
                    let bytes = self.load_bytes(8, arg, module, store)?;
                    self.push_value(Val::I64(bytes[0] as u8 as i64));
                },

                Instr::I64Load16S(arg) => {
                    let bytes = self.load_bytes(16, arg, module, store)?;
                    self.push_value(Val::I64(i16::from_le_bytes(bytes[..2].try_into().unwrap()) as i64));
                }

                Instr::I64Load16U(arg) => {
                    let bytes = self.load_bytes(16, arg, module, store)?;
                    self.push_value(Val::I64(u16::from_le_bytes(bytes[..2].try_into().unwrap()) as i64));
                },

                Instr::I64Load32S(arg) => {
                    let bytes = self.load_bytes(32, arg, module, store)?;
                    self.push_value(Val::I64(i32::from_le_bytes(bytes[..4].try_into().unwrap()) as i64));
                }

                Instr::I64Load32U(arg) => {
                    let bytes = self.load_bytes(32, arg, module, store)?;
                    self.push_value(Val::I64(u32::from_le_bytes(bytes[..4].try_into().unwrap()) as i64));
                },

                Instr::I32Store(arg) => {
                    let c = self.pop_value()?.as_i32();
                    self.store_bytes(32, arg, c as i64, module, store)?;
                },

                Instr::I32Store8(arg) => {
                    let c = self.pop_value()?.as_i32();
                    self.store_bytes(8, arg, c as i64, module, store)?;
                },

                Instr::I32Store16(arg) => {
                    let c = self.pop_value()?.as_i32();
                    self.store_bytes(16, arg, c as i64, module, store)?;
                },

                Instr::I64Store(arg) => {
                    let c = self.pop_value()?.as_i64();
                    self.store_bytes(64, arg, c, module, store)?;
                },

                Instr::I64Store8(arg) => {
                    let c = self.pop_value()?.as_i64();
                    self.store_bytes(8, arg, c, module, store)?;
                },

                Instr::I64Store16(arg) => {
                    let c = self.pop_value()?.as_i64();
                    self.store_bytes(16, arg, c, module, store)?;
                },

                Instr::I64Store32(arg) => {
                    let c = self.pop_value()?.as_i64();
                    self.store_bytes(32, arg, c, module, store)?;
                },

                Instr::F32Store(arg) => {
                    let c = self.pop_value()?.as_f32();
                    self.store_bytes(32, arg, c.to_bits() as i64, module, store)?;
                },

                Instr::F64Store(arg) => {
                    let c = self.pop_value()?.as_f64();
                    self.store_bytes(64, arg, c.to_bits() as i64, module, store)?;
                },

                Instr::MemorySize => {
                    let a = *module.mem_addrs
                        .get(0)
                        .ok_or(ExecuteError::InvalidMemAddressIndex)?;

                    let mem = store.mems
                        .get(a)
                        .expect("Memory instance should exist.");

                    let size = mem.data.len() / PAGE_SIZE;
                    self.push_value(Val::I32(size as i32));
                },

                Instr::MemoryGrow => {
                    let a = *module.mem_addrs
                        .get(0)
                        .ok_or(ExecuteError::InvalidMemAddressIndex)?;

                    let mem = store.mems
                        .get_mut(a)
                        .expect("Memory instance should exist.");

                    let mem_max_size = mem.max.unwrap_or(u32::MAX) as usize;

                    let old_size = mem.data.len() / PAGE_SIZE;

                    let n = self.pop_value()?.as_i32();
                    let new_size = old_size.checked_add(n as usize).ok_or(ExecuteError::Trapped)?;

                    if new_size <= mem_max_size {
                        mem.data.resize(new_size, 0);
                        self.push_value(Val::I32(old_size as i32));
                    } else {
                        self.push_value(Val::I32(-1));
                    }
                },

                // Numeric Instructions
                Instr::I32Const(v) => self.push_value(Val::I32(*v)),
                Instr::I64Const(v) => self.push_value(Val::I64(*v)),
                Instr::F32Const(v) => self.push_value(Val::F32(*v)),
                Instr::F64Const(v) => self.push_value(Val::F64(*v)),

                Instr::I32Clz => self.unop_i32(|v| v.leading_zeros() as i32)?,
                Instr::I32Ctz => self.unop_i32(|v| v.trailing_zeros() as i32)?,
                Instr::I32Popcnt => self.unop_i32(|v| v.count_ones() as i32)?,

                Instr::I32Add => self.binop_i32(|a, b| a.wrapping_add(b))?,
                Instr::I32Sub => self.binop_i32(|a, b| a.wrapping_sub(b))?,
                Instr::I32Mul => self.binop_i32(|a, b| a.wrapping_mul(b))?,

                Instr::I32DivS => self.binop_i32_trap(|a, b| {
                    if b == 0 { return Err(ExecuteError::Trapped); }
                    if a == i32::MIN && b == -1 { return Err(ExecuteError::Trapped); }
                    Ok(a.wrapping_div(b))
                })?,
                Instr::I32DivU => self.binop_i32_trap(|a, b| {
                    let (a, b) = (a as u32, b as u32);
                    if b == 0 { return Err(ExecuteError::Trapped); }
                    Ok((a / b) as i32)
                })?,
                Instr::I32RemS => self.binop_i32_trap(|a, b| {
                    if b == 0 { return Err(ExecuteError::Trapped); }
                    if a == i32::MIN && b == -1 { return Ok(0); }
                    Ok(a.wrapping_rem(b))
                })?,
                Instr::I32RemU => self.binop_i32_trap(|a, b| {
                    let (a, b) = (a as u32, b as u32);
                    if b == 0 { return Err(ExecuteError::Trapped); }
                    Ok((a % b) as i32)
                })?,

                Instr::I32And => self.binop_i32(|a, b| a & b)?,
                Instr::I32Or => self.binop_i32(|a, b| a | b)?,
                Instr::I32Xor => self.binop_i32(|a, b| a ^ b)?,
                Instr::I32Shl => self.binop_i32(|a, b| a.wrapping_shl((b as u32) & 31))?,
                Instr::I32ShrS => self.binop_i32(|a, b| a.wrapping_shr((b as u32) & 31))?,
                Instr::I32ShrU => self.binop_i32(|a, b| ((a as u32).wrapping_shr((b as u32) & 31)) as i32)?,
                Instr::I32Rotl => self.binop_i32(|a, b| a.rotate_left((b as u32) & 31))?,
                Instr::I32Rotr => self.binop_i32(|a, b| a.rotate_right((b as u32) & 31))?,

                Instr::I32Eqz => self.testop_i32(|c| c == 0)?,
                Instr::I32Eq => self.relop_i32(|a, b| a == b)?,
                Instr::I32Ne => self.relop_i32(|a, b| a != b)?,
                Instr::I32LtS => self.relop_i32(|a, b| a < b)?,
                Instr::I32LtU => self.relop_i32(|a, b| (a as u32) < (b as u32))?,
                Instr::I32GtS => self.relop_i32(|a, b| a > b)?,
                Instr::I32GtU => self.relop_i32(|a, b| (a as u32) > (b as u32))?,
                Instr::I32LeS => self.relop_i32(|a, b| a <= b)?,
                Instr::I32LeU => self.relop_i32(|a, b| (a as u32) <= (b as u32))?,
                Instr::I32GeS => self.relop_i32(|a, b| a >= b)?,
                Instr::I32GeU => self.relop_i32(|a, b| (a as u32) >= (b as u32))?,

                Instr::I64Clz => self.unop_i64(|v| v.leading_zeros() as i64)?,
                Instr::I64Ctz => self.unop_i64(|v| v.trailing_zeros() as i64)?,
                Instr::I64Popcnt => self.unop_i64(|v| v.count_ones() as i64)?,

                Instr::I64Add => self.binop_i64(|a, b| a.wrapping_add(b))?,
                Instr::I64Sub => self.binop_i64(|a, b| a.wrapping_sub(b))?,
                Instr::I64Mul => self.binop_i64(|a, b| a.wrapping_mul(b))?,

                Instr::I64DivS => self.binop_i64_trap(|a, b| {
                    if b == 0 { return Err(ExecuteError::Trapped); }
                    if a == i64::MIN && b == -1 { return Err(ExecuteError::Trapped); }
                    Ok(a.wrapping_div(b))
                })?,
                Instr::I64DivU => self.binop_i64_trap(|a, b| {
                    let (a, b) = (a as u64, b as u64);
                    if b == 0 { return Err(ExecuteError::Trapped); }
                    Ok((a / b) as i64)
                })?,
                Instr::I64RemS => self.binop_i64_trap(|a, b| {
                    if b == 0 { return Err(ExecuteError::Trapped); }
                    if a == i64::MIN && b == -1 { return Ok(0); }
                    Ok(a.wrapping_rem(b))
                })?,
                Instr::I64RemU => self.binop_i64_trap(|a, b| {
                    let (a, b) = (a as u64, b as u64);
                    if b == 0 { return Err(ExecuteError::Trapped); }
                    Ok((a % b) as i64)
                })?,

                Instr::I64And => self.binop_i64(|a, b| a & b)?,
                Instr::I64Or => self.binop_i64(|a, b| a | b)?,
                Instr::I64Xor => self.binop_i64(|a, b| a ^ b)?,
                Instr::I64Shl => self.binop_i64(|a, b| a.wrapping_shl((b as u32) & 63))?,
                Instr::I64ShrS => self.binop_i64(|a, b| a.wrapping_shr((b as u32) & 63))?,
                Instr::I64ShrU => self.binop_i64(|a, b| ((a as u64).wrapping_shr((b as u32) & 63)) as i64)?,
                Instr::I64Rotl => self.binop_i64(|a, b| a.rotate_left((b as u32) & 63))?,
                Instr::I64Rotr => self.binop_i64(|a, b| a.rotate_right((b as u32) & 63))?,

                Instr::I64Eqz => self.testop_i64(|c| c == 0)?,
                Instr::I64Eq => self.relop_i64(|a, b| a == b)?,
                Instr::I64Ne => self.relop_i64(|a, b| a != b)?,
                Instr::I64LtS => self.relop_i64(|a, b| a < b)?,
                Instr::I64LtU => self.relop_i64(|a, b| (a as u64) < (b as u64))?,
                Instr::I64GtS => self.relop_i64(|a, b| a > b)?,
                Instr::I64GtU => self.relop_i64(|a, b| (a as u64) > (b as u64))?,
                Instr::I64LeS => self.relop_i64(|a, b| a <= b)?,
                Instr::I64LeU => self.relop_i64(|a, b| (a as u64) <= (b as u64))?,
                Instr::I64GeS => self.relop_i64(|a, b| a >= b)?,
                Instr::I64GeU => self.relop_i64(|a, b| (a as u64) >= (b as u64))?,

                Instr::F32Abs => self.unop_f32(|v| v.abs())?,
                Instr::F32Neg => self.unop_f32(|v| -v)?,
                Instr::F32Sqrt => self.unop_f32(|v| v.sqrt())?,
                Instr::F32Ceil => self.unop_f32(|v| v.ceil())?,
                Instr::F32Floor => self.unop_f32(|v| v.floor())?,
                Instr::F32Trunc => self.unop_f32(|v| v.trunc())?,
                Instr::F32Nearest => self.unop_f32(|v| v.round_ties_even())?,
                Instr::F32Add => self.binop_f32(|a, b| a + b)?,
                Instr::F32Sub => self.binop_f32(|a, b| a - b)?,
                Instr::F32Mul => self.binop_f32(|a, b| a * b)?,
                Instr::F32Div => self.binop_f32(|a, b| a / b)?,
                Instr::F32Min => self.binop_f32(|a, b| {
                    if a.is_nan() || b.is_nan() { f32::NAN } else { a.min(b) }
                })?,
                Instr::F32Max => self.binop_f32(|a, b| {
                    if a.is_nan() || b.is_nan() { f32::NAN } else { a.max(b) }
                })?,
                Instr::F32Copysign => self.binop_f32(|a, b| a.copysign(b))?,
                Instr::F32Eq => self.relop_f32(|a, b| a == b)?,
                Instr::F32Ne => self.relop_f32(|a, b| a != b)?,
                Instr::F32Lt => self.relop_f32(|a, b| a < b)?,
                Instr::F32Gt => self.relop_f32(|a, b| a > b)?,
                Instr::F32Le => self.relop_f32(|a, b| a <= b)?,
                Instr::F32Ge => self.relop_f32(|a, b| a >= b)?,

                Instr::F64Abs => self.unop_f64(|v| v.abs())?,
                Instr::F64Neg => self.unop_f64(|v| -v)?,
                Instr::F64Sqrt => self.unop_f64(|v| v.sqrt())?,
                Instr::F64Ceil => self.unop_f64(|v| v.ceil())?,
                Instr::F64Floor => self.unop_f64(|v| v.floor())?,
                Instr::F64Trunc => self.unop_f64(|v| v.trunc())?,
                Instr::F64Nearest => self.unop_f64(|v| v.round_ties_even())?,
                Instr::F64Add => self.binop_f64(|a, b| a + b)?,
                Instr::F64Sub => self.binop_f64(|a, b| a - b)?,
                Instr::F64Mul => self.binop_f64(|a, b| a * b)?,
                Instr::F64Div => self.binop_f64(|a, b| a / b)?,
                Instr::F64Min => self.binop_f64(|a, b| {
                    if a.is_nan() || b.is_nan() { f64::NAN } else { a.min(b) }
                })?,
                Instr::F64Max => self.binop_f64(|a, b| {
                    if a.is_nan() || b.is_nan() { f64::NAN } else { a.max(b) }
                })?,
                Instr::F64Copysign => self.binop_f64(|a, b| a.copysign(b))?,
                Instr::F64Eq => self.relop_f64(|a, b| a == b)?,
                Instr::F64Ne => self.relop_f64(|a, b| a != b)?,
                Instr::F64Lt => self.relop_f64(|a, b| a < b)?,
                Instr::F64Gt => self.relop_f64(|a, b| a > b)?,
                Instr::F64Le => self.relop_f64(|a, b| a <= b)?,
                Instr::F64Ge => self.relop_f64(|a, b| a >= b)?,

                Instr::I32WrapI64 => self.cvtop_from_i64(|v| Val::I32(v as i32))?,

                Instr::I64ExtendI32U => self.cvtop_from_i32(|v| Val::I64(v as u32 as i64))?,
                Instr::I64ExtendI32S => self.cvtop_from_i32(|v| Val::I64(v as i64))?,

                Instr::I32TruncF32U => self.cvtop_from_f32_trap(|v| {
                    if v.is_nan() || !(v > -1.0 && v < 4294967296.0) {
                        return Err(ExecuteError::Trapped);
                    }

                    Ok(Val::I32(v.trunc() as u32 as i32))
                })?,
                Instr::I32TruncF32S => self.cvtop_from_f32_trap(|v| {
                    if v.is_nan() || !(v >= -2147483648.0 && v < 2147483648.0) {
                        return Err(ExecuteError::Trapped);
                    }

                    Ok(Val::I32(v.trunc() as i32))
                })?,
                Instr::I64TruncF32U => self.cvtop_from_f32_trap(|v| {
                    if v.is_nan() || !(v > -1.0 && v < 18446744073709551616.0) {
                        return Err(ExecuteError::Trapped);
                    }

                    Ok(Val::I64(v.trunc() as u64 as i64))
                })?,
                Instr::I64TruncF32S => self.cvtop_from_f32_trap(|v| {
                    if v.is_nan() || !(v >= -9223372036854775808.0 && v < 9223372036854775808.0) {
                        return Err(ExecuteError::Trapped);
                    }

                    Ok(Val::I64(v.trunc() as i64))
                })?,
                Instr::I32TruncF64U => self.cvtop_from_f64_trap(|v| {
                    if v.is_nan() || !(v > -1.0 && v < 4294967296.0) {
                        return Err(ExecuteError::Trapped);
                    }

                    Ok(Val::I32(v.trunc() as u32 as i32))
                })?,
                Instr::I32TruncF64S => self.cvtop_from_f64_trap(|v| {
                    if v.is_nan() || !(v >= -2147483648.0 && v < 2147483648.0) {
                        return Err(ExecuteError::Trapped);
                    }

                    Ok(Val::I32(v.trunc() as i32))
                })?,
                Instr::I64TruncF64U => self.cvtop_from_f64_trap(|v| {
                    if v.is_nan() || !(v > -1.0 && v < 18446744073709551616.0) {
                        return Err(ExecuteError::Trapped);
                    }

                    Ok(Val::I64(v.trunc() as u64 as i64))
                })?,
                Instr::I64TruncF64S => self.cvtop_from_f64_trap(|v| {
                    if v.is_nan() || !(v >= -9223372036854775808.0 && v < 9223372036854775808.0) {
                        return Err(ExecuteError::Trapped);
                    }

                    Ok(Val::I64(v.trunc() as i64))
                })?,

                Instr::F32ConvertI32U => self.cvtop_from_i32(|v| Val::F32(v as u32 as f32))?,
                Instr::F32ConvertI32S => self.cvtop_from_i32(|v| Val::F32(v as f32))?,
                Instr::F64ConvertI32U => self.cvtop_from_i32(|v| Val::F64(v as u32 as f64))?,
                Instr::F64ConvertI32S => self.cvtop_from_i32(|v| Val::F64(v as f64))?,
                Instr::F32ConvertI64U => self.cvtop_from_i64(|v| Val::F32(v as u64 as f32))?,
                Instr::F32ConvertI64S => self.cvtop_from_i64(|v| Val::F32(v as f32))?,
                Instr::F64ConvertI64U => self.cvtop_from_i64(|v| Val::F64(v as u64 as f64))?,
                Instr::F64ConvertI64S => self.cvtop_from_i64(|v| Val::F64(v as f64))?,

                Instr::F32DemoteF64 => self.cvtop_from_f64(|v| Val::F32(v as f32))?,
                Instr::F64PromoteF32 => self.cvtop_from_f32(|v| Val::F64(v as f64))?,

                Instr::I32ReinterpretF32 => self.cvtop_from_f32(|v| Val::I32(v.to_bits() as i32))?,
                Instr::I64ReinterpretF64 => self.cvtop_from_f64(|v| Val::I64(v.to_bits() as i64))?,
                Instr::F32ReinterpretI32 => self.cvtop_from_i32(|v| Val::F32(f32::from_bits(v as u32)))?,
                Instr::F64ReinterpretI64 => self.cvtop_from_i64(|v| Val::F64(f64::from_bits(v as u64)))?,
            }
        }

        Ok(None)
    }
    
    /// Pushes a value onto the value/operand stack.
    pub fn push_value(&mut self, v: Val) {
        self.values.push(v);
    }

    /// Pops a values from the operand stack.
    pub fn pop_value(&mut self) -> Result<Val, ExecuteError> {
        self.values
            .pop()
            .ok_or(ExecuteError::UnexpectedStackUnderflow)
    }

    /// Enters a new function control frame.
    pub fn enter_frame(&mut self, func_type: FuncType) -> Result<Frame, ExecuteError> {
        let locals_start = self.locals.len();

        // add function params to locals
        for _ in 0..func_type.params.len() {
            let v = self.pop_value()?;
            self.locals.push(v);
        }
        self.locals[locals_start..].reverse();

        // set the new frame as the current frame
        let values_start = self.values.len();
        let prev = self.current_frame;

        self.current_frame = Frame {
            arity: func_type.results.len(),
            locals_start,
            values_start
        }; 

        Ok(prev)
    }

    /// Exits the current frame and restores the previous one.
    /// Keeps `arity` results on the operand stack.
    pub fn exit_frame(&mut self, prev: Frame) {
        let curr_frame = self.current_frame;

        // remove locals and function operands
        self.locals.truncate(curr_frame.locals_start);
        self.values.drain(curr_frame.values_start..self.values.len()-curr_frame.arity);

        self.current_frame = prev;
    }

    /// Enters a new structured control construct (block/loop/if) that returns `arity` values. Returns the current block.
    fn enter_block(&mut self, arity: usize) -> Block {
        let prev = self.current_block;

        self.current_block = Block { arity, values_start: self.values.len() };

        prev
    }

    /// Exits the current block into the previous one.
    /// Keeps `arity` results on the stack if we exited
    /// the structured control construct normally.
    fn exit_block(&mut self, prev: Block, unwinding: bool) {
        let curr_block = self.current_block;

        // If we exited normally, we should keep `arity` results on the stack and remove all the other values
        if !unwinding {
            let values_end = self.values.len() - curr_block.arity;

            // remove all values from start to end from operand stack
            self.values.drain(curr_block.values_start..values_end);
        }

        self.current_block = prev;
    }

    /// Calls the function at the given index.
    fn call_function(&mut self, func_idx: usize, store: &mut Store) -> Result<(), ExecuteError> {
        let func = store.funcs
            .get(func_idx)
            .expect(&format!("Function at index {func_idx} should exist."));
        
        match func {
            FuncInstance::Host { func_type: _, code: _ } => {
                // TODO: implement host function calling
            },

            FuncInstance::Wasm { func_type, module, code } => {
                let module = Rc::clone(module);
                let code = Rc::clone(code);

                let prev_frame = self.enter_frame((**func_type).clone())?;

                for v in code.locals.iter().copied().map(Val::zero) {
                    self.locals.push(v);
                }

                self.execute_instructions(&code.body.instructions, 0, store, &module)?;
            
                self.exit_frame(prev_frame);
            }
        }
        
        Ok(())
    }

    /// Loads `N` bits from the only defined memory instance into a static 8-byte buffer.
    fn load_bytes(&mut self, n: usize, arg: &MemArg, module: &Rc<ModuleInstance>, store: &Store) -> Result<[u8; 8], ExecuteError> {
        let a = *module.mem_addrs
            .get(0)
            .ok_or(ExecuteError::InvalidMemAddressIndex)?;

        let mem = store.mems
            .get(a)
            .expect("Memory instance should exist.");

        let num_bytes = n / 8;
        let i = self.pop_value()?.as_i32();

        let ea = (i as u32).checked_add(arg.offset).ok_or(ExecuteError::Trapped)?;
        let end = (ea as usize).checked_add(num_bytes).ok_or(ExecuteError::Trapped)?;

        if end > mem.data.len() {
            return Err(ExecuteError::Trapped);
        }

        let ea = ea as usize;

        // copy bytes into buffer
        let mut bytes = [0u8; 8];
        bytes[..num_bytes].copy_from_slice(&mem.data[ea..ea + num_bytes]);

        Ok(bytes)
    }

    /// Stores `N` bits into the only defined memory instance from a 8-byte constant `c`.
    fn store_bytes(&mut self, n: usize, arg: &MemArg, c: i64, module: &Rc<ModuleInstance>, store: &mut Store) -> Result<(), ExecuteError> {
        let a = *module.mem_addrs
            .get(0)
            .ok_or(ExecuteError::InvalidMemAddressIndex)?;

        let mem = store.mems
            .get_mut(a)
            .expect("Memory instance should exist.");

        let num_bytes = n / 8;
        let i = self.pop_value()?.as_i32();

        let ea = (i as u32).checked_add(arg.offset).ok_or(ExecuteError::Trapped)?;
        let end = (ea as usize).checked_add(num_bytes).ok_or(ExecuteError::Trapped)?;

        if end > mem.data.len() {
            return Err(ExecuteError::Trapped);
        }
        
        let bytes = c.to_le_bytes();
        mem.data[ea as usize..end].copy_from_slice(&bytes[..num_bytes]);

        Ok(())
    }

    fn unop_i32<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where
        F: FnOnce(i32) -> i32
    {
        let v = self.pop_value()?.as_i32();
        self.push_value(Val::I32(f(v)));
        Ok(())
    }

    fn unop_i64<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where
        F: FnOnce(i64) -> i64
    {
        let v = self.pop_value()?.as_i64();
        self.push_value(Val::I64(f(v)));
        Ok(())
    }

    fn unop_f32<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where
        F: FnOnce(f32) -> f32
    {
        let v = self.pop_value()?.as_f32();
        self.push_value(Val::F32(f(v)));
        Ok(())
    }

    fn unop_f64<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where
        F: FnOnce(f64) -> f64
    {
        let v = self.pop_value()?.as_f64();
        self.push_value(Val::F64(f(v)));
        Ok(())
    }

    fn binop_i32<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where
        F: FnOnce(i32, i32) -> i32
    {
        let c2 = self.pop_value()?.as_i32();
        let c1 = self.pop_value()?.as_i32();
        self.push_value(Val::I32(f(c1, c2)));
        Ok(())
    }

    fn binop_i64<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where
        F: FnOnce(i64, i64) -> i64
    {
        let c2 = self.pop_value()?.as_i64();
        let c1 = self.pop_value()?.as_i64();
        self.push_value(Val::I64(f(c1, c2)));
        Ok(())
    }

    fn binop_f32<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where
        F: FnOnce(f32, f32) -> f32
    {
        let c2 = self.pop_value()?.as_f32();
        let c1 = self.pop_value()?.as_f32();
        self.push_value(Val::F32(f(c1, c2)));
        Ok(())
    }

    fn binop_f64<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where
        F: FnOnce(f64, f64) -> f64
    {
        let c2 = self.pop_value()?.as_f64();
        let c1 = self.pop_value()?.as_f64();
        self.push_value(Val::F64(f(c1, c2)));
        Ok(())
    }

    fn binop_i32_trap<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where
        F: FnOnce(i32, i32) -> Result<i32, ExecuteError>
    {
        let c2 = self.pop_value()?.as_i32();
        let c1 = self.pop_value()?.as_i32();
        self.push_value(Val::I32(f(c1, c2)?));
        Ok(())
    }

    fn binop_i64_trap<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where
        F: FnOnce(i64, i64) -> Result<i64, ExecuteError>
    {
        let c2 = self.pop_value()?.as_i64();
        let c1 = self.pop_value()?.as_i64();
        self.push_value(Val::I64(f(c1, c2)?));
        Ok(())
    }

    fn testop_i32<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where
        F: FnOnce(i32) -> bool
    {
        let c = self.pop_value()?.as_i32();
        self.push_value(Val::I32(if f(c) { 1 } else { 0 }));
        Ok(())
    }

    fn testop_i64<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where
        F: FnOnce(i64) -> bool
    {
        let c = self.pop_value()?.as_i64();
        self.push_value(Val::I32(if f(c) { 1 } else { 0 }));
        Ok(())
    }

    fn relop_i32<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where
        F: FnOnce(i32, i32) -> bool
    {
        let c2 = self.pop_value()?.as_i32();
        let c1 = self.pop_value()?.as_i32();
        self.push_value(Val::I32(if f(c1, c2) { 1 } else { 0 }));
        Ok(())
    }

    fn relop_i64<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where
        F: FnOnce(i64, i64) -> bool
    {
        let c2 = self.pop_value()?.as_i64();
        let c1 = self.pop_value()?.as_i64();
        self.push_value(Val::I32(if f(c1, c2) { 1 } else { 0 }));
        Ok(())
    }

    fn relop_f32<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where
        F: FnOnce(f32, f32) -> bool
    {
        let c2 = self.pop_value()?.as_f32();
        let c1 = self.pop_value()?.as_f32();
        self.push_value(Val::I32(if f(c1, c2) { 1 } else { 0 }));
        Ok(())
    }

    fn relop_f64<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where
        F: FnOnce(f64, f64) -> bool
    {
        let c2 = self.pop_value()?.as_f64();
        let c1 = self.pop_value()?.as_f64();
        self.push_value(Val::I32(if f(c1, c2) { 1 } else { 0 }));
        Ok(())
    }

    fn cvtop_from_i32<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where 
        F: FnOnce(i32) -> Val
    {
        let v = self.pop_value()?.as_i32();
        self.push_value(f(v));
        Ok(())
    }

    fn cvtop_from_i64<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where 
        F: FnOnce(i64) -> Val
    {
        let v = self.pop_value()?.as_i64();
        self.push_value(f(v));
        Ok(())
    }

    fn cvtop_from_f32<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where 
        F: FnOnce(f32) -> Val
    {
        let v = self.pop_value()?.as_f32();
        self.push_value(f(v));
        Ok(())
    }

    fn cvtop_from_f32_trap<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where 
        F: FnOnce(f32) -> Result<Val, ExecuteError>
    {
        let v = self.pop_value()?.as_f32();
        self.push_value(f(v)?);
        Ok(())
    }

    fn cvtop_from_f64<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where 
        F: FnOnce(f64) -> Val
    {
        let v = self.pop_value()?.as_f64();
        self.push_value(f(v));
        Ok(())
    }

    fn cvtop_from_f64_trap<F>(&mut self, f: F) -> Result<(), ExecuteError>
    where 
        F: FnOnce(f64) -> Result<Val, ExecuteError>
    {
        let v = self.pop_value()?.as_f64();
        self.push_value(f(v)?);
        Ok(())
    }
    
    /// Returns the arity of the given `BlockType`.
    fn block_arity(block_type: BlockType) -> usize {
        match block_type {
            BlockType::Empty => 0,
            BlockType::Val(_) => 1,
        }
    }
}