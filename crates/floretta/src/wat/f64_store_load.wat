(module
  (memory 1)
  (func (export "roundtrip") (param f64) (result f64)
    (f64.store
      (i32.const 0)
      (local.get 0))
    (f64.load
      (i32.const 0))))
