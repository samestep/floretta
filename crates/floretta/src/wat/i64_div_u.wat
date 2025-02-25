(module
  (func (export "div_u") (param i64 i64) (result i64)
    (i64.div_u
      (local.get 0)
      (local.get 1))))
