(module
  (func (export "or") (param i64 i64) (result i64)
    (i64.or
      (local.get 0)
      (local.get 1))))
