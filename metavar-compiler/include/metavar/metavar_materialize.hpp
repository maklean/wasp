#pragma once

#include "metavar_materialize_detail.hpp"
#include "metavar_result.hpp"
#include "typed_metavar.hpp"
#include "../utils/function_attributes.hpp"
#include "../utils/asserts.hpp"

#include <type_traits>

namespace wasp::internal {

/*
    Base case for the template recursion - we've traversed all variants of each metavar for the stencil generator.

    Check the value of the StencilGenerator::cond() function:
        - do nothing if the value is false;
        - push the function pointer for the generated stencil into the MetaVarMaterializedList.
*/
template<typename Materializer, auto... metaVarTypes>
struct metavar_materialize_helper {

    template<int numIntegralParams, int numFloatingParams, auto... remainingMetaVarTypes>
    struct impl {
        static_assert(sizeof...(remainingMetaVarTypes) == 0, "The number of remaining metavars in the base case should be 0.");

        // TArgs = accumulated type arguments (from the PRIMITIVE_TYPE metavars), VArgs = accumulated values (from the BOOL/ENUM metavars)
        template<typename... TArgs>
        struct impl2 {

            template<auto... VArgs>
            struct impl3 {

                // Matches the invalid stencil configurations (the ones that fail cond())
                template<typename Dummy, typename Enable = void>
                struct impl4 {
                    static void invoke(MetaVarMaterializedList* result, const PartialMetaVarValueInstance& instance) {}
                };

                // Matches the valid stencil configurations (the ones that pass cond())
                template<typename Dummy>
                struct impl4<Dummy, typename std::enable_if<(
                    std::is_same<Dummy, void>::value &&
                    std::integral_constant<bool, (Materializer::template cond<TArgs..., VArgs...>())>::value)>::type>
                {
                    // The compiler shouldn't optimize nor inline this function for performance reasons: https://github.com/sillycross/WasmNow/blob/main/fastinterp/metavar.hpp#L343
                    static void NO_INLINE NO_OPTS invoke(MetaVarMaterializedList* result, const PartialMetaVarInstance& instance) {
                        using UserFnGetter = typename metavar_get_user_fn_helper<type_tpl_sequence_t<uint64_t, numIntegralParams, double, numFloatingParams>>
                            ::template impl<Materializer, TArgs...>::template impl2<VArgs...>;
                        
                        // metavar values list should be parallel to the stencil generator's metavar list
                        ReleaseAssert(instance.value.size() == result->m_metavars.size());

                        MetaVarMaterializedInstance inst;

                        inst.m_values = instance.m_values;
                        inst.m_fnPtr = UserFnGetter::get();

                        result->m_instances.push_back(inst);
                    }
                };
            };
        };
    };

    // Appends all stencil functions for the given StencilGenerator/Materializer into the given MetaVarMaterializedList
    static void materialize(MetaVarMaterializedList* result) {
        PartialMetaVarInstance inst;

        impl<0 /* numIntegralParams*/, 0 /* numFloatingParams*/, metaVarTypes...>::template
            impl2<>::template impl3<>::template impl4<void>::invoke(result, inst);
    }
};

/*
    Recursive case for the template recursion (i.e., when there's at least one metavar remaining) - we take one of the 
    metavar types out: 'currMetaVarType', from the metavar type sequence: 'remainingMetaVarType', and iterate over its variants.
*/
template<typename Materializer, auto... metaVarTypes>
template<int numIntegralParams, int numFloatingParams, auto currMetaVarType, auto... remainingMetaVarTypes>
struct metavar_materialize_helper<Materializer, metaVarTypes...>::impl<numIntegralParams, numFloatingParams, currMetaVarType, remainingMetaVarTypes...> {
    
    // TArgs = accumulated type arguments (from the PRIMITIVE_TYPE metavars), VArgs = accumulated values (from the BOOL/ENUM metavars)
    template<typename... TArgs>
    struct impl2 {
        template<auto... VArgs>
        struct impl3 {

