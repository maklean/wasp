#!/bin/bash

# delete old output (if any)
rm -rf output

# create 'output' folder
mkdir -p output

# compile stencils entry point to LLVM IR
echo "Compiling stencils entry point to LLVM IR..."
clang++ -std=c++17 -O3 -fno-pic -fno-pie -mcmodel=medium -emit-llvm -c src/library/stencils_entry.cpp -o output/stencils_entry.bc

# compile metavar compiler to executable
echo "Compiling MetaVar Compiler to an executable..."
clang++ -std=c++17 src/main.cpp src/library/stencil_registry.cpp src/utils/fake_symbol_resolver.cpp src/utils/relocation_type_resolver.cpp -o output/main \
        $(llvm-config --cxxflags --ldflags --system-libs --libs core orcjit irreader support executionengine native target) \
        -ldl -lpthread

# run metavar compiler executable
echo -e "Running executable...\n"
./output/main