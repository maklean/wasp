#include "../../include/library/stencil_registry.hpp"

namespace wasp {

/*
    NOTE: make sure to pass the address of this definition when resolving
    the symbol for this function - this is so we can access the result vector
    after executing the entry point through LLJIT has finished populating it.
*/
extern "C" std::vector<BoilerplatePack>& GetAllBoilerplatePacks() {
    static std::vector<BoilerplatePack> v{};
    return v;
}

}; // namespace wasp