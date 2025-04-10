(module
  (func $tuple (param i32 f64 i64 f32) (result f32 i32 f64 i64)
    (local.get 3)
    (local.get 0)
    (local.get 1)
    (local.get 2))
  (func (export "tuple") (param f64 i64 f32 i32) (result f32 i32 f64 i64)
    (call $tuple
      (local.get 3)
      (local.get 0)
      (local.get 1)
      (local.get 2))))
