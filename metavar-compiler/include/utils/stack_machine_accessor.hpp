#pragma once

#include <cstdint>
#include <type_traits>

#include "stencil_placeholders.hpp"
#include "function_attributes.hpp"

namespace wasp {

#define INT_TOP 0

#define INT_PUSH 8
#define INT_2ND_TOP 9
#define INT_3RD_TOP 10

#define FLOAT_TOP 1
#define FLOAT_PUSH 11
#define FLOAT_2ND_TOP 12

#define TOKEN_PASTEx(x, y) x ## y
#define TOKEN_PASTE(x, y) TOKEN_PASTEx(x, y)

#define DEF_DATA(x) INTERNAL_DEFINE_INDEX_CONSTANT_PLACEHOLDER(x)
#define GET_DATA(x) TOKEN_PASTE(CONSTANT_PLACEHOLDER_, x)

// Gets the address of something sitting on the stack, or smth like that idrk.
// Always has to be inlined so a call instruction isn't emitted.
template<typename LocalVarType>
inline LocalVarType* ALWAYS_INLINE GetLocalVarAddress(uintptr_t stackFrame, uint64_t offset) noexcept {
    return reinterpret_cast<LocalVarType*>(stackFrame + offset);
}

namespace internal {
    // Gets the item at the top of the stack.
    template<typename T>
    T* ALWAYS_INLINE GetStackTop(uintptr_t stackFrame) {
        if constexpr(!std::is_floating_point<T>::value) {
            DEF_DATA(INT_TOP);
            return GetLocalVarAddress<T>(stackFrame, GET_DATA(INT_TOP));
        } else {
            DEF_DATA(FLOAT_TOP);
            return GetLocalVarAddress<T>(stackFrame, GET_DATA(FLOAT_TOP));
        }
    }

    template<typename T>
    T* ALWAYS_INLINE GetStack2ndTop(uintptr_t stackFrame) {
        if constexpr(!std::is_floating_point<T>::value) {
            DEF_DATA(INT_2ND_TOP);
            return GetLocalVarAddress<T>(stackFrame, GET_DATA(INT_2ND_TOP));
        } else {
            DEF_DATA(FLOAT_2ND_TOP);
            return GetLocalVarAddress<T>(stackFrame, GET_DATA(FLOAT_2ND_TOP));
        }
    }

    template<typename T>
    T* ALWAYS_INLINE GetStack3rdTop(uintptr_t stackFrame) {
        static_assert(!std::is_floating_point<T>::value);
        DEF_DATA(INT_3RD_TOP);
        return GetLocalVarAddress<T>(stackFrame, GET_DATA(INT_3RD_TOP));
    }

    // gets the location where to push
    template<typename T>
    T* ALWAYS_INLINE GetStackPush(uintptr_t stackFrame) {
        if constexpr(!std::is_floating_point<T>::value) {
            DEF_DATA(INT_PUSH);
            return GetLocalVarAddress<T>(stackFrame, GET_DATA(INT_PUSH));
        } else {
            DEF_DATA(FLOAT_PUSH);
            return GetLocalVarAddress<T>(stackFrame, GET_DATA(FLOAT_PUSH));
        }
    }
}; // internal

// Takes several inputs of the same type, produces zero or one output
template<typename InputType, int numInputOnStack, typename OutputType>
struct StackMachineAccessor {
    static_assert(numInputOnStack >= 0 && numInputOnStack <= 2); // only accept [0, 2] on the stack

    // Gets the input at the given position on the stack (0 is the top).
    template<int inputOrd>
    static InputType ALWAYS_INLINE GetInput(uintptr_t stackFrame) {
        static_assert(inputOrd >= 0 && inputOrd < numInputOnStack);

        if constexpr(inputOrd == 0) {
            return *internal::GetStackTop<InputType>(stackFrame);
        } else {
            static_assert(inputOrd == 1);
            return *internal::GetStack2ndTop<InputType>(stackFrame);
        }
    }

    // Gets the output location of the given position on the stack.
    static OutputType* ALWAYS_INLINE GetOutputLoc(uintptr_t stackFrame) {
        static_assert(!std::is_same<OutputType, void>::value);

        if constexpr(std::is_floating_point<InputType>::value != std::is_floating_point<OutputType>::value || numInputOnStack == 0) {
            return internal::GetStackPush<OutputType>(stackFrame);
        } else if constexpr(numInputOnStack == 1) {
            return internal::GetStackTop<OutputType>(stackFrame);
        } else {
            static_assert(numInputOnStack == 2);
            return internal::GetStack2ndTop<OutputType>(stackFrame);
        }
    }
};

#undef INT_TOP
#undef INT_PUSH
#undef INT_2ND_TOP
#undef INT_3RD_TOP

#undef FLOAT_TOP
#undef FLOAT_PUSH
#undef FLOAT_2ND_TOP

#undef DEF_DATA
#undef GET_DATA

}; // wasp

