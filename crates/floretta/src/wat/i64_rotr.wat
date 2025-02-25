(module
  (func (export "rotr") (param i64 i64) (result i64)
    (i64.rotr
      (local.get 0)
      (local.get 1))))
