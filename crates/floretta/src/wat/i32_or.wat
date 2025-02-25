(module
  (func (export "or") (param i32 i32) (result i32)
    (i32.or
      (local.get 0)
      (local.get 1))))
