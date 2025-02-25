(module
  (func (export "min") (param f64 f64) (result f64)
    (f64.min
      (local.get 0)
      (local.get 1))))
