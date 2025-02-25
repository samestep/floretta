(module
  (func (export "clobber") (param f64 f64) (result f64 f64)
    (local.set 0
      (local.get 1))
    (local.get 1)
    (local.get 0)))
