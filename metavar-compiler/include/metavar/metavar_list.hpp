#pragma once

#include "metavar_result.hpp"
#include "metavar_materialize.hpp"

#include <vector>

namespace wasp {

// List of TypedMetaVar
template<auto... metaVarTypes>
class TypedMetaVarList {
private:
    // Accumulated MetaVars
    std::vector<MetaVar> m_mvl;

    // Base case
    void BuildMetaVarList() {}

    // Recursive case - populate TypedMetaVarList::m_mvl
    template<typename CurrMetaVarType, typename... RemainingMetaVarTypes>
    void BuildMetaVarList(CurrMetaVarType curr, RemainingMetaVarTypes... remaining) {
        m_mvl.push_back(curr);
        BuildMetaVarList(remaining...);
    }

    TypedMetaVarList(TypedMetaVar<metaVarTypes>... metavars) {
        BuildMetaVarList(metavars...);
    }
public:
    // Creates a TypedMetaVarList from a sequence of TypedMetaVar.
    template<auto... _metaVarTypes>
    friend TypedMetaVarList<_metaVarTypes...> CreateMetaVarList(TypedMetaVar<_metaVarTypes>... metavars);

    // Generates a MetaVarMaterializedList from a TypedMetaVarList and a Materializer/StencilGenerator
    template<typename Materializer>
    MetaVarMaterializedList Materialize() {
        MetaVarMaterializedList result;

        result.m_metavars = m_mvl;
        internal::metavar_materialize_helper<Materializer, metaVarTypes...>::materialize(&result);

        return result;
    }
};

template<auto... metaVarTypes>
TypedMetaVarList<metaVarTypes...> CreateMetaVarList(TypedMetaVar<metaVarTypes>... metavars) {
    static_assert(sizeof...(metaVarTypes) > 0, "Cannot create TypedMetaVarList from empty list of metavars.");
    
    return TypedMetaVarList<metaVarTypes...>(metavars...);
}

}; // namespace wasp