(module
  (func (export "mul") (param i64 i64) (result i64)
    (i64.mul
      (local.get 0)
      (local.get 1))))
