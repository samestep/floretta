(module
  (func (export "min") (param f32 f32) (result f32)
    (f32.min
      (local.get 0)
      (local.get 1))))
