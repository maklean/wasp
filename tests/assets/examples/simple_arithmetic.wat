(module
  (func $add (export "add") (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.add)

  (func $fact (export "fact") (param $x i64) (result i64)
    ;; if $x is less than 0, trap.
    (if (i64.lt_s (local.get $x) (i64.const 0))
      (then unreachable))
    
    (if (result i64) (i64.le_s (local.get $x) (i64.const 1))
      (then (i64.const 1))
      (else
        (i64.mul
          (local.get $x)
          (call $fact (i64.sub (local.get $x) (i64.const 1))))))))