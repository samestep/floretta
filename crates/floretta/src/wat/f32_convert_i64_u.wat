(module
  (func (export "convert") (param i64) (result f32)
    (f32.convert_i64_u
      (local.get 0))))
