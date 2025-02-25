(module
  (func (export "shr_s") (param i32 i32) (result i32)
    (i32.shr_s
      (local.get 0)
      (local.get 1))))
