use crate::{binary::ParsedModule, errors::ValidationError, structure::{FuncType, GlobalType, ImportDesc, Limits, TableType, ValType}};

/// Wasm module validator context + stacks.
pub(crate) struct Validator<'a> {
    /// Module being validated.
    pub module: &'a ParsedModule,

    /// Function types declared in the module.
    pub types: &'a Vec<FuncType>,

    /// Functions imported and declared in the module.
    pub funcs: Vec<&'a FuncType>,

    /// Tables imported and declared in the module.
    pub tables: Vec<&'a TableType>,

    /// Linear memories imported and declared in the module.
    pub mems: Vec<&'a Limits>,

    /// Globals imported and declared in the module.
    pub globals: Vec<&'a GlobalType>,

    /// Locals in the current function being validated.
    pub locals: Vec<ValType>,

    /// Operand Stack.
    pub opds: Vec<ValType>,

    /// Control Frame Stack.
    pub ctrls: Vec<CtrlFrame>
}

impl<'a> Validator<'a> {
    fn new(module: &'a ParsedModule) -> Result<Self, ValidationError> {
        let types: &'a Vec<FuncType> = &module.types;

        // types of imported funcs + module-defined funcs
        let mut funcs: Vec<&'a FuncType> = Vec::new();

        for import in &module.imports {
            if let ImportDesc::Func(func_type_idx) = &import.desc {
                let index = *func_type_idx as usize;

                let func_type = module.types.get(index)
                    .ok_or(ValidationError::UndefinedType { index })?;

                funcs.push(func_type);
            }
        }

        for func in &module.funcs {
            let index = func.type_idx as usize;

            let func_type = module.types.get(index)
                .ok_or(ValidationError::UndefinedType { index })?;

            funcs.push(func_type);

        }
            
        // imported table types + module-defined table types
        let tables: Vec<&'a TableType> = module.imports
            .iter()
            .filter_map(|import| match &import.desc {
                ImportDesc::Table(table_type) => Some(table_type),
                _ => None
            })
            .chain(
                module.tables
                    .iter()
                    .map(|table| &table.table_type)
            ).collect();
        
        // imported mem types + module-defined mem-types
        let mems: Vec<&'a Limits> = module.imports
            .iter()
            .filter_map(|im| match &im.desc {
                ImportDesc::Mem(mem_type) => Some(mem_type),
                _ => None,
            })
            .chain(module.mems.iter().map(|mem| &mem.mem_type))
            .collect();

        // imported global types + module-defined global types
        let globals: Vec<&'a GlobalType> = module.imports
            .iter()
            .filter_map(|im| match &im.desc {
                ImportDesc::Global(global_type) => Some(global_type),
                _ => None,
            })
            .chain(module.globals.iter().map(|global| &global.global_type))
            .collect();
        
        Ok(Self {
            module,
            types,
            funcs,
            tables,
            mems,
            globals,
            locals: Vec::new(),
            opds: Vec::new(),
            ctrls: Vec::new(),
        })
    }

    pub fn validate(module: &'a ParsedModule) -> Result<(), ValidationError> {
        let mut this = Self::new(module)?;

        // validate functions
        for func in &module.funcs {
            func.validate(&mut this)?;
        }

        // validate tables
        for table in &module.tables {
            table.validate()?;
        }

        // validate linear memories
        for mem in &module.mems {
            mem.validate()?;
        }

        // validate globals
        for global in &module.globals {
            global.validate(&mut this)?;
        }

        // validate element segments
        for elem in &module.elem {
            elem.validate(&mut this)?
        }

        // validate data segments
        for data in &module.data {
            data.validate(&mut this)?;
        }

        // validate start function if it exists
        if let Some(start_func_idx) = module.start {
            let start_func_idx = start_func_idx as usize;

            let func = this.funcs
                .get(start_func_idx)
                .ok_or(ValidationError::UndefinedFunction { index: start_func_idx })?;

            // params and results have to be empty for the start function to be valid
            if !func.params.is_empty() || !func.results.is_empty() {
                return Err(ValidationError::InvalidStartFunction { params: func.params.clone(), results: func.results.clone() });
            }
        }

        // validate exports + check for unique names
        let mut seen_names = std::collections::HashSet::new();

        for export in &module.exports {
            if !seen_names.insert(&export.name) {
                return Err(ValidationError::DuplicateExportName { name: export.name.clone() });
            }

            export.validate(&mut this)?;
        }

        // validate imports
        for import in &module.imports {
            import.validate(&mut this)?;
        }

        // validate functypes
        for func_type in &module.types {
            func_type.validate()?;
        }

        // check for too many tables and mems
        if this.tables.len() > 1 {
            return Err(ValidationError::TooManyTables { count: this.tables.len() });
        }
        
        if this.mems.len() > 1 {
            return Err(ValidationError::TooManyMems { count: this.mems.len() });
        }

        Ok(())
    }

    /// Pushes the given value type onto the operand stack.
    pub fn push_opd(&mut self, val_type: ValType) {
        self.opds.push(val_type);
    }

