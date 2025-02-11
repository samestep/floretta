(module
  (func (export "loop") (param f64) (result f64)
    (f64.mul
      (local.get 0)
      (loop (result f64)
        (local.tee 0
          (f64.sub
            (local.get 0)
            (f64.const 1)))
        (br_if 0
          (f64.ge
            (local.get 0)
            (f64.const 0)))))))
