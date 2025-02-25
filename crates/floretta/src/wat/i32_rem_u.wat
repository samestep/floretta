(module
  (func (export "rem_u") (param i32 i32) (result i32)
    (i32.rem_u
      (local.get 0)
      (local.get 1))))
