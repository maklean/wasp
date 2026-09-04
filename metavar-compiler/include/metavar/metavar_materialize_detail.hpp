#pragma once

#include "../utils/type_tpl_sequence.hpp"
#include "../utils/primitive_types.hpp"

/*
    NOTE: well, I can say I understand what's happening conceptually - other than that, whatever
    kind of C++ metaprogramming sorcery is happening here is a black box.
*/

namespace wasp::internal {

// Determines whether we should continue down this branch given a partial selection of metavar values.
template<typename T, typename... TArgs>
struct metavar_prefix_cond_fn_checker {
    template<auto... VArgs>
    struct impl3 {

        /*
            Performs member function existence.

            The first test() function gets picked if the cond() function (with the current
            number of arguments) exists.

            Otherwise, the second test() function gets picked.
        */
        struct has_cond {
            typedef char one;

            struct two { char x[2]; };

            template<typename C> static one test(decltype(&C::template cond<TArgs..., VArgs...>));
            template<typename C> static two test(...);

            #pragma clang diagnostic push
            #pragma clang diagnostic ignored "-Wzero-as-null-pointer-constant"
                constexpr static bool value = (sizeof(test<T>(0)) == sizeof(char));
            #pragma clang diagnostic pop
        };

        template<typename Dummy, typename Enable = void>
        struct impl4 {
            // in the case the 'cond' function doesn't exist, we treat it as a passing cond check
            struct impl5 : std::true_type {};
        };

        template<typename Dummy>
        struct impl4<Dummy, typename std::enable_if<(std::is_same<Dummy, void>::value && has_cond::value)>::type> {
            // 'cond' function exists, set the impl5 result to whatever 'cond' evaluates to using what we have
            struct impl5 : std::integral_constant<bool, (T::template cond<TArgs..., VArgs...>())> {};
        };
    };

    // to get the value easily
    template<auto... VArgs>
    using impl = typename impl3<VArgs...>::template impl4<void>::impl5;
};

// Gets the function address of the StencilGenerator::f() function given a valid sequence of type and value arguments.
template<typename C>
struct metavar_get_user_fn_helper;

template<typename... TSeq>
struct metavar_get_user_fn_helper<type_tpl_sequence<TSeq...>> {
    template<typename Materializer, typename... TArgs>
    struct impl {
        template<auto... VArgs>
        struct impl2 {
            static void* get() {
                return reinterpret_cast<void *>(Materializer::template f<TArgs..., VArgs..., TSeq...>);
            }
        };
    };
};

// Generate mappings of: primitive type => enumerator
#define F(T) , ENUM_##T

// so we can represent primitive types as integrals, allowing them to be represented as metavar values in MetaVarMaterializedInstance::m_values.
enum class MVTypeIdLabelHelper {
    ENUM_void
    FOR_EACH_PRIMITIVE_TYPE
};

#undef F

// constexpr-if branch static_assert(false, ...) workaround
template<class T> struct type_dependent_false : std::false_type {};

}; // namespace wasp::internal