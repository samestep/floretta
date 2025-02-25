(module
  (func (export "shl") (param i32 i32) (result i32)
    (i32.shl
      (local.get 0)
      (local.get 1))))
