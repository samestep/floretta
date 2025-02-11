(module $my_module
  (func $my_func (export "my_export") (param $my_param f64) (result f64)
    (local.get $my_param)))
