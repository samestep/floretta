(module
  (func (export "select") (param i32 f64 f64) (result f64)
    f64.const 42
    local.get 1
    local.get 0
    br_if 0
    drop
    drop
    local.get 2))
