(module
  (func (export "mul") (param f32 f32) (result f32)
    (f32.mul (local.get 0) (local.get 1))))
