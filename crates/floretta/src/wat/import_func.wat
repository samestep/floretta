(module
  (import "f64" "exp" (func $exp (param f64) (result f64)))
  (func (export "sigmoid") (param $x f64) (result f64)
    (f64.div
      (f64.const 1)
      (f64.add
        (f64.const 1)
        (call $exp
          (f64.neg
            (local.get $x)))))))
