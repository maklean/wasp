#pragma once

namespace wasp::internal {

// Holds a list of types.
template<typename...>
struct type_tpl_sequence {};

// Recursive case for gen_type_tpl_sequence<T, N, S...> - generate gen_type_sequence(T, N-1, T, S...), i.e, add a new T to the sequence, go to N-1
template<typename T, int N, typename... S>
struct gen_type_tpl_sequence : gen_type_tpl_sequence<T, N-1, T, S...> {};

// Base case for gen_type_tpl_sequence - generate a type_tpl_sequence from the accumulated type sequence.
template<typename T, typename... S>
struct gen_type_tpl_sequence<T, 0, S...> {
    using type = type_tpl_sequence<S...>;
};

// prepend N copies of T to an existing type sequence 'C'.
template<typename T, int N, typename C>
struct append_type_tpl_sequence_front;

template<typename T, int N, typename... TS>
struct append_type_tpl_sequence_front<T, N, type_tpl_sequence<TS...>> {
    using type = typename gen_type_tpl_sequence<T, N, TS...>::type;
};

// build N2 copies of T2, then prepend N1 copies of T1
template<typename T1, int N1, typename T2, int N2>
using type_tpl_sequence_t = typename append_type_tpl_sequence_front<T1, N1, typename gen_type_tpl_sequence<T2, N2>::type>::type;

}; // namespace wasp::internal