(module
  (import "env" "random_i32" (func $random_i32 (param i32 i32) (result i32)))
  (import "env" "print_i32" (func $print_i32 (param i32)))

  (func (export "add_random_i32")
    (local $first i32) (local $second i32)

    ;; generate two random values in [1, 100]
    (local.tee $first (call $random_i32 (i32.const 1) (i32.const 100)))
    (local.tee $second (call $random_i32 (i32.const 1) (i32.const 100)))

    ;; print the values, then add them and print the result
    (call $print_i32 (local.get $first))
    (call $print_i32 (local.get $second))

    i32.add
    call $print_i32))