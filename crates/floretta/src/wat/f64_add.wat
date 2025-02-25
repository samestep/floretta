(module
  (func (export "add") (param f64 f64) (result f64)
    (f64.add
      (local.get 0)
      (local.get 1))))
