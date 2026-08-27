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
#include <utility>

#define DEFAULT_ENTRY_POINT_DIR "output/stencils_entry.bc"
#define DEFAULT_STENCILS_OBJ_FILE_DIR "output/stencils.o"

// Initializes all the necessary tools in LLVM.
static void initLLVM(int argc, char** argv);

// Executes the stencil library entry point using LLJIT, returns a set of the symbol names of all generated stencil functions.
static std::unordered_set<std::string> retrieveStencilSymbolNames(const char* executableDir, std::string_view entryPointDir);

// Emits an object file (compiled with the GHC calling convention) containing all (given) generated stencil functions.
static void emitStencilObjFile(const std::unordered_set<std::string>& stencilSymbolNames, const char* executableDir, std::string_view entryPointDir, std::string_view objFileDir);

// Loads and parses the stencils object file.
static std::pair<std::unique_ptr<llvm::object::ObjectFile>, llvm::object::ELF64LEObjectFile*> parseStencilsObjectFile(std::string_view objFileDir);

int main(int argc, char** argv) {
    initLLVM(argc, argv);

    std::string_view entryPointDir = argc >= 2 ? argv[1] : DEFAULT_ENTRY_POINT_DIR;
    std::string_view objFileDir = argc >= 3 ? argv[2] : DEFAULT_STENCILS_OBJ_FILE_DIR;

    auto set = retrieveStencilSymbolNames(argv[0], entryPointDir);
    emitStencilObjFile(set, argv[0], entryPointDir, objFileDir);

    auto objs = parseStencilsObjectFile(objFileDir);

    auto obj = std::move(objs.first);
    auto elfObj = std::move(objs.second);

    return 0;
}

static void initLLVM(int argc, char** argv) {
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

static std::unordered_set<std::string> retrieveStencilSymbolNames(const char* executableDir, std::string_view entryPointDir) {
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

    #ifdef LOG_OUTPUT
        std::cout << "[LOG] Retrieved all " << stencilSymbolNames.size() << " stencil functions symbol names.\n";
    #endif

    return stencilSymbolNames;
}

static void emitStencilObjFile(const std::unordered_set<std::string>& stencilSymbolNames, const char* executableDir, std::string_view entryPointDir, std::string_view objFileDir) {
    #ifdef LOG_OUTPUT
        std::cout << '\n';
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

        size_t setGHC{}, deletedBody{};
    #endif

    // Set calling convention of stencil functions to GHC, delete any other functions (so we get a smaller object file)
    for(auto& func: module->functions()) {
        if(func.isDeclaration()) continue;

        std::string funcName = func.getName().str();
        if(stencilSymbolNames.count(funcName)) {
            func.setCallingConv(llvm::CallingConv::GHC);

            #ifdef LOG_OUTPUT
                setGHC++;
            #endif
        } else {
            func.deleteBody();

            #ifdef LOG_OUTPUT
                deletedBody++;
            #endif
        }
    }

    #ifdef LOG_OUTPUT
        std::cout << "[LOG] Set GHC Calling Convention (" << setGHC << " functions)" << " and removed any unused functions (" << deletedBody << " functions).\n";
    #endif

    // get target for the current (host) machine's architecture
    std::string targetTriple = llvm::Triple(llvm::sys::getDefaultTargetTriple()).normalize();

    std::string errorStr;
    const llvm::Target* target = llvm::TargetRegistry::lookupTarget(targetTriple, errorStr);
    if(!target) {
        std::cerr << "Failed to get target: " << errorStr << '\n';
        exit(EXIT_FAILURE);
    }

    // build target machine from target
    llvm::TargetOptions options;
    options.FunctionSections = true; // easier to deal with relocations if each stencil function is separated

    auto targetMachine = std::unique_ptr<llvm::TargetMachine>(
        target->createTargetMachine(
            targetTriple,
            "generic",
            "",
            options,
            llvm::Reloc::Static,
            llvm::CodeModel::Medium, // paper suggests -mcmodel=medium
            llvm::CodeGenOptLevel::Aggressive // paper suggest -O3
        )
    );
    
    module->setDataLayout(targetMachine->createDataLayout());
    module->setTargetTriple(targetTriple);

    #ifdef LOG_OUTPUT
        std::cout << "[LOG] Created target machine (" << targetTriple << ").\n";
    #endif

    // emit stencils object file
    std::error_code ec;
    llvm::raw_fd_ostream dest(objFileDir, ec, llvm::sys::fs::OF_None);

    if(ec) {
        std::cerr << "Could not open file: " << ec.message() << '\n';
        exit(EXIT_FAILURE);
    }

    llvm::legacy::PassManager pass;
    if(targetMachine->addPassesToEmitFile(pass, dest, nullptr, llvm::CodeGenFileType::ObjectFile)) {
        std::cerr << "TargetMachine can't emit an object file for this target (" << targetTriple << ").\n";
        exit(EXIT_FAILURE);
    }

    pass.run(*module);
    dest.flush();

    #ifdef LOG_OUTPUT
        std::cout << "[LOG] Emitted stencils object file at: " << objFileDir << '\n';
    #endif
}

static std::pair<std::unique_ptr<llvm::object::ObjectFile>, llvm::object::ELF64LEObjectFile*> parseStencilsObjectFile(std::string_view objFileDir) {
    #ifdef LOG_OUTPUT
        std::cout << '\n';
    #endif

    auto bufferOfErr = llvm::MemoryBuffer::getFile(objFileDir);
    if(!bufferOfErr) {
        std::cerr << "Failed to read stencils object file(" << objFileDir << ").\n";
        exit(EXIT_FAILURE);
    }

    auto objOrErr = llvm::object::ObjectFile::createObjectFile(bufferOfErr.get()->getMemBufferRef());
    if(!objOrErr) {
        std::cerr << "Failed to parse stencils object file.\n";
        exit(EXIT_FAILURE);
    }

    std::unique_ptr<llvm::object::ObjectFile> obj = std::move(objOrErr.get());
    
    auto* elfObj = llvm::dyn_cast<llvm::object::ELF64LEObjectFile>(obj.get());
    if(!elfObj) {
        std::cerr << "Failed casting to ELF64 object file.\n";
        exit(EXIT_FAILURE);
    }

    #ifdef LOG_OUTPUT
        std::cout << "[LOG] Loaded and parsed '" << objFileDir << "'\n";
    #endif

    return { std::move(obj), std::move(elfObj) };
}