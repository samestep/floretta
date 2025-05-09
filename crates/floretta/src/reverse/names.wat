(module $my_module
  (type $dispatch (;0;) (func (param i32)))
  (type $tape_i32 (;1;) (func (param i32)))
  (type $tape_i32_bwd (;2;) (func (result i32)))
  (type $f32_pair (;3;) (func (result f32 f32)))
  (type $f32_unary (;4;) (func (param f32) (result f32)))
  (type $f32_bin (;5;) (func (param f32 f32) (result f32)))
  (type $f32_bin_bwd (;6;) (func (param f32) (result f32 f32)))
  (type $f64_pair (;7;) (func (result f64 f64)))
  (type $f64_unary (;8;) (func (param f64) (result f64)))
  (type $f64_bin (;9;) (func (param f64 f64) (result f64)))
  (type $f64_bin_bwd (;10;) (func (param f64) (result f64 f64)))
  (type $my_type (;11;) (func (param i32 f64) (result f64 i32)))
  (type $my_type_bwd (;12;) (func (param f64) (result f64)))
  (import "foo" "bar" (func $my_imported_func (;0;) (type $my_type)))
  (import "baz" "qux" (func $my_imported_func_bwd (;1;) (type $my_type_bwd)))
  (memory $tape_align_1 (;0;) 0)
  (memory $tape_align_4 (;1;) 0)
  (memory $tape_align_8 (;2;) 0)
  (memory $my_memory (;3;) 0)
  (memory $my_memory_bwd (;4;) 0)
  (global $tape_align_1 (;0;) (mut i32) i32.const 0)
  (global $tape_align_4 (;1;) (mut i32) i32.const 0)
  (global $tape_align_8 (;2;) (mut i32) i32.const 0)
  (global $my_global (;3;) f64 f64.const 0x0p+0 (;=0;))
  (export "my_exported_memory" (memory $my_memory))
  (export "my_other_exported_memory" (memory $my_memory_bwd))
  (export "my_exported_func" (func $my_func))
  (export "my_other_exported_func" (func $my_func_bwd))
  (func $tape_i32 (;2;) (type $tape_i32) (param i32)
    (local i32 i32)
    global.get $tape_align_4
    local.tee 1
    i32.const 65539
    i32.add
    i32.const 16
    i32.shr_u
    memory.size $tape_align_4
    i32.sub
    local.tee 2
    if ;; label = @1
      local.get 2
      memory.grow $tape_align_4
      drop
    end
    local.get 1
    i32.const 4
    i32.add
    global.set $tape_align_4
    local.get 1
    local.get 0
    i32.store $tape_align_4
  )
  (func $tape_i32_bwd (;3;) (type $tape_i32_bwd) (result i32)
    (local i32)
    global.get $tape_align_4
    i32.const 4
    i32.sub
    local.tee 0
    global.set $tape_align_4
    local.get 0
    i32.load $tape_align_4
  )
  (func $f32_sqrt (;4;) (type $f32_unary) (param f32) (result f32)
    (local f32 i32 i32)
    global.get $tape_align_4
    local.tee 2
    i32.const 65539
    i32.add
    i32.const 16
    i32.shr_u
    memory.size $tape_align_4
    i32.sub
    local.tee 3
    if ;; label = @1
      local.get 3
      memory.grow $tape_align_4
      drop
    end
    local.get 2
    i32.const 4
    i32.add
    global.set $tape_align_4
    local.get 2
    local.get 0
    f32.sqrt
    local.tee 1
    f32.store $tape_align_4
    local.get 1
  )
  (func $f32_sqrt_bwd (;5;) (type $f32_unary) (param f32) (result f32)
    (local f32 i32)
    global.get $tape_align_4
    i32.const 4
    i32.sub
    local.tee 2
    global.set $tape_align_4
    local.get 0
    local.get 2
    f32.load $tape_align_4
    local.tee 1
    local.get 1
    f32.add
    f32.div
  )
  (func $f32_mul (;6;) (type $f32_bin) (param f32 f32) (result f32)
    (local i32 i32)
    global.get $tape_align_4
    local.tee 2
    i32.const 65543
    i32.add
    i32.const 16
    i32.shr_u
    memory.size $tape_align_4
    i32.sub
    local.tee 3
    if ;; label = @1
      local.get 3
      memory.grow $tape_align_4
      drop
    end
    local.get 2
    i32.const 8
    i32.add
    global.set $tape_align_4
    local.get 2
    local.get 0
    f32.store $tape_align_4
    local.get 2
    local.get 1
    f32.store $tape_align_4 offset=4
    local.get 0
    local.get 1
    f32.mul
  )
  (func $f32_mul_bwd (;7;) (type $f32_bin_bwd) (param f32) (result f32 f32)
    (local i32)
    global.get $tape_align_4
    i32.const 8
    i32.sub
    local.tee 1
    global.set $tape_align_4
    local.get 0
    local.get 1
    f32.load $tape_align_4 offset=4
    f32.mul
    local.get 0
    local.get 1
    f32.load $tape_align_4
    f32.mul
  )
  (func $f32_div (;8;) (type $f32_bin) (param f32 f32) (result f32)
    (local f32 i32 i32)
    global.get $tape_align_4
    local.tee 3
    i32.const 65543
    i32.add
    i32.const 16
    i32.shr_u
    memory.size $tape_align_4
    i32.sub
    local.tee 4
    if ;; label = @1
      local.get 4
      memory.grow $tape_align_4
      drop
    end
    local.get 3
    i32.const 8
    i32.add
    global.set $tape_align_4
    local.get 3
    local.get 1
    f32.store $tape_align_4
    local.get 3
    local.get 0
    local.get 1
    f32.div
    local.tee 2
    f32.store $tape_align_4 offset=4
    local.get 2
  )
  (func $f32_div_bwd (;9;) (type $f32_bin_bwd) (param f32) (result f32 f32)
    (local f32 i32)
    global.get $tape_align_4
    i32.const 8
    i32.sub
    local.tee 2
    global.set $tape_align_4
    local.get 0
    local.get 2
    f32.load $tape_align_4
    f32.div
    local.tee 1
    local.get 1
    local.get 2
    f32.load $tape_align_4 offset=4
    f32.neg
    f32.mul
  )
  (func $f32_min (;10;) (type $f32_bin) (param f32 f32) (result f32)
    (local i32 i32)
    global.get $tape_align_1
    local.tee 2
    i32.const 65536
    i32.add
    i32.const 16
    i32.shr_u
    memory.size
    i32.sub
    local.tee 3
    if ;; label = @1
      local.get 3
      memory.grow
      drop
    end
    local.get 2
    i32.const 1
    i32.add
    global.set $tape_align_1
    local.get 2
    local.get 0
    local.get 1
    f32.gt
    i32.store8
    local.get 0
    local.get 1
    f32.min
  )
  (func $f32_min_bwd (;11;) (type $f32_bin_bwd) (param f32) (result f32 f32)
    (local i32)
    global.get $tape_align_1
    i32.const 1
    i32.sub
    local.tee 1
    global.set $tape_align_1
    local.get 1
    i32.load8_u
    if (type $f32_pair) (result f32 f32) ;; label = @1
      f32.const 0x0p+0 (;=0;)
      local.get 0
    else
      local.get 0
      f32.const 0x0p+0 (;=0;)
    end
  )
  (func $f32_max (;12;) (type $f32_bin) (param f32 f32) (result f32)
    (local i32 i32)
    global.get $tape_align_1
    local.tee 2
    i32.const 65536
    i32.add
    i32.const 16
    i32.shr_u
    memory.size
    i32.sub
    local.tee 3
    if ;; label = @1
      local.get 3
      memory.grow
      drop
    end
    local.get 2
    i32.const 1
    i32.add
    global.set $tape_align_1
    local.get 2
    local.get 0
    local.get 1
    f32.lt
    i32.store8
    local.get 0
    local.get 1
    f32.max
  )
  (func $f32_max_bwd (;13;) (type $f32_bin_bwd) (param f32) (result f32 f32)
    (local i32)
    global.get $tape_align_1
    i32.const 1
    i32.sub
    local.tee 1
    global.set $tape_align_1
    local.get 1
    i32.load8_u
    if (type $f32_pair) (result f32 f32) ;; label = @1
      f32.const 0x0p+0 (;=0;)
      local.get 0
    else
      local.get 0
      f32.const 0x0p+0 (;=0;)
    end
  )
  (func $f32_copysign (;14;) (type $f32_bin) (param f32 f32) (result f32)
    (local i32 i32)
    global.get $tape_align_1
    local.tee 2
    i32.const 65536
    i32.add
    i32.const 16
    i32.shr_u
    memory.size
    i32.sub
    local.tee 3
    if ;; label = @1
      local.get 3
      memory.grow
      drop
    end
    local.get 2
    i32.const 1
    i32.add
    global.set $tape_align_1
    local.get 2
    local.get 0
    local.get 1
    f32.copysign
    local.get 0
    f32.eq
    i32.store8
    local.get 0
    local.get 1
    f32.copysign
  )
  (func $f32_copysign_bwd (;15;) (type $f32_bin_bwd) (param f32) (result f32 f32)
    (local i32)
    global.get $tape_align_1
    i32.const 1
    i32.sub
    local.tee 1
    global.set $tape_align_1
    local.get 0
    local.get 0
    f32.const -0x0p+0 (;=-0;)
    f32.copysign
    local.get 1
    i32.load8_u
    select
    f32.const 0x0p+0 (;=0;)
  )
  (func $f64_sqrt (;16;) (type $f64_unary) (param f64) (result f64)
    (local f64 i32 i32)
    global.get $tape_align_8
    local.tee 2
    i32.const 65543
    i32.add
    i32.const 16
    i32.shr_u
    memory.size $tape_align_8
    i32.sub
    local.tee 3
    if ;; label = @1
      local.get 3
      memory.grow $tape_align_8
      drop
    end
    local.get 2
    i32.const 8
    i32.add
    global.set $tape_align_8
    local.get 2
    local.get 0
    f64.sqrt
    local.tee 1
    f64.store $tape_align_8
    local.get 1
  )
  (func $f64_sqrt_bwd (;17;) (type $f64_unary) (param f64) (result f64)
    (local f64 i32)
    global.get $tape_align_8
    i32.const 8
    i32.sub
    local.tee 2
    global.set $tape_align_8
    local.get 0
    local.get 2
    f64.load $tape_align_8
    local.tee 1
    local.get 1
    f64.add
    f64.div
  )
  (func $f64_mul (;18;) (type $f64_bin) (param f64 f64) (result f64)
    (local i32 i32)
    global.get $tape_align_8
    local.tee 2
    i32.const 65551
    i32.add
    i32.const 16
    i32.shr_u
    memory.size $tape_align_8
    i32.sub
    local.tee 3
    if ;; label = @1
      local.get 3
      memory.grow $tape_align_8
      drop
    end
    local.get 2
    i32.const 16
    i32.add
    global.set $tape_align_8
    local.get 2
    local.get 0
    f64.store $tape_align_8
    local.get 2
    local.get 1
    f64.store $tape_align_8 offset=8
    local.get 0
    local.get 1
    f64.mul
  )
  (func $f64_mul_bwd (;19;) (type $f64_bin_bwd) (param f64) (result f64 f64)
    (local i32)
    global.get $tape_align_8
    i32.const 16
    i32.sub
    local.tee 1
    global.set $tape_align_8
    local.get 0
    local.get 1
    f64.load $tape_align_8 offset=8
    f64.mul
    local.get 0
    local.get 1
    f64.load $tape_align_8
    f64.mul
  )
  (func $f64_div (;20;) (type $f64_bin) (param f64 f64) (result f64)
    (local f64 i32 i32)
    global.get $tape_align_8
    local.tee 3
    i32.const 65551
    i32.add
    i32.const 16
    i32.shr_u
    memory.size $tape_align_8
    i32.sub
    local.tee 4
    if ;; label = @1
      local.get 4
      memory.grow $tape_align_8
      drop
    end
    local.get 3
    i32.const 16
    i32.add
    global.set $tape_align_8
    local.get 3
    local.get 1
    f64.store $tape_align_8
    local.get 3
    local.get 0
    local.get 1
    f64.div
    local.tee 2
    f64.store $tape_align_8 offset=8
    local.get 2
  )
  (func $f64_div_bwd (;21;) (type $f64_bin_bwd) (param f64) (result f64 f64)
    (local f64 i32)
    global.get $tape_align_8
    i32.const 16
    i32.sub
    local.tee 2
    global.set $tape_align_8
    local.get 0
    local.get 2
    f64.load $tape_align_8
    f64.div
    local.tee 1
    local.get 1
    local.get 2
    f64.load $tape_align_8 offset=8
    f64.neg
    f64.mul
  )
  (func $f64_min (;22;) (type $f64_bin) (param f64 f64) (result f64)
    (local i32 i32)
    global.get $tape_align_1
    local.tee 2
    i32.const 65536
    i32.add
    i32.const 16
    i32.shr_u
    memory.size
    i32.sub
    local.tee 3
    if ;; label = @1
      local.get 3
      memory.grow
      drop
    end
    local.get 2
    i32.const 1
    i32.add
    global.set $tape_align_1
    local.get 2
    local.get 0
    local.get 1
    f64.gt
    i32.store8
    local.get 0
    local.get 1
    f64.min
  )
  (func $f64_min_bwd (;23;) (type $f64_bin_bwd) (param f64) (result f64 f64)
    (local i32)
    global.get $tape_align_1
    i32.const 1
    i32.sub
    local.tee 1
    global.set $tape_align_1
    local.get 1
    i32.load8_u
    if (type $f64_pair) (result f64 f64) ;; label = @1
      f64.const 0x0p+0 (;=0;)
      local.get 0
    else
      local.get 0
      f64.const 0x0p+0 (;=0;)
    end
  )
  (func $f64_max (;24;) (type $f64_bin) (param f64 f64) (result f64)
    (local i32 i32)
    global.get $tape_align_1
    local.tee 2
    i32.const 65536
    i32.add
    i32.const 16
    i32.shr_u
    memory.size
    i32.sub
    local.tee 3
    if ;; label = @1
      local.get 3
      memory.grow
      drop
    end
    local.get 2
    i32.const 1
    i32.add
    global.set $tape_align_1
    local.get 2
    local.get 0
    local.get 1
    f64.lt
    i32.store8
    local.get 0
    local.get 1
    f64.max
  )
  (func $f64_max_bwd (;25;) (type $f64_bin_bwd) (param f64) (result f64 f64)
    (local i32)
    global.get $tape_align_1
    i32.const 1
    i32.sub
    local.tee 1
    global.set $tape_align_1
    local.get 1
    i32.load8_u
    if (type $f64_pair) (result f64 f64) ;; label = @1
      f64.const 0x0p+0 (;=0;)
      local.get 0
    else
      local.get 0
      f64.const 0x0p+0 (;=0;)
    end
  )
  (func $f64_copysign (;26;) (type $f64_bin) (param f64 f64) (result f64)
    (local i32 i32)
    global.get $tape_align_1
    local.tee 2
    i32.const 65536
    i32.add
    i32.const 16
    i32.shr_u
    memory.size
    i32.sub
    local.tee 3
    if ;; label = @1
      local.get 3
      memory.grow
      drop
    end
    local.get 2
    i32.const 1
    i32.add
    global.set $tape_align_1
    local.get 2
    local.get 0
    local.get 1
    f64.copysign
    local.get 0
    f64.eq
    i32.store8
    local.get 0
    local.get 1
    f64.copysign
  )
  (func $f64_copysign_bwd (;27;) (type $f64_bin_bwd) (param f64) (result f64 f64)
    (local i32)
    global.get $tape_align_1
    i32.const 1
    i32.sub
    local.tee 1
    global.set $tape_align_1
    local.get 0
    local.get 0
    f64.const -0x0p+0 (;=-0;)
    f64.copysign
    local.get 1
    i32.load8_u
    select
    f64.const 0x0p+0 (;=0;)
  )
  (func $my_func (;28;) (type $my_type) (param $my_int_param i32) (param $my_float_param f64) (result f64 i32)
    (local f32 f64 i32)
    local.get $my_float_param
    local.get $my_int_param
    i32.const 0
    call $tape_i32
  )
  (func $my_func_bwd (;29;) (type $my_type_bwd) (param $result_0 f64) (result f64)
    (local $my_float_param f64) (local f32 f64) (local $tmp_i32 i32) (local $branch_f64_0 f64)
    local.get $result_0
    local.set $branch_f64_0
    call $tape_i32_bwd
    loop (type $dispatch) (param i32) ;; label = @1
      block (type $dispatch) (param i32) ;; label = @2
        block (type $dispatch) (param i32) ;; label = @3
          br_table 1 (;@2;) 0 (;@3;)
        end
        unreachable
      end
    end
    local.get $branch_f64_0
    f64.const 0x0p+0 (;=0;)
    local.set $branch_f64_0
    local.get $my_float_param
    f64.add
    local.set $my_float_param
    local.get $my_float_param
  )
)
