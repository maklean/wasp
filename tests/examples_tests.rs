use std::rc::Rc;
use rand::RngExt;

use wasp::{errors::{ExecutionError, TrapReason}, module::Module, runtime::{ExternVal, FuncInstance, HostFunc, Store, Val}, structure::{FuncType, ValType}};

#[test]
fn simple_arithmetic() {
    let mut module = Module::decode_from_file("tests/assets/examples/binaries/simple_arithmetic.wasm")
        .expect("simple_arithmetic module should decode properly.");

    module.validate()
        .expect("simple_arithmetic module is valid.");
    
    let mut store = Store::new();
    let instance = module.instantiate(&mut store, &[])
        .expect("simple_arithmetic module should instantiate properly.");

    // call add(10, 20)
    let add_results = instance.invoke_exported_function("add", &[Val::I32(10), Val::I32(20)], &mut store)
        .expect("add(10, 20) should execute properly.");

    assert_eq!(add_results.len(), 1);
    assert_eq!(add_results[0], Val::I32(30));

    // call fact(15)
    let fact_results = instance.invoke_exported_function("fact", &[Val::I64(15)], &mut store)
        .expect("fact(15) should execute properly.");

    assert_eq!(fact_results.len(), 1);
    assert_eq!(fact_results[0], Val::I64(1_307_674_368_000));

    // call fact(-10) - should trap.
    let fact_err_results = instance.invoke_exported_function("fact", &[Val::I64(-10)], &mut store);
    assert!(matches!(fact_err_results, Err(ExecutionError::Trapped(TrapReason::Unreachable))))
}

#[test]
fn host_function_imports() {
    let mut module = Module::decode_from_file("tests/assets/examples/binaries/host_function_imports.wasm")
        .expect("host_function_imports module should decode properly.");

    module.validate()
        .expect("host_function_imports module is valid.");

    let mut store = Store::new();
    
    // TODO: make Store::add_function() method

    // add random_i32(i32, i32) -> i32 to the store.
    let random_i32_addr = store.funcs.len();
    store.funcs.push(FuncInstance::Host { 
        func_type: Rc::new(FuncType {
            params: vec![ValType::I32, ValType::I32],
            results: vec![ValType::I32]
        }),

        code: Rc::new(HostFunc::new(|_store, args| {
            let (min, max) = (args[0].as_i32(), args[1].as_i32());
            let mut rng = rand::rng();

            Ok(vec![Val::I32(rng.random_range(min..=max))])
        }))
    });

    // add print_i32(i32) to the store.
    let print_i32_addr = store.funcs.len();
    store.funcs.push(FuncInstance::Host { 
        func_type: Rc::new(FuncType {
            params: vec![ValType::I32],
            results: vec![]
        }),

        code: Rc::new(HostFunc::new(|_store, args| {
            println!("{}", args[0].as_i32());
            Ok(vec![])
        }))
    });

    // instantiate module with imports
    let instance = module.instantiate(&mut store, &[ExternVal::Func(random_i32_addr), ExternVal::Func(print_i32_addr)])
        .expect("simple_arithmetic module should instantiate properly.");

    // call add_random_i32() - should print three numbers to the console. The two add operands, then the result.
    instance.invoke_exported_function("add_random_i32", &[], &mut store)
        .expect("add_random_i32() should execute properly.");
}