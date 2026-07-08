# wasp

A WebAssembly (Wasm) interpreter written in Rust following the [WebAssembly 1.0 spec](https://www.w3.org/TR/wasm-core-1/). Building it as an intersection between my interests
in web infrastructure and interpreters.

## TODO

To make the base interpreter, I'm just implementing each semantic phase in order:

- [X] Decoder (parses all Wasm sections, instructions, expressions into an in-memory module representation)
    - **NOTE:** I've added per-section unit tests for the decoding (which you can run with `cargo test`), eventually I plan on getting rid of those and test the entire interpreter using the Wasm spec test suite instead.
- [X] Validator (type checking functions & instructions)
- [ ] Execution (executing the module through a module instance)

Once the interpreter passes the spec test suite, next up is a baseline JIT.