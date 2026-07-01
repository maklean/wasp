use crate::{definitions::{FuncType, GlobalType, ImportDesc, Limits, TableType}, errors::ValidateError, module::Module};

pub struct Validator;

impl Validator {
    pub fn validate(module: &Module) -> Result<(), ValidateError> {
        let ctx = Context::new(module);

        /*
            NOTE: I'm gonna make it so each type has a validate() method
            that follows the validation rules from the spec.

            If there's any errors, a ValidateError will be propagated back
            up to this function and returned.

            This tree-walking approach should make it easy to validate
            everything since each type calls validate() on other types
            (its members) and those calls can turn into other validate()
            calls, allowing me to validate everything easily.

            That being said, I have no idea how I'm gonna do this yet, but it's
            good I came up with this idea :)
        */
        
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