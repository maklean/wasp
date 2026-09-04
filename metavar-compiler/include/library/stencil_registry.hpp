#pragma once

#include "../metavar/metavar_result.hpp"

#include <string>
#include <vector>

namespace wasp {

// Stencil generator name + instance list.
struct BoilerplatePack {
    // Name of the stencil generator (e.g., WasmNoop)
    std::string m_name;

    // Instance list for the stencil generator.
    MetaVarMaterializedList m_data;
};

#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wreturn-type-c-linkage"

// Returns the global boilerplate packs vector.
extern "C" std::vector<BoilerplatePack>& GetAllBoilerplatePacks();

#pragma clang diagnostic pop

// Adds a BoilerplatePack to the global BoilerplatePack list.
inline void __wasp_register_boilerplate_pack__(const char* name, MetaVarMaterializedList* data) {
    GetAllBoilerplatePacks().push_back(BoilerplatePack { name, std::move(*data) });
}

// Generates the stencil variants for the given stencil generator 
// and registers it in the global boilerplate packs vector.
template<typename T>
void RegisterBoilerplate(const char *name) {
    MetaVarMaterializedList result = T::metavars().template Materialize<T>();

    __wasp_register_boilerplate_pack__(name, &result);
}

}; // namespace wasp