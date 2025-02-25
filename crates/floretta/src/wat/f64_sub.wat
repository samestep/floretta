(module
  (func (export "sub") (param f64 f64) (result f64)
    (f64.sub
      (local.get 0)
      (local.get 1))))
