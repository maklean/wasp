#include <iostream>

#include "../include/library/stencil_registry.hpp"
#include "../include/utils/debug.hpp"
#include "../include/utils/fake_symbol_resolver.hpp"

#include <llvm/Support/InitLLVM.h>
#include <llvm/Support/TargetSelect.h>
#include <llvm/Support/MemoryBuffer.h>
#include <llvm/Support/SourceMgr.h>
#include <llvm/IR/LLVMContext.h>
#include <llvm/IRReader/IRReader.h>

#include <llvm/Target/TargetMachine.h>
#include <llvm/MC/TargetRegistry.h>
#include <llvm/IR/LegacyPassManager.h>
#include <llvm/Support/FileSystem.h>
#include <llvm/TargetParser/Triple.h>
#include <llvm/TargetParser/Host.h>

#include <llvm/ExecutionEngine/Orc/LLJIT.h>
#include <llvm/ExecutionEngine/Orc/ThreadSafeModule.h>

#include <llvm/Object/ObjectFile.h>
#include <llvm/Object/ELFObjectFile.h>
#include <llvm/BinaryFormat/ELF.h>

#include <unordered_set>
#include <unordered_map>
#include <string>
#include <string_view>
#include <cstdlib>
#include <memory>

#define DEFAULT_ENTRY_POINT_DIR "output/stencils_entry.bc"

// Initializes all the necessary tools in LLVM.
void initLLVM(int argc, char** argv);

// Executes the stencil library entry point using LLJIT, returns a set of the symbol names of all generated stencil functions.
std::unordered_set<std::string> retrieveStencilSymbolNames(const char* executableDir, std::string_view entryPointDir);

int main(int argc, char** argv) {
    initLLVM(argc, argv);

    std::string_view entryPointDir = argc >= 2 ? argv[1] : DEFAULT_ENTRY_POINT_DIR;

    auto set = retrieveStencilSymbolNames(argv[0], entryPointDir);

    return 0;
}

void initLLVM(int argc, char** argv) {
    llvm::InitLLVM(argc, argv);

    llvm::InitializeNativeTarget();
    llvm::InitializeNativeTargetAsmPrinter();
    llvm::InitializeNativeTargetAsmParser();

    llvm::InitializeAllTargetInfos();
    llvm::InitializeAllTargets();
    llvm::InitializeAllTargetMCs();
    llvm::InitializeAllAsmPrinters();

    #ifdef LOG_OUTPUT
        std::cout << "[LOG] Initialized LLVM stuff.\n";
    #endif
}

