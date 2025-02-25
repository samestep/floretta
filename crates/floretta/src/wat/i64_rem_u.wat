(module
  (func (export "rem_u") (param i64 i64) (result i64)
    (i64.rem_u
      (local.get 0)
      (local.get 1))))
