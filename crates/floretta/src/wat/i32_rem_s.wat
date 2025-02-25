(module
  (func (export "rem_s") (param i32 i32) (result i32)
    (i32.rem_s
      (local.get 0)
      (local.get 1))))
