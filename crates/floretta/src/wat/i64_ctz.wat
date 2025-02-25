(module
  (func (export "ctz") (param i64) (result i64)
    (i64.ctz
      (local.get 0))))
