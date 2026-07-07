use crate::{definitions::{FuncType, GlobalType, ImportDesc, Limits, Mutability, TableType, ValType}, errors::ValidateError, instructions::MemArg, module::Module};

pub struct Validator<'a> {
    ctx: Context<'a>,

    /// Operand Stack - keeps track of types of operand values.
    opds: Vec<ValType>,

    /// Control Stack - keeps track of surrounding control constructs.
    ctrls: Vec<CtrlFrame>,
}

impl<'a> Validator<'a> {
    pub fn validate(module: &'a Module) -> Result<(), ValidateError> {
        let mut this = Self { ctx: Context::new(module), opds: Vec::new(), ctrls: Vec::new() };
        
        Ok(())
    }

    /// Validates a binary operator.
    pub fn binop(&mut self, t: ValType) -> Result<(), ValidateError> {
        self.pop_opd_expect(t)?;
        self.pop_opd_expect(t)?;
        self.push_opd(t);

        Ok(())
    }

    /// Validates a unary operator.
    pub fn unop(&mut self, t: ValType) -> Result<(), ValidateError> {
        self.pop_opd_expect(t)?;
        self.push_opd(t);

        Ok(())
    }

    /// Validates a test operator.
    pub fn testop(&mut self, t: ValType) -> Result<(), ValidateError> {
        self.pop_opd_expect(t)?;
        self.push_opd(ValType::I32);

        Ok(())
    }

    /// Validates a relational operator.
    pub fn relop(&mut self, t: ValType) -> Result<(), ValidateError> {
        self.pop_opd_expect(t)?;
        self.pop_opd_expect(t)?;
        self.push_opd(ValType::I32);

        Ok(())
    }

    /// Validates a conversion operator.
    pub fn cvtop(&mut self, from: ValType, to: ValType) -> Result<(), ValidateError> {
        self.pop_opd_expect(from)?;
        self.push_opd(to);

        Ok(())
    }

    /// Pushes a value type onto the operand stack.
    pub fn push_opd(&mut self, val_type: ValType) {
        self.opds.push(val_type);
    }

    /// Pops a value type from the operand stack.
    /// If we're popping past the end of the current
    /// control frame while being in dead/unreachable code,
    /// `ValType::Unknown` is returned as the value type.
    pub fn pop_opd(&mut self) -> Result<ValType, ValidateError> {
        let Some(frame) = self.ctrls.last() else {
            return Err(ValidateError::ExpectedAtLeastOneControlFrame);
        };

        if self.opds.len() == frame.height && frame.unreachable {
            // we're trying to pop past the end of the frame while being in dead code, return Unknown
            return Ok(ValType::Unknown);
        } else if self.opds.len() == frame.height  {
            // we're trying to pop past the end without being in dead code
            return Err(ValidateError::PoppingOutsideOfControlFrame);
        }

        Ok(self.opds.pop().unwrap())
    }

    /// Pops a value type off the operand stack as long as it matches the expected type.
    pub fn pop_opd_expect(&mut self, expect: ValType) -> Result<ValType, ValidateError> {
        let actual = self.pop_opd()?;

        // Unknown matches any type
        if actual == ValType::Unknown {
            return Ok(expect);
        } else if expect == ValType::Unknown {
            return Ok(actual);
        }

        // check for match
        if actual != expect {
            return Err(ValidateError::ExpectedOperandInOpdStack { expect, actual });
        }

        Ok(actual)
    }

    /// Pushes a collection of value types onto the operand stack.
    pub fn push_opds(&mut self, mut val_types: Vec<ValType>) {
        self.opds.append(&mut val_types);
    }

    /// Pops multiple operands from the operand stack as long as they match the given types.
    pub fn pop_opds(&mut self, val_types: Vec<ValType>) -> Result<(), ValidateError> {
        for expect in val_types.iter().rev() {
            self.pop_opd_expect(expect.clone())?;
        }

        Ok(())
    }

    /// Creates a new control frame with the given label and end types, and pushes it onto the control stack.
    pub fn push_ctrl(&mut self, label_types: Vec<ValType>, end_types: Vec<ValType>) {
        let frame = CtrlFrame { label_types, end_types, height: self.opds.len(), unreachable: false };
        self.ctrls.push(frame);
    }

