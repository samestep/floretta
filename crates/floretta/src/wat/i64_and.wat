(module
  (func (export "and") (param i64 i64) (result i64)
    (i64.and
      (local.get 0)
      (local.get 1))))
