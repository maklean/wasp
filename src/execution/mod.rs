use std::rc::Rc;

use crate::{errors::{ExecutionError, RuntimeStack, TrapReason}, runtime::{FuncInstance, ModuleInstance, Store, Val}, structure::{Func, FuncType, LabelKind, MemArg}};

/// Wasm module instance executor.
#[derive(Default)]
pub(crate) struct Executor {
    /// Operand stack.
    pub values: Vec<Val>,

    /// Locals of all currently active functions.
    pub locals: Vec<Val>,

    /// Function call frame stack.
    pub frames: Vec<Frame>,

    /// Control construct stack.
    pub blocks: Vec<Block>,
}

impl Executor {
    /// Maximum function call depth.
    const MAX_CALL_DEPTH: usize = 1024;

    /// Initializes an `Executor` with the `args` pushed onto the operand stack.
    pub(crate) fn with_args(args: Vec<Val>) -> Self {
        Self {
            values: args,
            ..Default::default()
        }
    }
    
    /// Executes every call frame on the call frame stack until we've reached the target frame count.
    fn run(&mut self, store: &mut Store) -> Result<(), ExecutionError> {
        while !self.frames.is_empty() {
            let frame = self.frames.last().unwrap();

            if frame.pc >= frame.code.body.instructions.len() {
                self.pop_frame()?;
                continue;
            }

            let module = Rc::clone(&frame.module);
            let instr = frame.code.body.instructions[frame.pc].clone();
            self.frames.last_mut().unwrap().pc += 1;

            instr.execute(self, store, module)?;
        }

        Ok(())
    }

    /// Executes the function at the given address in the store.
    pub(crate) fn execute_function(&mut self, func_addr: usize, store: &mut Store, main: bool) -> Result<(), ExecutionError> {
        let func = &store.funcs[func_addr];

        match func {
            FuncInstance::Host { func_type, code } => {
                let code = Rc::clone(code);
                let params: Vec<Val> = self.values.drain(self.values.len() - func_type.params.len()..).collect();

                for v in (code.func)(store, params)? {
                    self.push_value(v);
                }
            },

            FuncInstance::Wasm { func_type, module, code } => {
                self.push_frame((**func_type).clone(), Rc::clone(code), Rc::clone(module))?;

                // this should only trigger for the outermost function. If there's multiple loops, the whole thing actually breaks
                if main { self.run(store)?; }
            }
        }

        Ok(())
    }

    /// Pushses the given value onto the operand stack.
    pub(crate) fn push_value(&mut self, v: Val) {
        self.values.push(v);
    }

    /// Pops and returns a value from the operand stack.
    pub(crate) fn pop_value(&mut self) -> Result<Val, ExecutionError> {
        self.values
            .pop()
            .ok_or(ExecutionError::UnexpectedStackUnderflow(RuntimeStack::Operand))
    }

    /// Retrieves the value at the top of the operand stack, but does not consume it.
    pub(crate) fn peek_value(&self) -> Result<Val, ExecutionError> {
        self.values
            .last()
            .copied()
            .ok_or(ExecutionError::UnexpectedStackUnderflow(RuntimeStack::Operand))
    }

    /// Pushes a new function call frame onto the call frame stack. 
    /// Pushes its parameters and locals onto the locals stack.
    pub(crate) fn push_frame(&mut self, func_type: FuncType, code: Rc<Func>, module: Rc<ModuleInstance>) -> Result<(), ExecutionError> {
        if self.frames.len() >= Self::MAX_CALL_DEPTH {
            return Err(ExecutionError::Trapped(TrapReason::CallStackExhausted));
        }

        let locals_start = self.locals.len();

        // add function params to locals
        for _ in 0..func_type.params.len() {
            let v = self.pop_value()?;
            self.locals.push(v);
        }

        self.locals[locals_start..].reverse();

        // add zero'ed locals to locals stack
        for v in code.locals.iter().copied().map(Val::zero) {
            self.locals.push(v);
        }

        let arity = func_type.results.len();
        let values_start = self.values.len();
        let blocks_start = self.blocks.len();

        self.frames.push(Frame {
            arity,
            locals_start,
            values_start,
            blocks_start,
            pc: 0,
            code,
            module,
        });

        self.push_block(arity, LabelKind::Block);

        Ok(())
    }

