(module
  (func (export "select") (param i32 f64 f64) (result f64)
    local.get 0
    if
      local.get 1
      local.set 2
    end
    local.get 2))
