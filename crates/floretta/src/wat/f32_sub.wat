(module
  (func (export "sub") (param f32 f32) (result f32)
    (f32.sub
      (local.get 0)
      (local.get 1))))