    /// Pops a control frame from the control stack (will type-check for matching end types).
    pub fn pop_ctrl(&mut self) -> Result<Vec<ValType>, ValidateError> {
        if self.ctrls.last().is_none() {
            return Err(ValidateError::ExpectedAtLeastOneControlFrame);
        };

        let frame = self.ctrls.last().cloned().unwrap(); // yikes (TODO: maybe interior mutability prevents cloning)

        // since we're exiting a control frame, its end types should be sitting on the stack
        self.pop_opds(frame.end_types.clone())?;

        // should be back to the frame's height after popping all operands
        if self.opds.len() != frame.height {
            return Err(ValidateError::StackHeightMismatchAtEnd { expect: self.opds.len(), actual: frame.height });
        }

        self.ctrls.pop();

        Ok(frame.end_types)
    }

    /// Declares the current control frame as unreachable and truncates 
    /// the operand stack back to where said control frame begun.
    pub fn unreachable(&mut self) -> Result<(), ValidateError> {
        let Some(frame) = self.ctrls.last_mut() else {
            return Err(ValidateError::ExpectedAtLeastOneControlFrame);
        };

        // return back to the frame's height
        self.opds.truncate(frame.height);
        frame.unreachable = true;

        Ok(())
    }

    /// Returns the local at the given index.
    pub fn local_get(&self, index: u32) -> Result<ValType, ValidateError> {
        let index = index as usize;

        let local = *self.ctx.locals
            .get(index)
            .ok_or(ValidateError::LocalDoesntExist { index })?;

        Ok(local)
    }

    /// Sets (technically just type checks) the value of the local at the given index.
    pub fn local_set(&self, index: u32, val_type: ValType) -> Result<(), ValidateError> {
        let local = self.local_get(index)?;

        // types must be the same in order to set
        if local != val_type {
            return Err(ValidateError::LocalSetTypeMismatch { expect: local, actual: val_type });
        }

        // no need to mutate at self.ctx.locals[index] b/c we're just setting the same type
        Ok(())
    }

    /// Returns the value type of the global at the given index.
    pub fn global_get(&self, index: u32) -> Result<ValType, ValidateError> {
        let index = index as usize;

        let global = self.ctx.globals
            .get(index)
            .ok_or(ValidateError::GlobalDoesntExist { index })?;

        Ok(global.val_type)
    }

    /// Sets (technically just types checks) the value of the global at the given index.
    pub fn global_set(&self, index: u32, val_type: ValType) -> Result<(), ValidateError> {
        let index = index as usize;

        let global = self.ctx.globals
            .get(index)
            .ok_or(ValidateError::GlobalDoesntExist { index })?;

        // types must be the same in order to set
        if global.val_type != val_type {
            return Err(ValidateError::GlobalSetTypeMismatch { expect: global.val_type, actual: val_type });
        }

        // global must be mutable
        if global.mutability != Mutability::Var {
            return Err(ValidateError::GlobalMustBeMutable { index });
        }

        Ok(())
    }

    /// Checks whether a linear module is defined in the current module, throws an error if it doesn't.
    pub fn verify_mem_exists(&self) -> Result<(), ValidateError> {
        // this is hardcoded since there's only at most one linear memory allowed in Wasm 1.0
        if self.ctx.mems.len() < 1 {
            Err(ValidateError::LinearMemoryDoesntExist)
        } else {
            Ok(())
        }
    }

    /// Checks the alignment of the given `MemArg` against the number of bytes in the `bit_width`.
    pub fn mem_load(&self, mem_arg: MemArg, bit_width: usize) -> Result<(), ValidateError> {
        self.verify_mem_exists()?;

        // alignment must not be larger than bit width divided by 8
        let alignment = 2u64.pow(mem_arg.align);
        let num_bytes = (bit_width / 8) as u64;

        if alignment > num_bytes {
            return Err(ValidateError::AlignmentIsLargerThanBitWidth{ alignment, num_bytes });
        }

        Ok(())
    }

    fn check_frame_exists(&self, index: u32) -> Result<(), ValidateError> {
        let index = index as usize;

        if self.ctrls.len() <= index {
            return Err(ValidateError::InvalidControlFrameIndex { index });
        }

        Ok(())
    }

