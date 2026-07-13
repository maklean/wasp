use std::rc::Rc;
use crate::{definitions::{Func, FuncType, Mutability, ValType}, errors::ExecuteError, instructions::{BlockType, Instr}};

/// Runtime representation of a Wasm value.
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
        func_type: FuncType,

        /// Originating module (used to resolve references to other definitions).
        module: Rc<ModuleInstance>,

        /// Function definition.
        code: Func,
    },

    /// A function provided by the host (import).
    Host {
        /// Signature of the function.
        func_type: FuncType,

        /// Host function.
        code: HostFunc,
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

    pub fn execute(
        &mut self, 
        instrs: &[Instr], 
        level: usize, 
        store: &mut Store, 
        module: &Rc<ModuleInstance>
    ) -> Result<Option<usize>, ExecuteError> {
        for instr in instrs {
            match instr {
                Instr::Unreachable => return Err(ExecuteError::Trapped),
                Instr::Nop => {},

                Instr::I32Const(v) => self.push_value(Val::I32(*v)),
                Instr::I64Const(v) => self.push_value(Val::I64(*v)),
                Instr::F32Const(v) => self.push_value(Val::F32(*v)),
                Instr::F64Const(v) => self.push_value(Val::F64(*v)),

                Instr::Drop => { self.pop_value()?; },

                Instr::Block(block_type, body) => {
                    let arity = Self::block_arity(*block_type);
                    let prev = self.enter_block(arity);

                    // execute instructions in block with nested level
                    let branch = self.execute(body, level + 1, store, module)?;
                
                    // check if we're unwinding to a branch that's further up
                    let unwinding = branch.is_some_and(|target| target <= level);
                    self.exit_block(prev, unwinding);

                    /*
                        if we exited the block through a br/br_if/return 
                        and we're unwinding, we should exit
                        early.
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
                        let return_level = self.execute(body, level + 1, store, module)?;

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
                        self.execute(then_block, level + 1, store, module)?
                    } else {
                        self.execute(else_block, level + 1, store, module)?
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
                }

                Instr::Return => return Ok(Some(0)),

                _ => todo!()
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
    
    fn block_arity(block_type: BlockType) -> usize {
        match block_type {
            BlockType::Empty => 0,
            BlockType::Val(_) => 1,
        }
    }
}