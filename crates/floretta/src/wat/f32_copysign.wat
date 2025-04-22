(module
  (func (export "copysign") (param f32 f32) (result f32)
    (f32.copysign
      (local.get 0)
      (local.get 1))))
