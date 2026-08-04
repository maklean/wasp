# wasp - A WebAssembly 1.0 Runtime

A WebAssembly (Wasm) runtime written in Rust following the [WebAssembly 1.0 spec](https://www.w3.org/TR/wasm-core-1/). Building it as an intersection between my interests in web infrastructure and interpreters.

It also fully passes the WebAssembly 1.0 spec test suite - which you can run with `cargo test`.
> Make sure you have `wabt` installed on your system, because I use `wast2json`.

**NOTE:** There's [another branch](https://github.com/maklean/wasp/tree/tree-walk) that uses a tree-walk execution model, where the instruction sequence forms an AST and the Rust call stack is used to handle nested function calls and control constructs. Had to ditch it because of its inability to handle deep recursion without pre-allocating more stack memory.
