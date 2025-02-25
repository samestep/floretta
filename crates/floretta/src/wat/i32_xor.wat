(module
  (func (export "xor") (param i32 i32) (result i32)
    (i32.xor
      (local.get 0)
      (local.get 1))))
