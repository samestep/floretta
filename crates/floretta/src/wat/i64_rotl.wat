(module
  (func (export "rotl") (param i64 i64) (result i64)
    (i64.rotl
      (local.get 0)
      (local.get 1))))
