(module
  (func (export "tuple") (param i32 f64 i64 f32) (result f32 i32 f64 i64)
    (local.get 3)
    (local.get 0)
    (local.get 1)
    (local.get 2)))
