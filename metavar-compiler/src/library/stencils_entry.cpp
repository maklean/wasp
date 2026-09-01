#include "../../include/library/stencils_entry.hpp"
#include "../../include/library/stencil_registry.hpp"
#include "../../include/library/generators/export.hpp"

extern "C" void __wasp_build_stencil_library__() {
    using namespace wasp;

    // RegisterBoilerplate<StencilGeneratorClass>("StencilGeneratorName")
    RegisterBoilerplate<WasmNoop>("WasmNoop");
    RegisterBoilerplate<WasmConstant>("WasmConstant");
    RegisterBoilerplate<WasmI32Add>("WasmI32Add");
}