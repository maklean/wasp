#include <iostream>

#include "../include/library/stencil_registry.hpp"
#include "../include/library/stencil_result.hpp"
#include "../include/utils/debug.hpp"
#include "../include/utils/fake_symbol_resolver.hpp"
#include "../include/utils/asserts.hpp"
#include "../include/utils/relocation_type_resolver.hpp"

#include <nlohmann/json.hpp>

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

#include <llvm/Support/Path.h>

#include <unordered_set>
#include <unordered_map>
#include <string>
#include <string_view>
#include <cstdlib>
#include <memory>
#include <utility>
#include <fstream>

#define ENTRY_POINT_FNAME "stencils_entry.bc"
#define STENCILS_OBJ_FNAME "stencils.o"
#define STENCILS_MANIFEST_FNAME "stencils.json"

// Initializes all the necessary tools in LLVM.
static void initLLVM(int argc, char** argv);

// Resolves the directory for the given program file.
static std::string resolveDir(const char* argv0, std::string_view fname);

struct StencilInformation {
    std::unordered_set<std::string> stencilSymbolNames;
    std::unordered_map<std::string, std::vector<std::pair<std::string, uint64_t>>> stencilMetaVarConfigs;
};

// Executes the stencil library entry point using LLJIT, returns a set of the symbol names of all generated stencil functions with their metavar configs.
static StencilInformation retrieveStencilInformation(const char* executableDir, std::string_view entryPointDir);

// Emits an object file (compiled with the GHC calling convention) containing all (given) generated stencil functions.
static void emitStencilObjFile(const std::unordered_set<std::string>& stencilSymbolNames, const char* executableDir, std::string_view entryPointDir, std::string_view objFileDir);

// Loads and parses the stencils object file.
struct ParsedStencilsObject {
    std::unique_ptr<llvm::MemoryBuffer> buffer;
    std::unique_ptr<llvm::object::ObjectFile> object;
    llvm::object::ELF64LEObjectFile* elf;
};

static ParsedStencilsObject parseStencilsObjectFile(std::string_view objFileDir);

// Builds a relocation map from the stencils object file that maps section index to its relocations.
static std::unordered_map<uint64_t, std::vector<llvm::object::RelocationRef>> buildRelocationMap(const std::unique_ptr<llvm::object::ObjectFile>& obj);

// Builds the stencil library (maps stencil symbol name => Stencil)
static std::unordered_map<std::string, Stencil> buildStencilLibrary(
    const std::unique_ptr<llvm::object::ObjectFile>& obj, 
    llvm::object::ELF64LEObjectFile* elfObj, 
    const std::unordered_set<std::string>& stencilSymbolNames,
    const std::unordered_map<uint64_t, std::vector<llvm::object::RelocationRef>>& relocationMap
);

// Emits the JSON manifest from the stencil library hashmap.
static void emitJsonManifest(const std::unordered_map<std::string, Stencil>& stencilLibrary, const std::unordered_map<std::string, std::vector<std::pair<std::string, uint64_t>>>& stencilMetaVarConfigs, std::string_view jsonFileDir);

int main(int argc, char** argv) {
    initLLVM(argc, argv);

    std::string entryPointDir = resolveDir(argv[0], ENTRY_POINT_FNAME);
    std::string objFileDir = resolveDir(argv[0], STENCILS_OBJ_FNAME);
    std::string jsonFileDir = resolveDir(argv[0], STENCILS_MANIFEST_FNAME);

    auto stencilInformation = retrieveStencilInformation(argv[0], entryPointDir);

    auto set = std::move(stencilInformation.stencilSymbolNames);
    auto metavarConfigs = std::move(stencilInformation.stencilMetaVarConfigs);

    emitStencilObjFile(set, argv[0], entryPointDir, objFileDir);

    auto parsed = parseStencilsObjectFile(objFileDir);

    auto& obj = parsed.object;
    auto* elfObj = parsed.elf;

    auto relocationMap = buildRelocationMap(obj);

    auto lib = buildStencilLibrary(obj, elfObj, set, relocationMap);

    emitJsonManifest(lib, metavarConfigs, jsonFileDir);

    return 0;
}

