(module
  (func (export "convert") (param i32) (result f32)
    (f32.convert_i32_s
      (local.get 0))))
