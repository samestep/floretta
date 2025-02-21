(module $my_module
  (type $dispatch (;0;) (func (param i32)))
  (type $tape_i32 (;1;) (func (param i32)))
  (type $tape_i32_bwd (;2;) (func (result i32)))
  (type $f32_bin (;3;) (func (param f32 f32) (result f32)))
  (type $f64_bin (;4;) (func (param f64 f64) (result f64)))
  (type $f32_bin_bwd (;5;) (func (param f32) (result f32 f32)))
  (type $f64_bin_bwd (;6;) (func (param f64) (result f64 f64)))
  (type $my_type (;7;) (func (param f64) (result f64)))
  (type $my_type_bwd (;8;) (func (param f64) (result f64)))
  (memory $tape_align_4 (;0;) 0)
  (memory $tape_align_8 (;1;) 0)
  (memory $my_memory (;2;) 0)
  (memory $my_memory_bwd (;3;) 0)
  (global $tape_align_4 (;0;) (mut i32) i32.const 0)
  (global $tape_align_8 (;1;) (mut i32) i32.const 0)
  (global $my_global (;2;) f64 f64.const 0x0p+0 (;=0;))
  (export "my_export" (func $my_func))
  (func $tape_i32 (;0;) (type $tape_i32) (param i32)
    (local i32 i32)
    global.get $tape_align_4
    local.tee 1
    i32.const 65539
    i32.add
    i32.const 16
    i32.shr_u
    memory.size
    i32.sub
    local.tee 2
    if ;; label = @1
      local.get 2
      memory.grow
      drop
    end
    local.get 1
    i32.const 4
    i32.add
    global.set $tape_align_4
    local.get 1
    local.get 0
    i32.store
  )
  (func $tape_i32_bwd (;1;) (type $tape_i32_bwd) (result i32)
    (local i32)
    global.get $tape_align_4
    i32.const 4
    i32.sub
    local.tee 0
    global.set $tape_align_4
    local.get 0
    i32.load
  )
  (func $f32_mul (;2;) (type $f32_bin) (param f32 f32) (result f32)
    (local i32 i32)
    global.get $tape_align_4
    local.tee 2
    i32.const 65543
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
    i32.const 8
    i32.add
    global.set $tape_align_4
    local.get 2
    local.get 0
    f32.store
    local.get 2
    local.get 1
    f32.store offset=4
    local.get 0
    local.get 1
    f32.mul
  )
  (func $f32_div (;3;) (type $f32_bin) (param f32 f32) (result f32)
    (local f32 i32 i32)
    global.get $tape_align_4
    local.tee 3
    i32.const 65543
    i32.add
    i32.const 16
    i32.shr_u
    memory.size
    i32.sub
    local.tee 4
    if ;; label = @1
      local.get 4
      memory.grow
      drop
    end
    local.get 3
    i32.const 8
    i32.add
    global.set $tape_align_4
    local.get 3
    local.get 1
    f32.store
    local.get 3
    local.get 0
    local.get 1
    f32.div
    local.tee 2
    f32.store offset=4
    local.get 2
  )
  (func $f64_mul (;4;) (type $f64_bin) (param f64 f64) (result f64)
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
  (func $f64_div (;5;) (type $f64_bin) (param f64 f64) (result f64)
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
  (func $f32_mul_bwd (;6;) (type $f32_bin_bwd) (param f32) (result f32 f32)
    (local i32)
    global.get $tape_align_4
    i32.const 8
    i32.sub
    local.tee 1
    global.set $tape_align_4
    local.get 0
    local.get 1
    f32.load offset=4
    f32.mul
    local.get 0
    local.get 1
    f32.load
    f32.mul
  )
  (func $f32_div_bwd (;7;) (type $f32_bin_bwd) (param f32) (result f32 f32)
    (local f32 i32)
    global.get $tape_align_4
    i32.const 8
    i32.sub
    local.tee 2
    global.set $tape_align_4
    local.get 0
    local.get 2
    f32.load
    f32.div
    local.tee 1
    local.get 1
    local.get 2
    f32.load offset=4
    f32.neg
    f32.mul
  )
  (func $f64_mul_bwd (;8;) (type $f64_bin_bwd) (param f64) (result f64 f64)
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
  (func $f64_div_bwd (;9;) (type $f64_bin_bwd) (param f64) (result f64 f64)
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
  (func $my_func (;10;) (type $my_type) (param $my_param f64) (result f64)
    local.get $my_param
  )
  (func $my_func_bwd (;11;) (type $my_type_bwd) (param $result_0 f64) (result f64)
    (local $my_param f64) (local $tmp_f64 f64) (local $stack_f64_0 f64)
    local.get $result_0
    local.set $stack_f64_0
    i32.const 0
    loop (type $dispatch) (param i32) ;; label = @1
      block (type $dispatch) (param i32) ;; label = @2
        block (type $dispatch) (param i32) ;; label = @3
          br_table 1 (;@2;) 0 (;@3;)
        end
        unreachable
      end
    end
    local.get $stack_f64_0
    f64.const 0x0p+0 (;=0;)
    local.set $stack_f64_0
    local.get $my_param
    f64.add
    local.set $my_param
    local.get $my_param
  )
)
