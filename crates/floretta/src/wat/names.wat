(module $my_module
  (type $my_type (func (param f64) (result f64)))
  (memory $my_memory 0)
  (global $my_global f64
    (f64.const 0))
  (func $my_func (export "my_export") (type $my_type)
    (param $my_param f64) (result f64)
    (local.get $my_param)))
