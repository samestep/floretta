(module
  (func (export "shl") (param i64 i64) (result i64)
    (i64.shl
      (local.get 0)
      (local.get 1))))
