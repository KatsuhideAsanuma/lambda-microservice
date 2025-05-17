(module
  (func $run (export "run") (param i32 i32) (result i32 i32)
    ;; Return the input parameters as is
    local.get 0
    local.get 1
  )
  (memory (export "memory") 1)
)
