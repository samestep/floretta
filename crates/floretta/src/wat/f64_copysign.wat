(module
  (func (export "copysign") (param f64 f64) (result f64)
    (f64.copysign
      (local.get 0)
      (local.get 1))))
