use crate::{definitions::{FuncType, GlobalType, ImportDesc, Limits, TableType, ValType}, errors::ValidateError, module::Module};

pub struct Validator<'a> {
    ctx: Context<'a>,
    opds: Vec<ValType>,
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
    }

    pub fn push_opd(&mut self, val_type: ValType) {
        self.opds.push(val_type);
    }

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

    pub fn pop_opd_expect(&mut self, expect: ValType) -> Result<ValType, ValidateError> {
        let actual = self.pop_opd()?;

        // Unknown matches any type
        if actual == ValType::Unknown {
            return Ok(expect);
        } else if actual == ValType::Unknown {
            return Ok(actual);
        }

        // check for match
        if actual != expect {
            return Err(ValidateError::ExpectedOperandInOpdStack { expect, actual });
        }

        Ok(actual)
    }

    pub fn push_opds(&mut self, mut val_types: Vec<ValType>) {
        self.opds.append(&mut val_types);
    }

    pub fn pop_opds(&mut self, val_types: Vec<ValType>) -> Result<(), ValidateError> {
        for expect in val_types.iter().rev() {
            self.pop_opd_expect(expect.clone())?;
        }

        Ok(())
    }

    pub fn push_ctrl(&mut self, label_types: Vec<ValType>, end_types: Vec<ValType>) {
        let frame = CtrlFrame { label_types, end_types, height: self.opds.len(), unreachable: false };
        self.ctrls.push(frame);
    }

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

        Ok(frame.end_types)
    }

    pub fn unreachable(&mut self) -> Result<(), ValidateError> {
        let Some(frame) = self.ctrls.last_mut() else {
            return Err(ValidateError::ExpectedAtLeastOneControlFrame);
        };

        // return back to the frame's height
        self.opds.truncate(frame.height);
        frame.unreachable = true;

        Ok(())
    }

}

struct Context<'a> {
    types: &'a Vec<FuncType>,
    funcs: Vec<&'a FuncType>,
    tables: Vec<&'a TableType>,
    mems: Vec<&'a Limits>,
    globals: Vec<&'a GlobalType>,
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
            globals
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