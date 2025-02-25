(module
  (func (export "clobber") (param f32 f32) (result f32 f32)
    (local.set 0
      (local.get 1))
    (local.get 1)
    (local.get 0)))
