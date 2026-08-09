use std::rc::Rc;

use crate::{errors::ExecutionError, runtime::{ModuleInstance, Store}, structure::{Func, FuncType, Mutability, ValType}};

/// Wasm Address.
pub type Addr = usize;

/// Wasm table function element.
pub type FuncElem = Option<Addr>;

/// Runtime representation of a Wasm value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Val {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64)
}

impl Val {
    /// Returns the value inside a `Val::I32`.
    pub fn as_i32(&self) -> i32 { 
        match self {
            Self::I32(v) => *v,
            _ => unreachable!("cannot convert to i32 from another value type.")
        }
    }

    /// Returns the value inside a `Val::I64`.
    pub fn as_i64(&self) -> i64 { 
        match self {
            Self::I64(v) => *v,
            _ => unreachable!("cannot convert to i64 from another value type.")
        }
    }

    /// Returns the value inside a `Val::F32`.
    pub fn as_f32(&self) -> f32 { 
        match self {
            Self::F32(v) => *v,
            _ => unreachable!("cannot convert to f32 from another value type.")
        }
    }

    /// Returns the value inside a `Val::F64`.
    pub fn as_f64(&self) -> f64 { 
        match self {
            Self::F64(v) => *v,
            _ => unreachable!("cannot convert to f64 from another value type.")
        }
    }

    /// Returns the zero'ed `Val` of the given `ValType`.
    pub fn zero(val_type: ValType) -> Self {
        match val_type {
            ValType::I32 => Self::I32(0),
            ValType::I64 => Self::I64(0),
            ValType::F32 => Self::F32(0.0),
            ValType::F64 => Self::F64(0.0),

            _ => panic!("Called Val::zero() on ValType::Unknown.")
        }
    }
}

impl Into<ValType> for Val {
    fn into(self) -> ValType {
        match self {
            Self::I32(_) => ValType::I32,
            Self::I64(_) => ValType::I64,
            Self::F32(_) => ValType::F32,
            Self::F64(_) => ValType::F64
        }
    }
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

/// Wasm Host Function.
pub struct HostFunc {
    /// Function callback.
    pub func: Box<dyn Fn(&mut Store, Vec<Val>) -> Result<Vec<Val>, ExecutionError>>,
}

impl HostFunc {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&mut Store, Vec<Val>) -> Result<Vec<Val>, ExecutionError> + 'static,
    {
        Self { func: Box::new(f) }
    }
}

/// Runtime representation of a Wasm table.
pub struct TableInstance {
    /// Function elements stored in the table.
    pub elem: Vec<FuncElem>,

    /// Maximum size of the table.
    pub max: Option<u32>,
}

/// Runtime representation of a Wasm linear memory.
pub struct MemInstance {
    /// Bytes stored in the memory.
    pub data: Vec<u8>,

    /// Maximum size of the memory.
    pub max: Option<u32>,
}

/// Runtime representation of a Wasm global variable.
pub struct GlobalInstance {
    /// Runtime value of the lobal.
    pub value: Val,

    /// Mutability of the global.
    pub mutability: Mutability,
}

/// Runtime representation of a Wasm export.
pub struct ExportInstance {
    /// Name of export.
    pub name: String,

    /// Value of export.
    pub value: ExternVal,
}

/// Wasm module import/export value.
#[derive(Clone, Copy)]
pub enum ExternVal {
    /// Imported/Exported function address in store.
    Func(Addr),

    /// Imported/Exported table address in store.
    Table(Addr),

    /// Imported/Exported memory address in store.
    Mem(Addr),

    /// Imported/Exported global address in store.
    Global(Addr),
}