static void initLLVM(int argc, char** argv) {
    llvm::InitLLVM(argc, argv);

    llvm::InitializeNativeTarget();
    llvm::InitializeNativeTargetAsmPrinter();
    llvm::InitializeNativeTargetAsmParser();

    #ifdef LOG_OUTPUT
        std::cout << "[LOG] Initialized LLVM stuff.\n";
    #endif
}

static std::string resolveDir(const char* argv0, std::string_view fname) {
    llvm::SmallString<256> exePath(argv0);

    llvm::sys::fs::make_absolute(exePath);
    llvm::sys::path::remove_filename(exePath);
    llvm::sys::path::append(exePath, fname);

    return std::string{ exePath };
}

static StencilInformation retrieveStencilInformation(const char* executableDir, std::string_view entryPointDir) {
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

    // maps symbolName => metaVar config
    std::unordered_map<std::string, std::vector<std::pair<std::string, uint64_t>>> stencilMetaVarConfigs;

    for(const auto& bp: packs) {
        for(const auto& inst: bp.m_data.m_instances) {
            uint64_t stencilFuncAddr = reinterpret_cast<uint64_t>(inst.m_fnPtr);

            if(!addrToSym.count(stencilFuncAddr)) {
                std::cerr << "Failed to find stencil function address for stencil variant of: " << bp.m_name << '\n';
                exit(EXIT_FAILURE);
            }

            std::string stencilSymbol{ addrToSym[stencilFuncAddr] };
            stencilSymbolNames.insert(stencilSymbol);

            for(size_t i{}; i < inst.m_values.size(); i++) {
                stencilMetaVarConfigs[stencilSymbol].push_back(std::pair { bp.m_data.m_metavars[i].m_name, inst.m_values[i] });
            }
        }
    }

    #ifdef LOG_OUTPUT
        std::cout << "[LOG] Retrieved all " << stencilSymbolNames.size() << " stencil functions symbol names.\n";
    #endif

    return { stencilSymbolNames, stencilMetaVarConfigs };
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

static ParsedStencilsObject parseStencilsObjectFile(std::string_view objFileDir) {
    #ifdef LOG_OUTPUT
        std::cout << '\n';
    #endif

    auto bufferOfErr = llvm::MemoryBuffer::getFile(objFileDir);
    if(!bufferOfErr) {
        std::cerr << "Failed to read stencils object file(" << objFileDir << ").\n";
        exit(EXIT_FAILURE);
    }

    auto buffer = std::move(*bufferOfErr);

    auto objOrErr = llvm::object::ObjectFile::createObjectFile(buffer->getMemBufferRef());
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

    return { std::move(buffer), std::move(obj), elfObj };
}

static std::unordered_map<uint64_t, std::vector<llvm::object::RelocationRef>> buildRelocationMap(const std::unique_ptr<llvm::object::ObjectFile>& obj) {
    #ifdef LOG_OUTPUT
        std::cout << '\n';
    #endif

    std::unordered_map<uint64_t, std::vector<llvm::object::RelocationRef>> relocMap;
    size_t totalRelocations{};

    for(const auto& section: obj->sections()) {
        // skip over any section that isn't a relocation section
        auto targetOrErr = section.getRelocatedSection();
        if(!targetOrErr) {
            llvm::consumeError(targetOrErr.takeError());
            continue;
        }

        // map the targeted section's index to the list of relocations
        auto target = *targetOrErr;
        if(target == obj->section_end()) continue;

        uint64_t targetIndex = target->getIndex();
        for(const auto& relocation: section.relocations()) {
            relocMap[targetIndex].push_back(relocation);
            totalRelocations++;
        }
    }

    #ifdef LOG_OUTPUT
        std::cout << "[LOG] Built relocation map for " << relocMap.size() << " sections (total relocations: " << totalRelocations << ").\n";
    #endif

    return relocMap;
}

static std::unordered_map<std::string, Stencil> buildStencilLibrary(
    const std::unique_ptr<llvm::object::ObjectFile>& obj, 
    llvm::object::ELF64LEObjectFile* elfObj, 
    const std::unordered_set<std::string>& stencilSymbolNames,
    const std::unordered_map<uint64_t, std::vector<llvm::object::RelocationRef>>& relocationMap
) {
    #ifdef LOG_OUTPUT
        std::cout << '\n';
    #endif

    std::unordered_map<std::string, Stencil> stencils;

    // build Stencil for every stencil function
    for(const llvm::object::SymbolRef& symbol: obj->symbols()) {
        auto nameOrErr = symbol.getName();
        if(!nameOrErr) {
            llvm::consumeError(nameOrErr.takeError());
            continue;
        }

        std::string symbolName = nameOrErr->str();
        if(!stencilSymbolNames.count(symbolName)) continue;

        llvm::object::ELFSymbolRef elfSym(symbol);
        uint64_t symbolSize = elfSym.getSize();

        auto symbolAddressOrErr = symbol.getAddress();
        auto symbolSectionOrErr = symbol.getSection();

        if(!symbolAddressOrErr || !symbolSectionOrErr) {
            if(!symbolAddressOrErr) llvm::consumeError(symbolAddressOrErr.takeError());
            if(!symbolSectionOrErr) llvm::consumeError(symbolSectionOrErr.takeError());

            continue;
        }
        if(*symbolSectionOrErr == obj->section_end()) continue;

        uint64_t symbolAddress = *symbolAddressOrErr;
        auto symbolSectionIt = *symbolSectionOrErr;

        auto sectionContentsOrErr = symbolSectionIt->getContents();
        if(!sectionContentsOrErr) {
            llvm::consumeError(sectionContentsOrErr.takeError());
            continue;
        }

        // every stencil function should have at least one relocation registered (the continuation function)
        if(!relocationMap.count(symbolSectionIt->getIndex())) {
            std::cerr << "Failed to find relocations for symbol: " << symbolName << '\n';
            exit(EXIT_FAILURE);
        }

        llvm::StringRef sectionContents = *sectionContentsOrErr;
        uint64_t sectionAddress = symbolSectionIt->getAddress();

        // offset should be 0 and symbolSize should be the section size since every stencil function has its own section.
        ReleaseAssert(symbolAddress - sectionAddress == 0 && symbolSize == sectionContents.size());

        Stencil stencil;

        stencil.m_name = symbolName;

        // copy code from section into code buffer
        const uint8_t* bytesStart = reinterpret_cast<const uint8_t*>(sectionContents.data());
        const uint8_t* bytesEnd = bytesStart + symbolSize;

        stencil.m_code.assign(bytesStart, bytesEnd);

        const auto& relocations = relocationMap.at(symbolSectionIt->getIndex());

        stencil.m_relocations.reserve(relocations.size());

        for(const auto& relocation: relocations) {
            Relocation reloc;

            // since the function has the entire section to itself, the offset of the hole in the function == the secton offset
            reloc.m_offset = relocation.getOffset();
            reloc.m_elfRelocType = relocation.getType();

            // get symbol
            llvm::object::symbol_iterator targetSymIt = relocation.getSymbol();
            std::string targetSymbol;

            if(targetSymIt != obj->symbol_end()) {
                auto targetNameOrErr = targetSymIt->getName();
                if (targetNameOrErr) targetSymbol = targetNameOrErr->str();
                else llvm::consumeError(targetNameOrErr.takeError());
            }

            // match relocation kind and ordinal
            uint32_t ordinal;
            RelocationKind kind;

            if(IsMustTailPlaceholder(targetSymbol)) {
                ordinal = ExtractOrdinal(targetSymbol, MUSTTAIL_PREFIX);
                kind = RelocationKind::TailCall;
            } else if(IsNoTailPlaceholder(targetSymbol)) {
                ordinal = ExtractOrdinal(targetSymbol, NOTAIL_PREFIX);
                kind = RelocationKind::NonTailCall;
            } else if(IsDataPlaceholder(targetSymbol)) {
                ordinal = ExtractOrdinal(targetSymbol, DATA_PREFIX);
                kind = (relocation.getType() == llvm::ELF::R_X86_64_64) ? RelocationKind::U64Immediate : RelocationKind::NonTailCall;
            } else {
                std::cerr << "Failed to match symbol: " << targetSymbol << " in stencil function: " << symbolName << '\n';
                exit(EXIT_FAILURE);
            }

            reloc.m_ordinal = ordinal;
            reloc.m_kind = kind;

            llvm::object::DataRefImpl relocRef = relocation.getRawDataRefImpl();
            const auto* rela = elfObj->getRela(relocRef);
            reloc.m_addend = rela ? rela->r_addend : 0;

            // sanity checks
            if(kind == RelocationKind::TailCall || kind == RelocationKind::NonTailCall) {
                ReleaseAssert(reloc.m_elfRelocType == llvm::ELF::R_X86_64_PLT32 || reloc.m_elfRelocType == llvm::ELF::R_X86_64_PC32);
            }

            uint32_t patchWidth = (reloc.m_elfRelocType == llvm::ELF::R_X86_64_64) ? 8 : 4;
            ReleaseAssert(reloc.m_offset + patchWidth <= stencil.m_code.size());

            uint32_t width = (reloc.m_elfRelocType == llvm::ELF::R_X86_64_64) ? 8 : 4;
            for(uint32_t i = 0; i < width; i++) {
                ReleaseAssert(stencil.m_code[reloc.m_offset + i] == 0);
            }

            stencil.m_relocations.push_back(reloc);
        }

        stencils.insert({ symbolName, stencil });
    }

    #ifdef LOG_OUTPUT
        std::cout << "[LOG] Finished building stencil library.\n";
    #endif

    return stencils;
}

static void emitJsonManifest(const std::unordered_map<std::string, Stencil>& stencilLibrary, const std::unordered_map<std::string, std::vector<std::pair<std::string, uint64_t>>>& stencilMetaVarConfigs, std::string_view jsonFileDir) {
    #ifdef LOG_OUTPUT
        std::cout << '\n';
    #endif

    nlohmann::json manifest = nlohmann::json::object();

    for(const auto& [name, stencil] : stencilLibrary) {
        nlohmann::json entry;
        
        entry["code"] = stencil.m_code;

        nlohmann::json relocs = nlohmann::json::array();
        for(const auto& reloc : stencil.m_relocations) {
            nlohmann::json r;
            r["offset"] = reloc.m_offset;
            r["type"] = reloc.m_elfRelocType;
            r["kind"] = static_cast<uint32_t>(reloc.m_kind);
            r["ordinal"] = reloc.m_ordinal;
            r["addend"] = reloc.m_addend;
            relocs.push_back(std::move(r));
        }
        entry["relocations"] = std::move(relocs);

        for(const auto& [metaVarName, metaVarValue]: stencilMetaVarConfigs.at(name)) {
            entry[metaVarName] = metaVarValue;
        }

        manifest[name] = std::move(entry);
    }

    std::ofstream jsonFile{ std::string(jsonFileDir) };
    if(!jsonFile) {
        std::cerr << "Failed to open " << jsonFileDir << " for writing.\n";
        exit(EXIT_FAILURE);
    }

    jsonFile << manifest.dump(2);

    if(!jsonFile) {
        std::cerr << "Failed to write JSON manifest to " << jsonFileDir << '\n';
        exit(EXIT_FAILURE);
    }

    #ifdef LOG_OUTPUT
        std::cout << "[LOG] Emitted stencils JSON manifest to: '" << jsonFileDir << "'.\n";
    #endif
}