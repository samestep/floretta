(module
  (func (export "xor") (param i64 i64) (result i64)
    (i64.xor
      (local.get 0)
      (local.get 1))))
