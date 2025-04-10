(module
  (memory 1)
  (func (export "roundtrip") (param f32) (result f32)
    (f32.store
      (i32.const 0)
      (local.get 0))
    (f32.load
      (i32.const 0))))
