(module
  (func (export "select") (param i32 f64 f64) (result f64)
    local.get 0
    if (result f64)
      local.get 1
    else
      local.get 2
    end))
