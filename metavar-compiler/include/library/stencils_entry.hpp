#pragma once

/*
    Registers all stencil generators into the global BoilerplatePack list.

    NOTE: The source file should be compiled to LLVM IR, and loaded by the
    MetaVar Compiler entry point so this function can be executed by LLJIT.
*/
extern "C" void __wasp_build_stencil_library__();