    /// Pops a function call frame from the call frame stack. Keeps
    /// its return values on the operand stack.
    pub(crate) fn pop_frame(&mut self) -> Result<Frame, ExecutionError> {
        let frame = self.frames.pop()
            .ok_or(ExecutionError::UnexpectedStackUnderflow(RuntimeStack::Frame))?;

        // remove function's locals, and blocks (temporaries are cleared once Instr::end is reached)
        self.locals.truncate(frame.locals_start);
        self.blocks.truncate(frame.blocks_start);

        Ok(frame)
    }

    /// Pushes a new control construct that returns `arity` values onto the control construct stack.
    pub(crate) fn push_block(&mut self, arity: usize, kind: LabelKind) {
        self.blocks.push(Block {
            arity,
            values_start: self.values.len(),
            kind,
        });
    }

    /// Pops a control construct from the control construct stack.
    pub(crate) fn pop_block(&mut self) -> Result<Block, ExecutionError> {
        self.blocks.pop()
            .ok_or(ExecutionError::UnexpectedStackUnderflow(RuntimeStack::Block))
    }

    /// Executes a unary i32 operator.
    pub(crate) fn unop_i32<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where
        F: FnOnce(i32) -> i32
    {
        let v = self.pop_value()?.as_i32();
        self.push_value(Val::I32(f(v)));
        Ok(())
    }

    /// Executes a unary i64 operator.
    pub(crate) fn unop_i64<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where
        F: FnOnce(i64) -> i64
    {
        let v = self.pop_value()?.as_i64();
        self.push_value(Val::I64(f(v)));
        Ok(())
    }

    /// Executes a unary f32 operator.
    pub(crate) fn unop_f32<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where
        F: FnOnce(f32) -> f32
    {
        let v = self.pop_value()?.as_f32();
        self.push_value(Val::F32(f(v)));
        Ok(())
    }

    /// Executes a unary f64 operator.
    pub(crate) fn unop_f64<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where
        F: FnOnce(f64) -> f64
    {
        let v = self.pop_value()?.as_f64();
        self.push_value(Val::F64(f(v)));
        Ok(())
    }

    /// Executes a binary i32 operator.
    pub(crate) fn binop_i32<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where
        F: FnOnce(i32, i32) -> i32
    {
        let c2 = self.pop_value()?.as_i32();
        let c1 = self.pop_value()?.as_i32();
        self.push_value(Val::I32(f(c1, c2)));
        Ok(())
    }

    /// Executes a binary i64 operator.
    pub(crate) fn binop_i64<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where
        F: FnOnce(i64, i64) -> i64
    {
        let c2 = self.pop_value()?.as_i64();
        let c1 = self.pop_value()?.as_i64();
        self.push_value(Val::I64(f(c1, c2)));
        Ok(())
    }

    /// Executes a binary f32 operator.
    pub(crate) fn binop_f32<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where
        F: FnOnce(f32, f32) -> f32
    {
        let c2 = self.pop_value()?.as_f32();
        let c1 = self.pop_value()?.as_f32();
        self.push_value(Val::F32(f(c1, c2)));
        Ok(())
    }

    /// Executes a binary f64 operator.
    pub(crate) fn binop_f64<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where
        F: FnOnce(f64, f64) -> f64
    {
        let c2 = self.pop_value()?.as_f64();
        let c1 = self.pop_value()?.as_f64();
        self.push_value(Val::F64(f(c1, c2)));
        Ok(())
    }

    /// Executes a binary i32 operator that may trap.
    pub(crate) fn binop_i32_trap<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where
        F: FnOnce(i32, i32) -> Result<i32, ExecutionError>
    {
        let c2 = self.pop_value()?.as_i32();
        let c1 = self.pop_value()?.as_i32();
        self.push_value(Val::I32(f(c1, c2)?));
        Ok(())
    }

