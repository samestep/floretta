(module
  (func (export "rotl") (param i32 i32) (result i32)
    (i32.rotl
      (local.get 0)
      (local.get 1))))
