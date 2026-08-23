#pragma once

#include "../utils/opaque_params.hpp"

#include <type_traits>

namespace wasp {

enum class MetaVarType {
    // Enumerates through all the Wasm primitive types: { i32, i64, f32, f64 }
    WASM_PRIMITIVE_TYPE,

    // Enumerates through { false, true }
    BOOL,

    // Enumerates through all enumerators in the enum class (except for the `X_END_OF_ENUM` enumerator)
    ENUM,
};

// Runtime metavar.
struct MetaVar {
    // Type of the MetaVar.
    MetaVarType m_type;

    // The name of the MetaVar (for informational/debugging purposes only).
    const char* m_name{};

    // The name of the enum variant (for informational/debugging purposes only - ONLY RELEVANT IF `m_type` == `wasp::MetaVarType::ENUM`)
    const char* m_enum_typename{};

    // The upperbound of the enum (ONLY RELEVANT IF `m_type` == `wasp::MetaVarType::ENUM`)
    int m_enum_upperbound{};
};

// Compile-time Metavar which is meant to be iterated over. 'v' holds the type of the MetaVar.
template<auto v>
class TypedMetaVar : public MetaVar {
private:
    TypedMetaVar() = default;
public:
    /*
        Creates a `TypedMetaVar` from an exclusive upperbound enum variant (conventionally named X_END_OF_ENUM).
        All enumerators in [0, X_END_OF_ENUM) will be later enumerated.
    */
    template<auto enumUpperBoundExclusive>
    friend TypedMetaVar<enumUpperBoundExclusive> CreateEnumMetaVar(const char* name);

    /*
        Creates a `TypedMetaVar` for a `WASM_PRIMITIVE_TYPE` MetaVar.
        All Wasm primitive types (i32, i64, f32, f64) will be later enumerated.
    */
    friend TypedMetaVar<MetaVarType::WASM_PRIMITIVE_TYPE> CreatePrimitiveTypeMetaVar(const char* name);

    /*
        Creates a `TypedMetaVar` for a `BOOL` MetaVar.
        All boolean values (false, true) will be later enumerated.
    */
    friend TypedMetaVar<MetaVarType::BOOL> CreateBoolMetaVar(const char* name);

    /*
        Creates an enum `TypedMetaVar` for a max integral parameter count.
        All values in [0, maxIntegralParamsInclusive] will be later enumerated.
    */
    template<int maxIntegralParamsInclusive>
    friend TypedMetaVar<static_cast<NumOpaqueIntegralParams>(maxIntegralParamsInclusive + 1)> CreateOpaqueIntegralParamsLimit();

    /*
        Creates an enum `TypedMetaVar` for a max float parameter count.
        All values in [0, maxFloatingParamsInclusive] will be later enumerated.
    */
    template<int maxFloatingParamsInclusive>
    friend TypedMetaVar<static_cast<NumOpaqueFloatingParams>(maxFloatingParamsInclusive + 1)> CreateOpaqueFloatingParamsLimit();
};

template<auto enumUpperBoundExclusive>
TypedMetaVar<enumUpperBoundExclusive> CreateEnumMetaVar(const char* name) {
    using EnumType = decltype(enumUpperBoundExclusive);

    // check validity of arguments
    static_assert(std::is_enum<EnumType>::value, "The provided value is not from an enum type.");
    static_assert(!std::is_same<EnumType, MetaVarType>::value, "Cannot use a MetaVarType as a type to be enumerated over.");
    static_assert(static_cast<int>(enumUpperBoundExclusive) > 0, "Enum upperbound must be greater than 0.");

    // create TypedMetaVar
    TypedMetaVar<enumUpperBoundExclusive> ret;

    ret.m_type = MetaVarType::ENUM;
    ret.m_name = name;
    ret.m_enum_typename = typeid(EnumType).name(); // it'll be mangled, but eh.
    ret.m_enum_upperbound = static_cast<int>(enumUpperBoundExclusive);

    return ret;
}

TypedMetaVar<MetaVarType::WASM_PRIMITIVE_TYPE> CreatePrimitiveTypeMetaVar(const char* name) {
    TypedMetaVar<MetaVarType::WASM_PRIMITIVE_TYPE> ret;

    ret.m_type = MetaVarType::WASM_PRIMITIVE_TYPE;
    ret.m_name = name;

    return ret;
}

TypedMetaVar<MetaVarType::BOOL> CreateBoolMetaVar(const char* name) {
    // create TypedMetaVar
    TypedMetaVar<MetaVarType::BOOL> ret;

    ret.m_type = MetaVarType::BOOL;
    ret.m_name = name;

    return ret;
}

template<int maxIntegralParamsInclusive = MAX_INTEGRAL_PARAMS>
TypedMetaVar<static_cast<NumOpaqueIntegralParams>(maxIntegralParamsInclusive + 1)> CreateOpaqueIntegralParamsLimit() {
    static_assert(0 <= maxIntegralParamsInclusive && maxIntegralParamsInclusive <= MAX_INTEGRAL_PARAMS, "maxIntegralParamsInclusive has to be in [0, MAX_INTEGRAL_PARAMS].");

    TypedMetaVar<static_cast<NumOpaqueIntegralParams>(maxIntegralParamsInclusive + 1)> ret;

    ret.m_type = MetaVarType::ENUM;
    ret.m_name = "numOpaqueIntegralParams";
    ret.m_enum_upperbound = maxIntegralParamsInclusive + 1; // convert to exclusive upperbound
    ret.m_enum_typename = "NumOpaqueIntegralParams";

    return ret;
}

template<int maxFloatingParamsInclusive = MAX_FLOATING_PARAMS>
TypedMetaVar<static_cast<NumOpaqueFloatingParams>(maxFloatingParamsInclusive + 1)> CreateOpaqueFloatingParamsLimit() {
    static_assert(0 <= maxFloatingParamsInclusive && maxFloatingParamsInclusive <= MAX_FLOATING_PARAMS, "maxFloatingParamsInclusive has to be in [0, MAX_FLOATING_PARAMS].");

    TypedMetaVar<static_cast<NumOpaqueFloatingParams>(maxFloatingParamsInclusive + 1)> ret;

    ret.m_type = MetaVarType::ENUM;
    ret.m_name = "numOpaqueFloatingParams";
    ret.m_enum_upperbound = maxFloatingParamsInclusive + 1; // convert to exclusive upperbound
    ret.m_enum_typename = "NumOpaqueFloatingParams";

    return ret;
}

}; // namespace wasp