    /// Gets the control frame at the given index.
    fn get_ctrl_frame(&self, index: u32) -> Result<&CtrlFrame, ValidateError> {
        self.check_frame_exists(index)?;

        Ok(self.ctrls
            .get(self.ctrls.len() - 1 - index as usize)
            .unwrap()
        )
    }

    pub fn br(&mut self, index: u32) -> Result<(), ValidateError> {
        let target_ctrl = self.get_ctrl_frame(index)?;

        // type-check against label types
        self.pop_opds(target_ctrl.label_types.clone())?;

        // this should allow for every other instruction left in the current control frame to be validated regardless of what's on the stack.
        self.unreachable()?;

        Ok(())
    }

    pub fn br_if(&mut self, index: u32) -> Result<(), ValidateError> {
        let target_ctrl = self.get_ctrl_frame(index)?;
        let types = target_ctrl.label_types.clone();

        // pop condition
        self.pop_opd_expect(ValType::I32)?;

        // type-check against label types
        self.pop_opds(types.clone())?;

        // fallthrough path could still run, so we push back the operands
        self.push_opds(types);

        Ok(())
    }

    /// Basically a 'br' to any target in the `frame_indices` or at the `fallback_index`.
    pub fn br_table(&mut self, frame_indices: &Vec<u32>, fallback_index: u32) -> Result<(), ValidateError> {
        // pop index
        self.pop_opd_expect(ValType::I32)?;
        
        let fallback_frame = self.get_ctrl_frame(fallback_index)?;
        let expected_types = fallback_frame.label_types.clone();
        
        for &index in frame_indices {
            let frame = self.get_ctrl_frame(index)?;

            // must have the same label type as the fallback frame
            if frame.label_types != fallback_frame.label_types {
                return Err(ValidateError::ExpectedMatchingLabelTypes { 
                    expect: expected_types, 
                    actual: frame.label_types.clone() 
                })
            }
        }

        // basically like a 'br' at this point, all targets share the same expected types so we can type-check doing this
        self.pop_opds(expected_types)?;
        self.unreachable()?;
        
        Ok(())
    }

    pub fn return_instr(&mut self) -> Result<(), ValidateError> {
        // should be the very last frame
        self.br(self.ctrls.len() as u32 - 1)
    }
}

struct Context<'a> {
    /// The types of the functions declared in the current module.
    types: &'a Vec<FuncType>,

    /// List of functions declared in the current module.
    funcs: Vec<&'a FuncType>,

    /// List of tables declared in the current module.
    tables: Vec<&'a TableType>,

    /// List of linear memories declared in the current module.
    mems: Vec<&'a Limits>,

    /// List of globals declared in the current module.
    globals: Vec<&'a GlobalType>,

    /// List of locals in the current function (incl. params)
    locals: Vec<ValType>,

    /// Return type of the current function being validated.
    return_type: Option<Vec<ValType>>,
}

impl<'a> Context<'a> {
    /// Creates a `Context` from the given module.
    fn new(module: &'a Module) -> Self {
        let types: &'a Vec<FuncType> = &module.types;

        // Types of imported functions + types of module functions
        let funcs: Vec<&'a FuncType> = module.imports
            .iter()
            .filter_map(|im| match im.desc {
                ImportDesc::Func(type_idx) => Some(&module.types[type_idx as usize]),
                _ => None,
            })
            .chain(
                module.funcs
                    .iter()
                    .map(|f| &module.types[f.type_idx as usize])
            )
            .collect();

        // types of all tables
        let tables: Vec<&'a TableType> = module.tables
            .iter()
            .map(|t| &t.table_type)
            .collect();

        // All limits in linear memories
        let mems: Vec<&'a Limits> = module.mems
            .iter()
            .map(|m| &m.mem_type)
            .collect();

        // all global types in globals
        let globals: Vec<&'a GlobalType> = module.globals
            .iter()
            .map(|g| &g.global_type)
            .collect();
        
        Self {
            types,
            funcs,
            tables,
            mems,
            globals,

            // to be filled later when we enter any structured control construct in a function body
            locals: Vec::new(),
            return_type: None
        }
    }
}

#[derive(Clone)]
struct CtrlFrame {
    label_types: Vec<ValType>,
    end_types: Vec<ValType>,
    height: usize,
    unreachable: bool,
}