(module
  (func (export "sub") (param i64 i64) (result i64)
    (i64.sub
      (local.get 0)
      (local.get 1))))
