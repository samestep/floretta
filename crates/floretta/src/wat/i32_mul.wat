(module
  (func (export "mul") (param i32 i32) (result i32)
    (i32.mul
      (local.get 0)
      (local.get 1))))
