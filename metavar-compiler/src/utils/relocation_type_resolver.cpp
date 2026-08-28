#include "../../include/utils/relocation_type_resolver.hpp"

#include <string_view>
#include <cstdint>
#include <string>

#define CHECK_PREFIX(pre) \
    std::string_view prefix = pre; \
    return symbol.length() > prefix.length() && symbol.substr(0, prefix.length()) == prefix;

bool IsMustTailPlaceholder(std::string_view symbol) { CHECK_PREFIX(MUSTTAIL_PREFIX) }

bool IsNoTailPlaceholder(std::string_view symbol) { CHECK_PREFIX(NOTAIL_PREFIX) }

bool IsDataPlaceholder(std::string_view symbol) { CHECK_PREFIX(DATA_PREFIX) }

uint32_t ExtractOrdinal(std::string_view symbol, std::string_view prefix) {
    std::string_view ordinalStr = symbol.substr(prefix.length());
    
    return static_cast<uint32_t>(std::stoul(std::string { ordinalStr }));
}