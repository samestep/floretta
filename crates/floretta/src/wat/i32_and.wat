(module
  (func (export "and") (param i32 i32) (result i32)
    (i32.and
      (local.get 0)
      (local.get 1))))
