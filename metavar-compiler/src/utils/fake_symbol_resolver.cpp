#include "../../include/utils/fake_symbol_resolver.hpp"

#include <llvm/ExecutionEngine/Orc/LLJIT.h>

#include <sys/mman.h>
#include <mutex>
#include <vector>
#include <cstdint>
#include <cassert>
#include <memory>

/*
    this is straight-up copy and pasted from: https://github.com/sillycross/WasmNow/blob/main/runtime_lib_builder/fake_symbol_resolver.cpp

    (with some updates for LLVM 19+)
*/

namespace {

class FakeSymbolResolver : public llvm::orc::DefinitionGenerator {
public:
    FakeSymbolResolver()
        : m_lock(), m_curAddr(0), m_addrEnd(0), m_pastAddrs() {}

    ~FakeSymbolResolver() override {
        for (uintptr_t addr : m_pastAddrs) {
            munmap(reinterpret_cast<void*>(addr), x_length + x_overflow_protection_buffer * 2);
        }
    }

    llvm::Error tryToGenerate(
        llvm::orc::LookupState &LS,
        llvm::orc::LookupKind K,
        llvm::orc::JITDylib &JD,
        llvm::orc::JITDylibLookupFlags JDLookupFlags,
        const llvm::orc::SymbolLookupSet &symbols) override 
    {
        std::lock_guard<std::mutex> guard(m_lock);
        llvm::orc::SymbolMap newSymbols;

        for (auto& kv : symbols) {
            const llvm::orc::SymbolStringPtr& name = kv.first;
            if ((*name).empty()) continue;

            uintptr_t addr = GetNextAddress();
            newSymbols[name] = llvm::orc::ExecutorSymbolDef(
                llvm::orc::ExecutorAddr(static_cast<uint64_t>(addr)),
                llvm::JITSymbolFlags::Exported
            );
        }

        if (newSymbols.empty()) {
            return llvm::Error::success();
        }

        return JD.define(llvm::orc::absoluteSymbols(std::move(newSymbols)));
    }

private:
    uintptr_t GetNextAddress() {
        if (m_curAddr == 0 || m_curAddr >= m_addrEnd) {
            void* r = mmap(nullptr, x_length + x_overflow_protection_buffer * 2,
               PROT_READ | PROT_EXEC, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
            assert(r != MAP_FAILED);
            m_curAddr = reinterpret_cast<uintptr_t>(r) + x_overflow_protection_buffer;
            m_addrEnd = m_curAddr + x_length;
            m_pastAddrs.push_back(reinterpret_cast<uintptr_t>(r));
        }

        uintptr_t r = m_curAddr;
        m_curAddr += 32;
        return r;
    }

    static constexpr size_t x_length = 1024 * 1024;
    static constexpr size_t x_overflow_protection_buffer = 1024 * 1024;

    std::mutex m_lock;
    uintptr_t m_curAddr;
    uintptr_t m_addrEnd;
    std::vector<uintptr_t> m_pastAddrs;
};

} // anonymous namespace

void AddFakeSymbolResolverGenerator(llvm::orc::LLJIT* jit) {
    jit->getMainJITDylib().addGenerator(
        std::make_unique<FakeSymbolResolver>()
    );
}