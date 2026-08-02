use std::rc::Rc;

use crate::{errors::{ExecutionError, TrapReason}, runtime::{FuncInstance, Store, Val}, structure::{FuncType, Instr}};

/// Wasm module instance executor.
#[derive(Default)]
pub(crate) struct Executor {
    /// Operand stack.
    pub values: Vec<Val>,

    /// Locals of all currently active functions.
    pub locals: Vec<Val>,

    /// Current function frame we're in.
    pub current_frame: Frame,

    /// Current control construct we're in.
    pub current_block: Block,

    /// Current function call depth.
    pub call_depth: usize,
}

impl Executor {
    /// Maximum function call depth.
    const MAX_CALL_DEPTH: usize = 1000;

    /// Initializes an `Executor` with the `args` pushed onto the operand stack.
    pub(crate) fn with_args(args: Vec<Val>) -> Self {
        Self {
            values: args,
            ..Default::default()
        }
    }

    /// Executes the function at the given index in the store.
    pub(crate) fn execute_function(&mut self, func_idx: usize, store: &mut Store) -> Result<(), ExecutionError> {
        self.call_depth += 1;

        if self.call_depth > Self::MAX_CALL_DEPTH {
            return Err(ExecutionError::Trapped(TrapReason::CallStackExhausted));
        }

        let call_result = (|| -> Result<(), ExecutionError> {
            // this function should only be called on validated modules, so this should exist.
            let func = &store.funcs[func_idx];
        
            match func {
                FuncInstance::Host { func_type, code } => {
                    let code = Rc::clone(code);
                    let params: Vec<Val> = self.values.drain(self.values.len() - func_type.params.len()..).collect();
                
                    // execute function and push results to operand stack
                    for v in (code.func)(store, params)? {
                        self.push_value(v);
                    }
                },

                FuncInstance::Wasm { func_type, module, code } => {
                    let module = Rc::clone(&module);
                    let code = Rc::clone(&code);

                    let prev_frame = self.enter_frame((**func_type).clone())?;

                    // add zero'ed locals to locals stack, then execute function body
                    for v in code.locals.iter().copied().map(Val::zero) {
                        self.locals.push(v);
                    }

                    Instr::execute_sequence(&code.body.instructions, self, 0, store, module)?;

                    self.exit_frame(prev_frame);
                }
            }

            Ok(())
        })();

        self.call_depth -= 1;

        call_result
    }

    /// Pushses the given value onto the operand stack.
    pub(crate) fn push_value(&mut self, v: Val) {
        self.values.push(v);
    }

    /// Pops and returns a value from the operand stack.
    pub(crate) fn pop_value(&mut self) -> Result<Val, ExecutionError> {
        self.values
            .pop()
            .ok_or(ExecutionError::UnexpectedStackUnderflow)
    }

    /// Retrieves the value at the top of the operand stack, but does not consume it.
    pub(crate) fn peek_value(&self) -> Result<Val, ExecutionError> {
        self.values
            .get(0)
            .copied()
            .ok_or(ExecutionError::UnexpectedStackUnderflow)
    }

    /// Enters a new function call frame. Pushes it parameters onto the locals stack.
    /// Returns the previous function call frame.
    pub(crate) fn enter_frame(&mut self, func_type: FuncType) -> Result<Frame, ExecutionError> {
        let locals_start = self.locals.len();

        // add function params to locals
        for _ in 0..func_type.params.len() {
            let v = self.pop_value()?;
            self.locals.push(v);
        }

        self.locals[locals_start..].reverse();

        // set the new current frame
        let prev_frame = self.current_frame;

        let new_frame = Frame {
            arity: func_type.results.len(),
            locals_start,
            values_start: self.values.len()
        };

        self.current_frame = new_frame;

        Ok(prev_frame)
    }

    /// Exits the function call frame into the given previous one. Keeps the current
    /// function frame's return values on the operand stack.
    pub(crate) fn exit_frame(&mut self, prev_frame: Frame) {
        // remove function locals and temporaries
        self.locals.truncate(self.current_frame.locals_start);
        self.values.drain(self.current_frame.values_start..self.values.len() - self.current_frame.arity); // keep function return values

        self.current_frame = prev_frame;
    }

    /// Enters a new control construct that returns `arity` values. Returns the previous control construct.
    pub(crate) fn enter_block(&mut self, arity: usize) -> Block {
        let prev_block = self.current_block;

        self.current_block = Block {
            arity,
            values_start: self.values.len()
        };

        prev_block
    }

    /// Exits the current control construct into the given one. If we're not unwinding
    /// past this construct, its temporaries are removed from the operand stack and its 
    /// return values are kept on the operand stack.
    pub(crate) fn exit_block(&mut self, prev_block: Block, unwinding: bool) {
        if !unwinding {
            self.values.drain(self.current_block.values_start..self.values.len() - self.current_block.arity); // keep the block's return values
        }

        self.current_block = prev_block;
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
}

/// Function call frame.
#[derive(Default, Clone, Copy)]
pub struct Frame {
    /// Number of values the function returns.
    pub arity: usize,

    /// Where the function's locals begin in the executor's list of locals.
    pub locals_start: usize,

    /// Where the function's temporaries begin in the executor's operand stack.
    pub values_start: usize
}

/// Control construct.
#[derive(Default, Clone, Copy)]
pub struct Block {
    /// Number of values the construct returns.
    pub arity: usize,

    /// Where the construct's temporaries begin in the executor's operand stack.
    pub values_start: usize
}