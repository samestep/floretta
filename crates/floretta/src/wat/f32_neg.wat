(module
  (func (export "neg") (param f32) (result f32)
    (f32.neg
      (local.get 0))))
