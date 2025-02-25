(module
  (func (export "add") (param f32 f32) (result f32)
    (f32.add
      (local.get 0)
      (local.get 1))))
