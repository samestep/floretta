(module
  (func (export "shr_u") (param i64 i64) (result i64)
    (i64.shr_u
      (local.get 0)
      (local.get 1))))
