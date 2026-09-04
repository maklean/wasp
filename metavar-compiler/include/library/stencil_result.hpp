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
    // The type of stencil hole this relocation is.
    RelocationKind m_kind;

    // The offset of the relocation in the function (where it's situated).
    uint64_t m_offset;

    // The ordinal of the stencil hole.
    uint32_t m_ordinal;

    // Raw ELF relocation type.
    uint32_t m_elfRelocType;

    // r_addend from RELA entry.
    int64_t m_addend;
};

struct Stencil {
    // Symbol name of the stencil.
    std::string m_name;

    // Stencil machine code.
    std::vector<uint8_t> m_code;

    // Stencil's relocations.
    std::vector<Relocation> m_relocations;
};