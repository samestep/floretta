(module $my_module
  (type $my_type (func (param i32 f64) (result f64 i32)))
  (import "foo" "bar" (func $my_imported_func (type $my_type)))
  (memory $my_memory (export "my_exported_memory") 0)
  (global $my_global f64
    (f64.const 0))
  (func $my_func (export "my_exported_func") (type $my_type)
    (param $my_int_param i32) (param $my_float_param f64) (result f64 i32)
    (local.get $my_float_param)
    (local.get $my_int_param)))
