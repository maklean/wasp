#pragma once

#include <cstdint>
#include <string>
#include <vector>

enum class RelocationKind {
    // __dyn_specialization_musttail_boilerplate_function_placeholder_N hole
    TailCall,

    // __dyn_specialization_notail_boilerplate_function_placeholder_ hole
    NonTailCall,

    // __dyn_specialization_data_placeholder_ hole
    U64Immediate,
};

struct Relocation {
    RelocationKind kind;
};

struct Stencil {
    // Symbol name of the stencil.
    std::string m_name;

    // Stencil machine code.
    std::vector<uint8_t> m_code;

    // Stencil's relocations.
    std::vector<Relocation> m_relocations;
};