(module
  (func (export "popcnt") (param i64) (result i64)
    (i64.popcnt
      (local.get 0))))
