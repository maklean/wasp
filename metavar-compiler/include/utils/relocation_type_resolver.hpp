#pragma once

#include <string_view>
#include <cstdint>

// NOTE: should be consistent with the symbol names in stencil_placeholders.hpp
#define MUSTTAIL_PREFIX "__dyn_specialization_musttail_boilerplate_function_placeholder_"
#define NOTAIL_PREFIX "__dyn_specialization_notail_boilerplate_function_placeholder_"
#define DATA_PREFIX "__dyn_specialization_data_placeholder_"

// Returns whether the symbol name is a must-tail symbol.
bool IsMustTailPlaceholder(std::string_view symbol);

// Returns whether the symbol name is a no-tail symbol.
bool IsNoTailPlaceholder(std::string_view symbol);

// Returns whether the symbol name is a data symbol.
bool IsDataPlaceholder(std::string_view symbol);

// Extracts the ordinal from the symbol name with the given prefix.
uint32_t ExtractOrdinal(std::string_view symbol, std::string_view prefix);