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

namespace OpaqueParamsHelper {
    // Returns whether we have enough space for `num` more int params in registers.
    static constexpr bool CanPush(NumOpaqueIntegralParams size, int num = 1) {
        return static_cast<int>(size) + num <= MAX_INTEGRAL_PARAMS;
    }

    // Returns whether we have enough space for `num` more float params in registers.
    static constexpr bool CanPush(NumOpaqueFloatingParams size, int num = 1) {
        return static_cast<int>(size) + num <= MAX_FLOATING_PARAMS;
    }

    // Returns whether there are no int params in registers.
    static constexpr bool IsEmpty(NumOpaqueIntegralParams size) {
        return static_cast<int>(size) == 0;
    }

    // Returns whether there are no float params in registers.
    static constexpr bool IsEmpty(NumOpaqueFloatingParams size) {
        return static_cast<int>(size) == 0;
    }
}; // namespace OpaqueParamsHelper

}; // namespace wasp