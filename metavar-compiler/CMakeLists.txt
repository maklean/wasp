cmake_minimum_required(VERSION 3.31)

project(wasp_metavar_compiler LANGUAGES CXX)

# fetch nlohmann/json
include(FetchContent)

FetchContent_Declare(
    json
    URL https://github.com/nlohmann/json/releases/download/v3.11.3/json.tar.xz
)
FetchContent_MakeAvailable(json)

# Setup LLVM
find_package(LLVM REQUIRED CONFIG COMPONENTS
    core orcjit irreader support executionengine native target
)

include(LLVMConfig)

# stencils entry point
set(STENCILS_BC "${CMAKE_BINARY_DIR}/output/stencils_entry.bc")

add_custom_command(
    OUTPUT ${STENCILS_BC}
    COMMAND ${CMAKE_COMMAND} -E make_directory ${CMAKE_BINARY_DIR}/output
    COMMAND ${CMAKE_CXX_COMPILER}
            -O3 -fno-pic -fno-pie -mcmodel=medium -emit-llvm
            -std=c++17
            -I${CMAKE_CURRENT_SOURCE_DIR}/include
            -c ${CMAKE_CURRENT_SOURCE_DIR}/src/library/stencils_entry.cpp
            -o ${STENCILS_BC}
    DEPENDS ${CMAKE_CURRENT_SOURCE_DIR}/src/library/stencils_entry.cpp
    COMMENT "Compiling stencils_entry.cpp to LLVM bitcode"
    VERBATIM
)

add_custom_target(stencils_entry ALL DEPENDS ${STENCILS_BC})

# metavar compiler executable
add_executable(main
    src/main.cpp 
    src/library/stencil_registry.cpp 
    src/utils/fake_symbol_resolver.cpp 
    src/utils/relocation_type_resolver.cpp
)

set_target_properties(main PROPERTIES
    CXX_STANDARD 17
    CXX_STANDARD_REQUIRED ON
    CXX_EXTENSIONS OFF
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/output"
)

target_include_directories(main PRIVATE ${LLVM_INCLUDE_DIRS})
target_include_directories(main PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/include/)
target_compile_definitions(main PRIVATE ${LLVM_DEFINITIONS})

llvm_map_components_to_libnames(llvm_libs 
    core orcjit irreader support executionengine native target
)

target_link_libraries(main PRIVATE 
    ${llvm_libs} 
    ${LLVM_SYSTEM_LIBS}
    nlohmann_json::nlohmann_json
    dl 
    pthread
)

add_dependencies(main stencils_entry)