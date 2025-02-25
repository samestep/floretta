(module
  (func (export "max") (param f64 f64) (result f64)
    (f64.max
      (local.get 0)
      (local.get 1))))
