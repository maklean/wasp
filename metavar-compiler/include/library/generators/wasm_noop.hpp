#pragma once

#include "common.hpp"

namespace wasp {

struct WasmNoop {
    template<NumOpaqueIntegralParams numIntegralParams, NumOpaqueFloatingParams numFloatingParams>
    static constexpr bool cond() {
        // The only stencil generated for Noop should be full of passthroughs/OpaqueParams.
        return !OpaqueParamsHelper::CanPush(numIntegralParams) && !OpaqueParamsHelper::CanPush(numFloatingParams);
    }

    template<NumOpaqueIntegralParams numIntegralParams, NumOpaqueFloatingParams numFloatingParams, typename... OpaqueParams>
    static void f(uintptr_t stack, OpaqueParams... opaqueParams) noexcept {
        // pass passthroughs to the next stencil
        DEFINE_BOILERPLATE_FNPTR_PLACEHOLDER_0(void(*)(uintptr_t, OpaqueParams...) noexcept);
        BOILERPLATE_FNPTR_PLACEHOLDER_0(stack, opaqueParams...);
    }

    static auto metavars() {
        return CreateMetaVarList(
            CreateOpaqueIntegralParamsLimit(),
            CreateOpaqueFloatingParamsLimit()
        );
    }
};

}; // namespace wasp