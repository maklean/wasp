#pragma once

#include "typed_metavar.hpp"
#include "../utils/function_attributes.hpp"

#include <vector>
#include <cstdint>

namespace wasp {

// An instance of <stencilConfig, stencilFunctionAddr> pair.
struct MetaVarMaterializedInstance {
    // The combination of MetaVar values for this specific instance.
    std::vector<uint64_t> m_values;

    // The address of the generated stencil variant function.
    void* m_fnPtr;
};

// List of all metavars and generated stencil functions for a stencil generator.
struct MetaVarMaterializedList {
    // MetaVars declared by the stencil generator (should be parallel to `MetaVarMaterializedInstance::m_values`).
    std::vector<MetaVar> m_metavars;

    // Generated stencil functions.
    std::vector<MetaVarMaterializedInstance> m_instances;
};

namespace internal {

// Used to build up a runtime representation of the MetaVar values as we build a new stencil variant.
struct PartialMetaVarInstance {
    // MetaVar values.
    std::vector<uint64_t> m_values;

    /*
        Should never be inline due to resulting in slower compilation: https://github.com/sillycross/WasmNow/blob/main/fastinterp/metavar.hpp#L205

        Because this is passed through various branches during template recursion, we force each branch to have their own instance by returning a copy.
    */
    PartialMetaVarInstance NO_INLINE Push(uint64_t v) const {
        PartialMetaVarInstance ret = *this;
        ret.m_values.push_back(v);
        return ret;
    }
};

}; // namespace internal

}; // namespace wasp