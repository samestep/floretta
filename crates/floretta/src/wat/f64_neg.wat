(module
  (func (export "neg") (param f64) (result f64)
    (f64.neg
      (local.get 0))))
