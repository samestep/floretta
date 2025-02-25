(module
  (func (export "shr_s") (param i64 i64) (result i64)
    (i64.shr_s
      (local.get 0)
      (local.get 1))))