    /// Executes a binary i64 operator that may trap.
    pub(crate) fn binop_i64_trap<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where
        F: FnOnce(i64, i64) -> Result<i64, ExecutionError>
    {
        let c2 = self.pop_value()?.as_i64();
        let c1 = self.pop_value()?.as_i64();
        self.push_value(Val::I64(f(c1, c2)?));
        Ok(())
    }

    /// Executes a test operator on an i32.
    pub(crate) fn testop_i32<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where
        F: FnOnce(i32) -> bool
    {
        let c = self.pop_value()?.as_i32();
        self.push_value(Val::I32(if f(c) { 1 } else { 0 }));
        Ok(())
    }

    /// Executes a test operator on an i64.
    pub(crate) fn testop_i64<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where
        F: FnOnce(i64) -> bool
    {
        let c = self.pop_value()?.as_i64();
        self.push_value(Val::I32(if f(c) { 1 } else { 0 }));
        Ok(())
    }

    /// Executes a relational operator on i32s.
    pub(crate) fn relop_i32<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where
        F: FnOnce(i32, i32) -> bool
    {
        let c2 = self.pop_value()?.as_i32();
        let c1 = self.pop_value()?.as_i32();
        self.push_value(Val::I32(if f(c1, c2) { 1 } else { 0 }));
        Ok(())
    }

    /// Executes a relational operator on i64s.
    pub(crate) fn relop_i64<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where
        F: FnOnce(i64, i64) -> bool
    {
        let c2 = self.pop_value()?.as_i64();
        let c1 = self.pop_value()?.as_i64();
        self.push_value(Val::I32(if f(c1, c2) { 1 } else { 0 }));
        Ok(())
    }

    /// Executes a relational operator on f32s.
    pub(crate) fn relop_f32<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where
        F: FnOnce(f32, f32) -> bool
    {
        let c2 = self.pop_value()?.as_f32();
        let c1 = self.pop_value()?.as_f32();
        self.push_value(Val::I32(if f(c1, c2) { 1 } else { 0 }));
        Ok(())
    }

    /// Executes a relational operator on f64s.
    pub(crate) fn relop_f64<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where
        F: FnOnce(f64, f64) -> bool
    {
        let c2 = self.pop_value()?.as_f64();
        let c1 = self.pop_value()?.as_f64();
        self.push_value(Val::I32(if f(c1, c2) { 1 } else { 0 }));
        Ok(())
    }

    /// Executes a conversion operator from an i32.
    pub(crate) fn cvtop_from_i32<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where 
        F: FnOnce(i32) -> Val
    {
        let v = self.pop_value()?.as_i32();
        self.push_value(f(v));
        Ok(())
    }

    /// Executes a conversion operator from an i64.
    pub(crate) fn cvtop_from_i64<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where 
        F: FnOnce(i64) -> Val
    {
        let v = self.pop_value()?.as_i64();
        self.push_value(f(v));
        Ok(())
    }

    /// Executes a conversion operator from an f32.
    pub(crate) fn cvtop_from_f32<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where 
        F: FnOnce(f32) -> Val
    {
        let v = self.pop_value()?.as_f32();
        self.push_value(f(v));
        Ok(())
    }

    /// Executes a conversion operator from an f32 that may trap.
    pub(crate) fn cvtop_from_f32_trap<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where 
        F: FnOnce(f32) -> Result<Val, ExecutionError>
    {
        let v = self.pop_value()?.as_f32();
        self.push_value(f(v)?);
        Ok(())
    }

    /// Executes a conversion operator from an f64.
    pub(crate) fn cvtop_from_f64<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where 
        F: FnOnce(f64) -> Val
    {
        let v = self.pop_value()?.as_f64();
        self.push_value(f(v));
        Ok(())
    }

    /// Executes a conversion operator from an f64 that may trap.
    pub(crate) fn cvtop_from_f64_trap<F>(&mut self, f: F) -> Result<(), ExecutionError>
    where 
        F: FnOnce(f64) -> Result<Val, ExecutionError>
    {
        let v = self.pop_value()?.as_f64();
        self.push_value(f(v)?);
        Ok(())
    }