            // cond() function doesn't exist, or cond() returned true from the early-exit check. Iterate over variants based on metavar type.
            template<typename Dummy, typename Enable = void>
            struct impl4 {
                // Shouldn't be optimized or inlined due to being compile-time sensitive: https://github.com/sillycross/WasmNow/blob/main/fastinterp/metavar.hpp#L438
                static void NO_INLINE NO_OPTS invoke(MetaVarMaterializedList* result, const PartialMetaVarInstance& instance) {
                    using T = decltype(currMetaVarType);

                    if constexpr(std::is_same<T, MetaVarType>::value) {
                        if constexpr(currMetaVarType == MetaVarType::BOOL) {
                            // iterate over { false, true }
                            impl<numIntegralParams, numFloatingParams, remainingMetaVarTypes...>::template impl2<TArgs...>::template impl3<VArgs..., false>::template impl4<void>::invoke(result, instance.Push(false));
                            impl<numIntegralParams, numFloatingParams, remainingMetaVarTypes...>::template impl2<TArgs...>::template impl3<VArgs..., true>::template impl4<void>::invoke(result, instance.Push(true));
                        } else if constexpr(currMetaVarType == MetaVarType::WASM_PRIMITIVE_TYPE) {
                            // iterate over { i32, i64, f32, f64, void }
                            // TODO: we should be able to remove the iterations over pointer-to-primitive and void**s
                            constexpr uint64_t x_typeid_pointer_typeid_inc = 1000000000;

                            // iterate over non-pointer types
                            #define F(T) \
                                impl<numIntegralParams, numFloatingParams, remainingMetaVarTypes...>::template impl2<TArgs..., T>::template impl3<VArgs...>::template impl4<void>::invoke ( \
                                    result, instance.Push(static_cast<uint64_t>(MVTypeIdLabelHelper::ENUM_##T)) \
                                );
                            
                            F(void)
                            FOR_EACH_PRIMITIVE_TYPE

                            #undef F

                            // iterate over pointer types
                            #define F(T) \
                                impl<numIntegralParams, numFloatingParams, remainingMetaVarTypes...>::template impl2<TArgs..., T*>::template impl3<VArgs...>::template impl4<void>::invoke( \
                                    result, instance.Push(static_cast<uint64_t>(MVTypeIdLabelHelper::ENUM_##T) + x_typeid_pointer_typeid_inc));
                            
                            F(void)
                            FOR_EACH_PRIMITIVE_TYPE

                            impl<numIntegralParams, numFloatingParams, remainingMetaVarTypes...>::template impl2<TArgs..., void**>::template impl3<VArgs...>::template impl4<void>::invoke( \
                                    result, instance.Push(static_cast<uint64_t>(MVTypeIdLabelHelper::ENUM_void) + 2 * x_typeid_pointer_typeid_inc));

                            #undef F
                        } else {
                            static_assert(type_dependent_false<Materializer>::value, "Unexpected MetaVarType");
                        }
                    } else {
                        // it's an enum - assume currMetaVarType is the upperbound we're supposed to iterate up to
                        static_assert(std::is_enum<T>::value, "Expected enum.");

                        constexpr int ub = static_cast<int>(currMetaVarType);
                        static_assert(ub > 0, "X_END_OF_ENUM exclusive upperbound should be greater than 0.");

                        invoke_enum<T, 0, ub>(result, instance);
                    }
                }

                /*
                    The compiler should optimize this function. This should result in a lower number
                    of symbols to resolve, and a faster build-time-JIT (~3x faster).

                    We use binary recursion here. This is because a simple loop, i.e., for(int i = 0; i < ub, i++)
                    creates a deep template instantiation depth. If we do that across thousands of stencil generators,
                    we blow up compile-time/stack which could result in a crash.

                    Binary splitting (i.e., using `mid = (lb+ub)/2`) makes the depth `O(log ub)`
                */
                template<typename T, int lb, int ub>
                static void invoke_enum(MetaVarMaterializedList* result, const PartialMetaVarInstance& instance) {
                    if constexpr(lb + 1 == ub) {
                        // at the last element
                        constexpr T value = static_cast<T>(lb);

                        if constexpr(std::is_same<T, NumOpaqueIntegralParams>::value) {
                            // set number of integral params
                            impl<lb, numFloatingParams, remainingMetaVarTypes...>::template impl2<TArgs...>::template impl3<VArgs..., value>::template impl4<void>::invoke(
                                result, instance.Push(static_cast<uint64_t>(lb))
                            );
                        } else if constexpr(std::is_same<T, NumOpaqueFloatingParams>::value) {
                            // set number of floating params
                            impl<numIntegralParams, lb, remainingMetaVarTypes...>::template impl2<TArgs...>::template impl3<VArgs..., value>::template impl4<void>::invoke(
                                result, instance.Push(static_cast<uint64_t>(lb))
                            );
                        } else {
                            // add enum variant to values
                            impl<numIntegralParams, numFloatingParams, remainingMetaVarTypes...>::template impl2<TArgs...>::template impl3<VArgs..., value>::template impl4<void>::invoke(
                                result, instance.Push(static_cast<uint64_t>(lb))
                            );
                        }
                    } else {
                        // recurse into other branches
                        constexpr int mid = (lb + ub) / 2;

                        invoke_enum<T, lb, mid>(result, instance);
                        invoke_enum<T, mid, ub>(result, instance);
                    }
                }
            };

            // cond() exists and failed early - do nothing
            template<typename Dummy>
            struct impl4<Dummy, typename std::enable_if<(
                std::is_same<Dummy, void>::value &&
                !metavar_prefix_cond_fn_checker<Materializer, TArgs...>::template impl<VArgs...>::value)>::type
            >
            {
                static void invoke(MetaVarMaterializedList* result, const PartialMetaVarValueInstance& instance) {}
            };
        };
    };
};

}; // namespace wasp::internal