(module
  (func (export "convert") (param i64) (result f64)
    (f64.convert_i64_s
      (local.get 0))))
