(module
  (func (export "add") (param i64 i64) (result i64)
    (i64.add
      (local.get 0)
      (local.get 1))))
