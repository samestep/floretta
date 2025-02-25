(module
  (func (export "max") (param f32 f32) (result f32)
    (f32.max
      (local.get 0)
      (local.get 1))))
