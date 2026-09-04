#pragma once

#include "common.hpp"

namespace wasp {

// T.const
struct WasmConstant {
    template<typename OperandType>
    static constexpr bool cond() {
        // has to be one of {i32, i64, f32, f64}
        return std::is_same<OperandType, int32_t>::value ||
               std::is_same<OperandType, int64_t>::value ||
               std::is_same<OperandType, float>::value ||
               std::is_same<OperandType, double>::value;
    }

    template<typename OperandType, NumOpaqueIntegralParams numIntegralParams, NumOpaqueFloatingParams numFloatingParams, bool spillOutput>
    static constexpr bool cond() {
        if(std::is_floating_point<OperandType>::value) {
            if(OpaqueParamsHelper::CanPush(numIntegralParams)) return false;
            if(spillOutput && !OpaqueParamsHelper::IsEmpty(numFloatingParams)) return false;
        } else {
            if(OpaqueParamsHelper::CanPush(numFloatingParams)) return false;
            if(spillOutput && !OpaqueParamsHelper::IsEmpty(numIntegralParams)) return false;
        }

        return true;
    }

    template<typename OperandType, 
            NumOpaqueIntegralParams numIntegralParams, 
            NumOpaqueFloatingParams numFloatingParams, 
            bool spillOutput, 
            typename... OpaqueParams>
    static void f(uintptr_t stack, OpaqueParams... opaqueParams) noexcept {
        using SMA = StackMachineAccessor<OperandType, 0, OperandType>;

        DEFINE_CONSTANT_PLACEHOLDER_0(OperandType);
        OperandType value = CONSTANT_PLACEHOLDER_0;

        if constexpr(spillOutput) {
            *SMA::GetOutputLoc(stack) = value;

            DEFINE_BOILERPLATE_FNPTR_PLACEHOLDER_0(void(*)(uintptr_t, OpaqueParams...) noexcept);
            BOILERPLATE_FNPTR_PLACEHOLDER_0(stack, opaqueParams...);
        } else {
            DEFINE_BOILERPLATE_FNPTR_PLACEHOLDER_0(void(*)(uintptr_t, OpaqueParams..., OperandType) noexcept);
            BOILERPLATE_FNPTR_PLACEHOLDER_0(stack, opaqueParams..., value);
        }
    }

    static auto metavars() {
        return CreateMetaVarList(
            CreatePrimitiveTypeMetaVar("operandType"),
            CreateOpaqueIntegralParamsLimit(),
            CreateOpaqueFloatingParamsLimit(),
            CreateBoolMetaVar("spillOutput")
        );
    }
};

}; // namespace wasp