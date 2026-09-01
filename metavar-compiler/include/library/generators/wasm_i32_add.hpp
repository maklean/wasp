#pragma once

#include "common.hpp"

namespace wasp {

// T.const
struct WasmI32Add {
    template<typename OperandType>
    static constexpr bool cond() {
        // add is basically a wrapping add, so we use unsigned ints to get the same behavior
        return std::is_same<OperandType, uint32_t>::value || std::is_same<OperandType, uint64_t>::value;
    }

    template<typename OperandType, 
            NumOpaqueIntegralParams numIntegralParams, 
            NumOpaqueFloatingParams numFloatingParams, 
            NumInRegisterOperands numInRegisterOperand>
    static constexpr bool cond() {
        if(OpaqueParamsHelper::CanPush(numFloatingParams)) return false;

        if(numInRegisterOperand != NumInRegisterOperands::TWO) {
            return OpaqueParamsHelper::IsEmpty(numIntegralParams);
        }

        return OpaqueParamsHelper::CanPush(numIntegralParams, 2);
    }

    template<typename OperandType, 
            NumOpaqueIntegralParams numIntegralParams, 
            NumOpaqueFloatingParams numFloatingParams, 
            NumInRegisterOperands numInRegisterOperand,
            bool spillOutput>
    static constexpr bool cond() {
        return true;
    }

    template<typename OperandType, 
            NumOpaqueIntegralParams numIntegralParams, 
            NumOpaqueFloatingParams numFloatingParams, 
            NumInRegisterOperands numInRegisterOperand,
            bool spillOutput,
            typename... OpaqueParams>
    static void f(uintptr_t stack, OpaqueParams... opaqueParams, [[maybe_unused]] OperandType qa1, [[maybe_unused]] OperandType qa2) noexcept {
        using SMA = StackMachineAccessor<OperandType, 2 - static_cast<int>(numInRegisterOperand), OperandType>;

        OperandType lhs, rhs;
        
        if constexpr(numInRegisterOperand == NumInRegisterOperands::ZERO) {
            lhs = SMA::template GetInput<1>(stack);
            rhs = SMA::template GetInput<0>(stack);
        } else if constexpr(numInRegisterOperand == NumInRegisterOperands::ONE) {
            lhs = SMA::template GetInput<0>(stack);
            rhs = qa1;
        } else {
            static_assert(numInRegisterOperand == NumInRegisterOperands::TWO);

            lhs = qa1;
            rhs = qa2;
        }

        OperandType result = lhs + rhs;

        if constexpr(spillOutput) {
            *SMA::GetOutputLoc(stack) = result;

            DEFINE_BOILERPLATE_FNPTR_PLACEHOLDER_0(void(*)(uintptr_t, OpaqueParams...) noexcept);
            BOILERPLATE_FNPTR_PLACEHOLDER_0(stack, opaqueParams...);
        } else {
            DEFINE_BOILERPLATE_FNPTR_PLACEHOLDER_0(void(*)(uintptr_t, OpaqueParams... opaqueParams, OperandType) noexcept);
            BOILERPLATE_FNPTR_PLACEHOLDER_0(stack, opaqueParams..., result);
        }
    }

    static auto metavars() {
        return CreateMetaVarList(
            CreatePrimitiveTypeMetaVar("operandType"),
            CreateOpaqueIntegralParamsLimit(),
            CreateOpaqueFloatingParamsLimit(),
            CreateEnumMetaVar<NumInRegisterOperands::X_END_OF_ENUM>("numInRegisterOperand"),
            CreateBoolMetaVar("spillOutput")
        );
    }
};

}; // namespace wasp