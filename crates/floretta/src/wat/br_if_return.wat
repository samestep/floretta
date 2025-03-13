(module
  (func (export "select") (param i32 f64 f64) (result f64)
    local.get 1
    local.get 0
    br_if 0
    drop
    local.get 2))
