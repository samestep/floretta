(module
  (func (export "div_s") (param i64 i64) (result i64)
    (i64.div_s
      (local.get 0)
      (local.get 1))))
