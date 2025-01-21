(module
  (func (export "div") (param f32 f32) (result f32)
    (f32.div (local.get 0) (local.get 1))))
