(module
  (func (export "rem_s") (param i64 i64) (result i64)
    (i64.rem_s
      (local.get 0)
      (local.get 1))))
