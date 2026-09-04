# metavar-compiler

The MetaVar compiler constructs the stencil library (basically a JSON manifest) from stencil generators. Each stencil generator (which represents one or more
bytecode instructions) defines meta-variables to express different stencil variants.

## Installation Prereqs

- CMake (at least version 3.28)
- LLVM 19
- libffi
- zlib
- libxml2
- libzstd-dev

To install on Ubuntu/Debian:
```bash
sudo apt update
sudo apt install -y clang-19 clang libffi-dev zlib1g-dev libxml2-dev libzstd-dev llvm-19-dev cmake
```

## Building

The MetaVar compiler depends on tricks (GHC convention, tailcalls, dynamic relocations) that require compiling with clang `-O3` specifically:
```bash
cmake -B build -DCMAKE_CXX_COMPILER=clang++-19
cmake --build build -j$(nproc)
```

The resulting executable should be at `build/output/main`

> After running the executable, the stencil JSON manifest should be at `build/output/stencils.json`

## TODO:

- [X] Make CMake file for the MV compiler
- [ ] Implement stencil generators for other Wasm instructions