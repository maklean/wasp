#include <llvm/ExecutionEngine/Orc/LLJIT.h>

/*
    A workaround so when LLJIT needs to resolve all the stencil
    hole symbols which don't actually exist, it gets a trash address.
*/
void AddFakeSymbolResolverGenerator(llvm::orc::LLJIT* jit);