    /// Loads `N` bits from the only defined memory instance into a static 8-byte buffer.
    pub(crate) fn mem_load_bytes(&mut self, n: usize, arg: &MemArg, module: Rc<ModuleInstance>, store: &Store) -> Result<[u8; 8], ExecutionError> {
        // linear memory is guaranteed to exist due to validation
        let mem = &store.mems[module.mem_addrs[0]];

        let num_bytes = n / 8;
        let base_addr = self.pop_value()?.as_i32();

        let ea = (base_addr as u32).checked_add(arg.offset)
            .ok_or(ExecutionError::Trapped(TrapReason::OutOfBoundsMemoryAccess {
                addr: base_addr as usize, len: num_bytes, mem_size: mem.data.len()
            }))?;

        let end = (ea as usize).checked_add(num_bytes)
            .ok_or(ExecutionError::Trapped(TrapReason::OutOfBoundsMemoryAccess {
                addr: ea as usize, len: num_bytes, mem_size: mem.data.len()
            }))?;

        if end > mem.data.len() {
            return Err(ExecutionError::Trapped(TrapReason::OutOfBoundsMemoryAccess {
                addr: ea as usize, len: num_bytes, mem_size: mem.data.len()
            }));
        }

        let ea = ea as usize;
        
        // copy bytes into buffer
        let mut bytes = [0u8; 8];
        bytes[..num_bytes].copy_from_slice(&mem.data[ea..ea + num_bytes]);

        Ok(bytes)
    }

    /// Stores `N` bits into the only defined memory instance from a 8-byte constant `c`.
    pub(crate) fn mem_store_bytes(&mut self, n: usize, arg: &MemArg, c: i64, module: Rc<ModuleInstance>, store: &mut Store) -> Result<(), ExecutionError> {
        // linear memory is guaranteed to exist due to validation
        let mem = &mut store.mems[module.mem_addrs[0]];

        let num_bytes = n / 8;
        let base_addr = self.pop_value()?.as_i32();

        let ea = (base_addr as u32).checked_add(arg.offset)
            .ok_or(ExecutionError::Trapped(TrapReason::OutOfBoundsMemoryAccess {
                addr: base_addr as usize, len: num_bytes, mem_size: mem.data.len()
            }))?;

        let end = (ea as usize).checked_add(num_bytes)
            .ok_or(ExecutionError::Trapped(TrapReason::OutOfBoundsMemoryAccess {
                addr: ea as usize, len: num_bytes, mem_size: mem.data.len()
            }))?;

        if end > mem.data.len() {
            return Err(ExecutionError::Trapped(TrapReason::OutOfBoundsMemoryAccess {
                addr: ea as usize, len: num_bytes, mem_size: mem.data.len()
            }));
        }

        let ea = ea as usize;

        // store bytes from constant in memory
        let bytes = c.to_le_bytes();
        mem.data[ea..end].copy_from_slice(&bytes[..num_bytes]);

        Ok(())
    }
    
}

/// Function call frame.
#[derive(Default, Clone)]
pub struct Frame {
    /// Number of values the function returns.
    pub arity: usize,

    /// Where the function's locals begin in the executor's list of locals.
    pub locals_start: usize,

    /// Where the function's temporaries begin in the executor's operand stack.
    pub values_start: usize,

    /// Where the function begins in the control construct stack.
    pub blocks_start: usize,

    /// Program Counter (instruction index).
    pub pc: usize,

    /// Function call frame's code.
    pub code: Rc<Func>,

    /// The module the function is in.
    pub module: Rc<ModuleInstance>,
}

/// Control construct.
#[derive(Clone, Copy)]
pub struct Block {
    /// Type of control construct.
    pub kind: LabelKind,

    /// Number of values the construct returns.
    pub arity: usize,

    /// Where the construct's temporaries begin in the executor's operand stack.
    pub values_start: usize,
}