#pragma once

namespace wasp {

// Max number of integral parameters allowed at a time.
const int MAX_INTEGRAL_PARAMS = 3;

// Max number of floating parameters allowed at a time.
const int MAX_FLOATING_PARAMS = 3;

// Enumeration for the max number of integral params allowed at a time (needed for template recursion)
enum class NumOpaqueIntegralParams {
    X_END_OF_ENUM = MAX_INTEGRAL_PARAMS + 1
};

// Enumeration for the max number of floating params allowed at a time (needed for template recursion)
enum class NumOpaqueFloatingParams {
    X_END_OF_ENUM = MAX_FLOATING_PARAMS + 1
};

}; // namespace wasp