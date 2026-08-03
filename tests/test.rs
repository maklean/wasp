use wasp::{module::Module, runtime::{Store, Val}};

#[test]
fn test_module() {
    let mut module = Module::decode_from_file("tests/test.wasm")
        .expect("module should decode");

    module.validate()
        .expect("module should validate");

    let mut store = Store::new();
    let instance = module.instantiate(&mut store, &[])
        .expect("module should instantiate");

    let results = instance.invoke_exported_function("addTwo", &[Val::I32(10), Val::I32(20)], &mut store)
        .expect("function should run");

    assert_eq!(results[0].as_i32(), 30, "function result should be 30");
}