    /// Pops a value type from the operand stack.
    /// If we're popping past the end of the current
    /// control frame while being in dead/unreachable code,
    /// `ValType::Unknown` is returned as the value type.
    pub fn pop_opd(&mut self) -> Result<ValType, ValidationError> {
        let frame = self.ctrls.last()
            .ok_or(ValidationError::ExpectedControlFrame)?;

        if self.opds.len() == frame.height && frame.unreachable {
            // we're popping in dead code, ValType::Unknown should be returned.
            return Ok(ValType::Unknown);
        } else if self.opds.len() == frame.height {
            // we're popping past the frame in live code, error.
            return Err(ValidationError::PoppingOutsideControlFrame { frame_height: frame.height })
        }

        Ok(self.opds.pop().ok_or(ValidationError::ExpectedOperandInOpdStack)?)
    }

    /// Pops a value type off the operand stack as long as it matches the expected type. Returns the expected type.
    pub fn pop_opd_expect(&mut self, expect: ValType) -> Result<ValType, ValidationError> {
        let actual = self.pop_opd()?;

        // Unknown matches any type, return other type
        if actual == ValType::Unknown {
            return Ok(expect);
        } else if expect == ValType::Unknown {
            return Ok(actual);
        }

        if actual != expect {
            return Err(ValidationError::OperandMismatch { expect, actual });
        }

        Ok(actual)
    }

    /// Pushes a collection of value types onto the operand stack.
    pub fn push_opds(&mut self, mut val_types: Vec<ValType>) {
        self.opds.append(&mut val_types);
    }

    /// Pops multiple operands from the operand stack as long as they match the given types.
    pub fn pop_opds(&mut self, val_types: Vec<ValType>) -> Result<(), ValidationError> {
        for &expect in val_types.iter().rev() {
            self.pop_opd_expect(expect)?;
        }

        Ok(())
    }

    /// Enters a new structured control construct with the given label and end types.
    /// Pushes the `CtrlFrame` onto the validator's control frame stack.
    pub fn push_ctrl(&mut self, label_types: Vec<ValType>, end_types: Vec<ValType>) {
        self.ctrls.push(CtrlFrame {
            label_types,
            end_types,
            height: self.opds.len(),
            unreachable: false,
        })
    }

    /// Exits the current structured control construct. Performs a type-check on the
    /// operand stack for the expected end types. Returns the end types of the control
    /// construct.
    pub fn pop_ctrl(&mut self) -> Result<Vec<ValType>, ValidationError> {
        let frame = self.ctrls.last().cloned()
            .ok_or(ValidationError::ExpectedControlFrame)?;
        
        // type-check for end types
        self.pop_opds(frame.end_types.clone())?;

        // should be back at the height where we entered the construct.
        if self.opds.len() != frame.height {
            return Err(ValidationError::StackHeightMismatch { expect: frame.height, actual: self.opds.len() });
        }

        self.ctrls.pop();

        Ok(frame.end_types)
    }

    /// Retrieves the control frame (starting from the top of the stack).
    pub fn get_ctrl(&self, index: u32) -> Result<&CtrlFrame, ValidationError> {
        let index = index as usize;

        if self.ctrls.len() <= index {
            return Err(ValidationError::InvalidLabelIndex { ctrl_frame_count: self.ctrls.len(), index });
        }

        Ok(self.ctrls.get(self.ctrls.len()-1-index)
            .expect("control frame should exist."))
    }

    /// Declares the current control frame as unreachable and truncates 
    /// the operand stack back to where said control frame begun.
    pub fn unreachable(&mut self) -> Result<(), ValidationError> {
        let frame = self.ctrls.last_mut()
            .ok_or(ValidationError::ExpectedControlFrame)?;

        // return back to the frame's height
        self.opds.truncate(frame.height);
        frame.unreachable = true;

        Ok(())
    }

    /// Validates a binary operator.
    pub fn binop(&mut self, t: ValType) -> Result<(), ValidationError> {
        self.pop_opd_expect(t)?;
        self.pop_opd_expect(t)?;
        self.push_opd(t);

        Ok(())
    }

    /// Validates a unary operator.
    pub fn unop(&mut self, t: ValType) -> Result<(), ValidationError> {
        self.pop_opd_expect(t)?;
        self.push_opd(t);

        Ok(())
    }

    /// Validates a test operator.
    pub fn testop(&mut self, t: ValType) -> Result<(), ValidationError> {
        self.pop_opd_expect(t)?;
        self.push_opd(ValType::I32);

        Ok(())
    }

    /// Validates a relational operator.
    pub fn relop(&mut self, t: ValType) -> Result<(), ValidationError> {
        self.pop_opd_expect(t)?;
        self.pop_opd_expect(t)?;
        self.push_opd(ValType::I32);

        Ok(())
    }

    /// Validates a conversion operator.
    pub fn cvtop(&mut self, from: ValType, to: ValType) -> Result<(), ValidationError> {
        self.pop_opd_expect(from)?;
        self.push_opd(to);

        Ok(())
    }
}

/// Structured control instruction (or function body) frame.
#[derive(Clone)]
pub struct CtrlFrame {
    /// Types expected on the operand stack when we branch to this construct.
    pub label_types: Vec<ValType>,

    /// Types expected on the operand stack when we exit this construct.
    end_types: Vec<ValType>,

    /// Height of the operand stack at the time we entered this construct.
    height: usize,

    /// Whether we're currently in dead code in this construct.
    unreachable: bool,
}