# metavar-compiler

The MetaVar compiler constructs the stencil library (basically a JSON manifest) from stencil generators. Each stencil generator (which represents one or more
bytecode instructions) defines meta-variables to express different stencil variants.

## TODO:

- [X] Make CMake file for the MV compiler
- [ ] Implement stencil generators for other Wasm instructions