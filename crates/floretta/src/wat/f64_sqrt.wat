(module
  (func (export "sqrt") (param f64) (result f64)
    (f64.sqrt
      (local.get 0))))