std::unordered_set<std::string> retrieveStencilSymbolNames(const char* executableDir, std::string_view entryPointDir) {
    // create LLJIT instance
    auto jitOrErr = llvm::orc::LLJITBuilder().create();
    if(!jitOrErr) {
        std::cerr << "Failed to create LLJIT Instance: " << llvm::toString(jitOrErr.takeError()) << '\n';
        exit(EXIT_FAILURE);
    }

    auto jit = std::move(*jitOrErr);

    #ifdef LOG_OUTPUT
        std::cout << "[LOG] Created LLJIT Instance.\n";
    #endif

    // resolve symbols for GetAllBoilerplatePacks() and __wasp_register_boilerplate_pack__()
    llvm::orc::SymbolMap hostSymbols;

    hostSymbols[jit->mangleAndIntern("GetAllBoilerplatePacks")] = {
        llvm::orc::ExecutorAddr::fromPtr(&wasp::GetAllBoilerplatePacks),
        llvm::JITSymbolFlags::Exported
    };

    hostSymbols[jit->mangleAndIntern("__wasp_register_boilerplate_pack__")] = {
        llvm::orc::ExecutorAddr::fromPtr(&wasp::__wasp_register_boilerplate_pack__),
        llvm::JITSymbolFlags::Exported
    };

    auto& mainDylib = jit->getMainJITDylib();

    if(auto err = mainDylib.define(llvm::orc::absoluteSymbols(hostSymbols))) {
        std::cerr << "Failed to define host symbols: " << llvm::toString(std::move(err)) << '\n';
        exit(EXIT_FAILURE);
    }

    #ifdef LOG_OUTPUT
        std::cout << "[LOG] Resolved symbols for GetAllBoilerplatePacks() and __wasp_register_boilerplate_pack__().\n";
    #endif

    // resolve everything else that isn't in the dylib
    auto processSymbolsGenerator = llvm::orc::DynamicLibrarySearchGenerator::GetForCurrentProcess(
        jit->getDataLayout().getGlobalPrefix()
    );

    if(processSymbolsGenerator) {
        mainDylib.addGenerator(std::move(*processSymbolsGenerator));
    } else {
        std::cerr << "Failed to create process symbol generator\n";
        exit(EXIT_FAILURE);
    }

    // needed so the non-existent symbols for the stencil holes don't cause an error.
    AddFakeSymbolResolverGenerator(jit.get());

    #ifdef LOG_OUTPUT
        std::cout << "[LOG] Added fake symbol workaround thingy to JIT Instance.\n";
    #endif

    auto context = std::make_unique<llvm::LLVMContext>();

    llvm::SMDiagnostic err;
    auto module = llvm::parseIRFile(entryPointDir, err, *context);
    if(!module) {
        err.print(executableDir, llvm::errs());
        exit(EXIT_FAILURE);
    }

    #ifdef LOG_OUTPUT
        std::cout << "[LOG] Parsed stencil entry point into LLVM module.\n";
    #endif

    // needed later to map addr (from the JIT Instance) to funcNames
    std::vector<std::string> funcNames;
    for(auto& func: module->functions()) {
        if(func.isDeclaration()) continue;
        funcNames.push_back(func.getName().str());
    }

    llvm::orc::ThreadSafeModule tsm(std::move(module), std::move(context));

    if(auto err = jit->addIRModule(std::move(tsm))) {
        std::cerr << "Failed to add module to LLJIT instance: " << llvm::toString(std::move(err)) << '\n';
        exit(EXIT_FAILURE);
    }

    #ifdef LOG_OUTPUT
        std::cout << "[LOG] Added module to JIT Instance.\n";
    #endif

    // execute stencil entry point function
    auto entrySymOrErr = jit->lookup("__wasp_build_stencil_library__");
    if(!entrySymOrErr) {
        std::cerr << "Failed to find entry point symbol: " << llvm::toString(entrySymOrErr.takeError()) << '\n';
        exit(EXIT_FAILURE);
    }

    auto entryFn = entrySymOrErr->toPtr<void(*)()>();
    entryFn();

    auto& packs = wasp::GetAllBoilerplatePacks();

    #ifdef LOG_OUTPUT
        std::cout << "[LOG] Successfully populated packs: " << packs.size() << " stencil generator packs registered.\n";
    #endif

    std::unordered_map<uint64_t, std::string_view> addrToSym; // funcAddr (from JIT instance) => function symbol name
    for(const auto& funcName: funcNames) {
        auto funcAddrOrErr = jit->lookup(funcName);

        if(!funcAddrOrErr) {
            llvm::consumeError(funcAddrOrErr.takeError());
            continue;
        }

        addrToSym[funcAddrOrErr->getValue()] = funcName;
    }

    // get the symbol names of every stencil function
    std::unordered_set<std::string> stencilSymbolNames;

    for(const auto& bp: packs) {
        for(const auto& inst: bp.m_data.m_instances) {
            uint64_t stencilFuncAddr = reinterpret_cast<uint64_t>(inst.m_fnPtr);

            if(!addrToSym.count(stencilFuncAddr)) {
                std::cerr << "Failed to find stencil function address for stencil variant of: " << bp.m_name << '\n';
                exit(EXIT_FAILURE);
            }

            stencilSymbolNames.insert(std::string { addrToSym[stencilFuncAddr] });
        }
    }

    return stencilSymbolNames;
}