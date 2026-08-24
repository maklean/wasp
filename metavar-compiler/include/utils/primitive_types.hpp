#pragma once

// Calls F(T) for each T in {i32, i64, f32, f64}
#define FOR_EACH_PRIMITIVE_TYPE \
    F(int32_t) \
    F(int64_t) \
    F(float) \
    F(double)