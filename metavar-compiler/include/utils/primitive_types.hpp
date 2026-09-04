#pragma once

// Calls F(T) for each T in {i32, i64, f32, f64} - need u32 and u64 for operations that depend on wrapping (e.g., add)
#define FOR_EACH_PRIMITIVE_TYPE \
    F(uint32_t) \
    F(uint64_t) \
    F(int32_t) \
    F(int64_t) \
    F(float) \
    F(double)