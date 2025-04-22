(module
  (func (export "convert") (param i32) (result f64)
    (f64.convert_i32_s
      (local.get 0))))
