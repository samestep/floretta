use wasm_encoder::{
    BlockType, ConstExpr, FuncType, Function, GlobalType, MemArg, MemoryType, ValType,
};

use crate::util::NumImports;

pub const OFFSET_TYPES: u32 = 11;
pub const TYPE_DISPATCH: u32 = 0;
const TYPE_TAPE_I32: u32 = 1;
const TYPE_TAPE_I32_BWD: u32 = 2;
const TYPE_F32_PAIR: u32 = 3;
const TYPE_F32_UNARY: u32 = 4;
const TYPE_F32_BIN_FWD: u32 = 5;
const TYPE_F32_BIN_BWD: u32 = 6;
const TYPE_F64_PAIR: u32 = 7;
const TYPE_F64_UNARY: u32 = 8;
const TYPE_F64_BIN_FWD: u32 = 9;
const TYPE_F64_BIN_BWD: u32 = 10;

pub const OFFSET_MEMORIES: u32 = 3;
const MEM_TAPE_ALIGN_1: u32 = 0;
const MEM_TAPE_ALIGN_4: u32 = 1;
const MEM_TAPE_ALIGN_8: u32 = 2;

pub const OFFSET_GLOBALS: u32 = 3;
const GLOBAL_TAPE_ALIGN_1: u32 = 0;
const GLOBAL_TAPE_ALIGN_4: u32 = 1;
const GLOBAL_TAPE_ALIGN_8: u32 = 2;

pub const OFFSET_FUNCTIONS: u32 = 22;

pub struct FuncOffsets {
    num_imports: NumImports,
}

impl FuncOffsets {
    pub fn new(num_imports: NumImports) -> Self {
        Self { num_imports }
    }

    fn offset(&self) -> u32 {
        2 * self.num_imports.func
    }

    pub fn tape_i32(&self) -> u32 {
        self.offset()
    }

    pub fn tape_i32_bwd(&self) -> u32 {
        self.offset() + 1
    }

    pub fn f32_sqrt_fwd(&self) -> u32 {
        self.offset() + 2
    }

    pub fn f32_sqrt_bwd(&self) -> u32 {
        self.offset() + 3
    }

    pub fn f32_mul_fwd(&self) -> u32 {
        self.offset() + 4
    }

    pub fn f32_mul_bwd(&self) -> u32 {
        self.offset() + 5
    }

    pub fn f32_div_fwd(&self) -> u32 {
        self.offset() + 6
    }

    pub fn f32_div_bwd(&self) -> u32 {
        self.offset() + 7
    }

    pub fn f32_min_fwd(&self) -> u32 {
        self.offset() + 8
    }

    pub fn f32_min_bwd(&self) -> u32 {
        self.offset() + 9
    }

    pub fn f32_max_fwd(&self) -> u32 {
        self.offset() + 10
    }

    pub fn f32_max_bwd(&self) -> u32 {
        self.offset() + 11
    }

    pub fn f64_sqrt_fwd(&self) -> u32 {
        self.offset() + 12
    }

    pub fn f64_sqrt_bwd(&self) -> u32 {
        self.offset() + 13
    }

    pub fn f64_mul_fwd(&self) -> u32 {
        self.offset() + 14
    }

    pub fn f64_mul_bwd(&self) -> u32 {
        self.offset() + 15
    }

    pub fn f64_div_fwd(&self) -> u32 {
        self.offset() + 16
    }

    pub fn f64_div_bwd(&self) -> u32 {
        self.offset() + 17
    }

    pub fn f64_min_fwd(&self) -> u32 {
        self.offset() + 18
    }

    pub fn f64_min_bwd(&self) -> u32 {
        self.offset() + 19
    }

    pub fn f64_max_fwd(&self) -> u32 {
        self.offset() + 20
    }

    pub fn f64_max_bwd(&self) -> u32 {
        self.offset() + 21
    }
}

pub fn helper_types() -> impl Iterator<Item = (&'static str, FuncType)> {
    [
        (TYPE_DISPATCH, "dispatch", FuncType::new([ValType::I32], [])),
        (TYPE_TAPE_I32, "tape_i32", FuncType::new([ValType::I32], [])),
        (
            TYPE_TAPE_I32_BWD,
            "tape_i32_bwd",
            FuncType::new([], [ValType::I32]),
        ),
        (
            TYPE_F32_PAIR,
            "f32_pair",
            FuncType::new([], [ValType::F32, ValType::F32]),
        ),
        (
            TYPE_F32_UNARY,
            "f32_unary",
            FuncType::new([ValType::F32], [ValType::F32]),
        ),
        (
            TYPE_F32_BIN_FWD,
            "f32_bin",
            FuncType::new([ValType::F32, ValType::F32], [ValType::F32]),
        ),
        (
            TYPE_F32_BIN_BWD,
            "f32_bin_bwd",
            FuncType::new([ValType::F32], [ValType::F32, ValType::F32]),
        ),
        (
            TYPE_F64_PAIR,
            "f64_pair",
            FuncType::new([], [ValType::F64, ValType::F64]),
        ),
        (
            TYPE_F64_UNARY,
            "f64_unary",
            FuncType::new([ValType::F64], [ValType::F64]),
        ),
        (
            TYPE_F64_BIN_FWD,
            "f64_bin",
            FuncType::new([ValType::F64, ValType::F64], [ValType::F64]),
        ),
        (
            TYPE_F64_BIN_BWD,
            "f64_bin_bwd",
            FuncType::new([ValType::F64], [ValType::F64, ValType::F64]),
        ),
    ]
    .into_iter()
    .zip(0..)
    .map(|((i, name, ty), j)| {
        assert_eq!(i, j);
        (name, ty)
    })
}

pub fn helper_memories() -> impl Iterator<Item = (&'static str, MemoryType)> {
    let memory = MemoryType {
        minimum: 0,
        maximum: None,
        memory64: false,
        shared: false,
        page_size_log2: None,
    };
    [
        (MEM_TAPE_ALIGN_1, "tape_align_1"),
        (MEM_TAPE_ALIGN_4, "tape_align_4"),
        (MEM_TAPE_ALIGN_8, "tape_align_8"),
    ]
    .into_iter()
    .zip(0..)
    .map(move |((i, name), j)| {
        assert_eq!(i, j);
        (name, memory)
    })
}

pub fn helper_globals() -> impl Iterator<Item = (&'static str, GlobalType, ConstExpr)> {
    let ty = GlobalType {
        val_type: wasm_encoder::ValType::I32,
        mutable: true,
        shared: false,
    };
    [
        (GLOBAL_TAPE_ALIGN_1, "tape_align_1", ConstExpr::i32_const(0)),
        (GLOBAL_TAPE_ALIGN_4, "tape_align_4", ConstExpr::i32_const(0)),
        (GLOBAL_TAPE_ALIGN_8, "tape_align_8", ConstExpr::i32_const(0)),
    ]
    .into_iter()
    .zip(0..)
    .map(move |((i, name, init), j)| {
        assert_eq!(i, j);
        (name, ty, init)
    })
}

pub fn helper_functions() -> impl Iterator<Item = (&'static str, u32, Function)> {
    let offsets = FuncOffsets::new(NumImports::default());
    [
        (
            offsets.tape_i32(),
            "tape_i32",
            TYPE_TAPE_I32,
            func_tape_i32(),
        ),
        (
            offsets.tape_i32_bwd(),
            "tape_i32_bwd",
            TYPE_TAPE_I32_BWD,
            func_tape_i32_bwd(),
        ),
        (
            offsets.f32_sqrt_fwd(),
            "f32_sqrt",
            TYPE_F32_UNARY,
            func_f32_sqrt_fwd(),
        ),
        (
            offsets.f32_sqrt_bwd(),
            "f32_sqrt_bwd",
            TYPE_F32_UNARY,
            func_f32_sqrt_bwd(),
        ),
        (
            offsets.f32_mul_fwd(),
            "f32_mul",
            TYPE_F32_BIN_FWD,
            func_f32_mul_fwd(),
        ),
        (
            offsets.f32_mul_bwd(),
            "f32_mul_bwd",
            TYPE_F32_BIN_BWD,
            func_f32_mul_bwd(),
        ),
        (
            offsets.f32_div_fwd(),
            "f32_div",
            TYPE_F32_BIN_FWD,
            func_f32_div_fwd(),
        ),
        (
            offsets.f32_div_bwd(),
            "f32_div_bwd",
            TYPE_F32_BIN_BWD,
            func_f32_div_bwd(),
        ),
        (
            offsets.f32_min_fwd(),
            "f32_min",
            TYPE_F32_BIN_FWD,
            func_f32_min_fwd(),
        ),
        (
            offsets.f32_min_bwd(),
            "f32_min_bwd",
            TYPE_F32_BIN_BWD,
            func_f32_min_bwd(),
        ),
        (
            offsets.f32_max_fwd(),
            "f32_max",
            TYPE_F32_BIN_FWD,
            func_f32_max_fwd(),
        ),
        (
            offsets.f32_max_bwd(),
            "f32_max_bwd",
            TYPE_F32_BIN_BWD,
            func_f32_max_bwd(),
        ),
        (
            offsets.f64_sqrt_fwd(),
            "f64_sqrt",
            TYPE_F64_UNARY,
            func_f64_sqrt_fwd(),
        ),
        (
            offsets.f64_sqrt_bwd(),
            "f64_sqrt_bwd",
            TYPE_F64_UNARY,
            func_f64_sqrt_bwd(),
        ),
        (
            offsets.f64_mul_fwd(),
            "f64_mul",
            TYPE_F64_BIN_FWD,
            func_f64_mul_fwd(),
        ),
        (
            offsets.f64_mul_bwd(),
            "f64_mul_bwd",
            TYPE_F64_BIN_BWD,
            func_f64_mul_bwd(),
        ),
        (
            offsets.f64_div_fwd(),
            "f64_div",
            TYPE_F64_BIN_FWD,
            func_f64_div_fwd(),
        ),
        (
            offsets.f64_div_bwd(),
            "f64_div_bwd",
            TYPE_F64_BIN_BWD,
            func_f64_div_bwd(),
        ),
        (
            offsets.f64_min_fwd(),
            "f64_min",
            TYPE_F64_BIN_FWD,
            func_f64_min_fwd(),
        ),
        (
            offsets.f64_min_bwd(),
            "f64_min_bwd",
            TYPE_F64_BIN_BWD,
            func_f64_min_bwd(),
        ),
        (
            offsets.f64_max_fwd(),
            "f64_max",
            TYPE_F64_BIN_FWD,
            func_f64_max_fwd(),
        ),
        (
            offsets.f64_max_bwd(),
            "f64_max_bwd",
            TYPE_F64_BIN_BWD,
            func_f64_max_bwd(),
        ),
    ]
    .into_iter()
    .zip(0..)
    .map(|((i, name, ty, function), j)| {
        assert_eq!(i, j);
        (name, ty, function)
    })
}

struct Tape {
    memory: u32,
    global: u32,
    local: u32,
}

impl Tape {
    fn grow(self, f: &mut Function, local: u32, bytes: i32) {
        f.instructions()
            .global_get(self.global)
            .local_tee(self.local)
            .i32_const(bytes + 65535)
            .i32_add()
            .i32_const(16)
            .i32_shr_u()
            .memory_size(self.memory)
            .i32_sub()
            .local_tee(local)
            .if_(BlockType::Empty)
            .local_get(local)
            .memory_grow(self.memory)
            .drop()
            .end()
            .local_get(self.local)
            .i32_const(bytes)
            .i32_add()
            .global_set(self.global);
    }

    fn shrink(self, f: &mut Function, bytes: i32) {
        f.instructions()
            .global_get(self.global)
            .i32_const(bytes)
            .i32_sub()
            .local_tee(self.local)
            .global_set(self.global);
    }
}

fn func_tape_i32() -> Function {
    let [k, i, n] = [0, 1, 2];
    let mut f = Function::new([(2, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_4,
        global: GLOBAL_TAPE_ALIGN_4,
        local: i,
    }
    .grow(&mut f, n, 4);
    f.instructions()
        .local_get(i)
        .local_get(k)
        .i32_store(MemArg {
            offset: 0,
            align: 2,
            memory_index: MEM_TAPE_ALIGN_4,
        })
        .end();
    f
}

fn func_tape_i32_bwd() -> Function {
    let [i] = [0];
    let mut f = Function::new([(1, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_4,
        global: GLOBAL_TAPE_ALIGN_4,
        local: i,
    }
    .shrink(&mut f, 4);
    f.instructions()
        .local_get(i)
        .i32_load(MemArg {
            offset: 0,
            align: 2,
            memory_index: MEM_TAPE_ALIGN_4,
        })
        .end();
    f
}

fn func_f32_sqrt_fwd() -> Function {
    let [x, y, i, n] = [0, 1, 2, 3];
    let mut f = Function::new([(1, ValType::F32), (2, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_4,
        global: GLOBAL_TAPE_ALIGN_4,
        local: i,
    }
    .grow(&mut f, n, 4);
    f.instructions()
        .local_get(i)
        .local_get(x)
        .f32_sqrt()
        .local_tee(y)
        .f32_store(MemArg {
            offset: 0,
            align: 2,
            memory_index: MEM_TAPE_ALIGN_4,
        })
        .local_get(y)
        .end();
    f
}

fn func_f32_sqrt_bwd() -> Function {
    let [dy, y, i] = [0, 1, 2];
    let mut f = Function::new([(1, ValType::F32), (1, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_4,
        global: GLOBAL_TAPE_ALIGN_4,
        local: i,
    }
    .shrink(&mut f, 4);
    f.instructions()
        .local_get(dy)
        .local_get(i)
        .f32_load(MemArg {
            offset: 0,
            align: 2,
            memory_index: MEM_TAPE_ALIGN_4,
        })
        .local_tee(y)
        .local_get(y)
        .f32_add()
        .f32_div()
        .end();
    f
}

fn func_f32_mul_fwd() -> Function {
    let [x, y, i, n] = [0, 1, 2, 3];
    let mut f = Function::new([(2, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_4,
        global: GLOBAL_TAPE_ALIGN_4,
        local: i,
    }
    .grow(&mut f, n, 8);
    f.instructions()
        .local_get(i)
        .local_get(x)
        .f32_store(MemArg {
            offset: 0,
            align: 2,
            memory_index: MEM_TAPE_ALIGN_4,
        })
        .local_get(i)
        .local_get(y)
        .f32_store(MemArg {
            offset: 4,
            align: 2,
            memory_index: MEM_TAPE_ALIGN_4,
        })
        .local_get(x)
        .local_get(y)
        .f32_mul()
        .end();
    f
}

fn func_f32_mul_bwd() -> Function {
    let [dz, i] = [0, 1];
    let mut f = Function::new([(1, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_4,
        global: GLOBAL_TAPE_ALIGN_4,
        local: i,
    }
    .shrink(&mut f, 8);
    f.instructions()
        .local_get(dz)
        .local_get(i)
        .f32_load(MemArg {
            offset: 4,
            align: 2,
            memory_index: MEM_TAPE_ALIGN_4,
        })
        .f32_mul()
        .local_get(dz)
        .local_get(i)
        .f32_load(MemArg {
            offset: 0,
            align: 2,
            memory_index: MEM_TAPE_ALIGN_4,
        })
        .f32_mul()
        .end();
    f
}

fn func_f32_div_fwd() -> Function {
    let [x, y, z, i, n] = [0, 1, 2, 3, 4];
    let mut f = Function::new([(1, ValType::F32), (2, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_4,
        global: GLOBAL_TAPE_ALIGN_4,
        local: i,
    }
    .grow(&mut f, n, 8);
    f.instructions()
        .local_get(i)
        .local_get(y)
        .f32_store(MemArg {
            offset: 0,
            align: 2,
            memory_index: MEM_TAPE_ALIGN_4,
        })
        .local_get(i)
        .local_get(x)
        .local_get(y)
        .f32_div()
        .local_tee(z)
        .f32_store(MemArg {
            offset: 4,
            align: 2,
            memory_index: MEM_TAPE_ALIGN_4,
        })
        .local_get(z)
        .end();
    f
}

fn func_f32_div_bwd() -> Function {
    let [dz, dx, i] = [0, 1, 2];
    let mut f = Function::new([(1, ValType::F32), (1, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_4,
        global: GLOBAL_TAPE_ALIGN_4,
        local: i,
    }
    .shrink(&mut f, 8);
    f.instructions()
        .local_get(dz)
        .local_get(i)
        .f32_load(MemArg {
            offset: 0,
            align: 2,
            memory_index: MEM_TAPE_ALIGN_4,
        })
        .f32_div()
        .local_tee(dx)
        .local_get(dx)
        .local_get(i)
        .f32_load(MemArg {
            offset: 4,
            align: 2,
            memory_index: MEM_TAPE_ALIGN_4,
        })
        .f32_neg()
        .f32_mul()
        .end();
    f
}

fn func_f32_min_fwd() -> Function {
    let [x, y, i, n] = [0, 1, 2, 3];
    let mut f = Function::new([(2, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_1,
        global: GLOBAL_TAPE_ALIGN_1,
        local: i,
    }
    .grow(&mut f, n, 1);
    f.instructions()
        .local_get(i)
        .local_get(x)
        .local_get(y)
        .f32_gt()
        .i32_store8(MemArg {
            offset: 0,
            align: 0,
            memory_index: MEM_TAPE_ALIGN_1,
        })
        .local_get(x)
        .local_get(y)
        .f32_min()
        .end();
    f
}

fn func_f32_min_bwd() -> Function {
    let [dz, i] = [0, 1];
    let mut f = Function::new([(1, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_1,
        global: GLOBAL_TAPE_ALIGN_1,
        local: i,
    }
    .shrink(&mut f, 1);
    f.instructions()
        .local_get(i)
        .i32_load8_u(MemArg {
            offset: 0,
            align: 0,
            memory_index: MEM_TAPE_ALIGN_1,
        })
        .if_(BlockType::FunctionType(TYPE_F32_PAIR))
        .f32_const(0.)
        .local_get(dz)
        .else_()
        .local_get(dz)
        .f32_const(0.)
        .end()
        .end();
    f
}

fn func_f32_max_fwd() -> Function {
    let [x, y, i, n] = [0, 1, 2, 3];
    let mut f = Function::new([(2, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_1,
        global: GLOBAL_TAPE_ALIGN_1,
        local: i,
    }
    .grow(&mut f, n, 1);
    f.instructions()
        .local_get(i)
        .local_get(x)
        .local_get(y)
        .f32_lt()
        .i32_store8(MemArg {
            offset: 0,
            align: 0,
            memory_index: MEM_TAPE_ALIGN_1,
        })
        .local_get(x)
        .local_get(y)
        .f32_max()
        .end();
    f
}

fn func_f32_max_bwd() -> Function {
    let [dz, i] = [0, 1];
    let mut f = Function::new([(1, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_1,
        global: GLOBAL_TAPE_ALIGN_1,
        local: i,
    }
    .shrink(&mut f, 1);
    f.instructions()
        .local_get(i)
        .i32_load8_u(MemArg {
            offset: 0,
            align: 0,
            memory_index: MEM_TAPE_ALIGN_1,
        })
        .if_(BlockType::FunctionType(TYPE_F32_PAIR))
        .f32_const(0.)
        .local_get(dz)
        .else_()
        .local_get(dz)
        .f32_const(0.)
        .end()
        .end();
    f
}

fn func_f64_sqrt_fwd() -> Function {
    let [x, y, i, n] = [0, 1, 2, 3];
    let mut f = Function::new([(1, ValType::F64), (2, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_8,
        global: GLOBAL_TAPE_ALIGN_8,
        local: i,
    }
    .grow(&mut f, n, 8);
    f.instructions()
        .local_get(i)
        .local_get(x)
        .f64_sqrt()
        .local_tee(y)
        .f64_store(MemArg {
            offset: 0,
            align: 3,
            memory_index: MEM_TAPE_ALIGN_8,
        })
        .local_get(y)
        .end();
    f
}

fn func_f64_sqrt_bwd() -> Function {
    let [dy, y, i] = [0, 1, 2];
    let mut f = Function::new([(1, ValType::F64), (1, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_8,
        global: GLOBAL_TAPE_ALIGN_8,
        local: i,
    }
    .shrink(&mut f, 8);
    f.instructions()
        .local_get(dy)
        .local_get(i)
        .f64_load(MemArg {
            offset: 0,
            align: 3,
            memory_index: MEM_TAPE_ALIGN_8,
        })
        .local_tee(y)
        .local_get(y)
        .f64_add()
        .f64_div()
        .end();
    f
}

fn func_f64_mul_fwd() -> Function {
    let [x, y, i, n] = [0, 1, 2, 3];
    let mut f = Function::new([(2, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_8,
        global: GLOBAL_TAPE_ALIGN_8,
        local: i,
    }
    .grow(&mut f, n, 16);
    f.instructions()
        .local_get(i)
        .local_get(x)
        .f64_store(MemArg {
            offset: 0,
            align: 3,
            memory_index: MEM_TAPE_ALIGN_8,
        })
        .local_get(i)
        .local_get(y)
        .f64_store(MemArg {
            offset: 8,
            align: 3,
            memory_index: MEM_TAPE_ALIGN_8,
        })
        .local_get(x)
        .local_get(y)
        .f64_mul()
        .end();
    f
}

fn func_f64_mul_bwd() -> Function {
    let [dz, i] = [0, 1];
    let mut f = Function::new([(1, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_8,
        global: GLOBAL_TAPE_ALIGN_8,
        local: i,
    }
    .shrink(&mut f, 16);
    f.instructions()
        .local_get(dz)
        .local_get(i)
        .f64_load(MemArg {
            offset: 8,
            align: 3,
            memory_index: MEM_TAPE_ALIGN_8,
        })
        .f64_mul()
        .local_get(dz)
        .local_get(i)
        .f64_load(MemArg {
            offset: 0,
            align: 3,
            memory_index: MEM_TAPE_ALIGN_8,
        })
        .f64_mul()
        .end();
    f
}

fn func_f64_div_fwd() -> Function {
    let [x, y, z, i, n] = [0, 1, 2, 3, 4];
    let mut f = Function::new([(1, ValType::F64), (2, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_8,
        global: GLOBAL_TAPE_ALIGN_8,
        local: i,
    }
    .grow(&mut f, n, 16);
    f.instructions()
        .local_get(i)
        .local_get(y)
        .f64_store(MemArg {
            offset: 0,
            align: 3,
            memory_index: MEM_TAPE_ALIGN_8,
        })
        .local_get(i)
        .local_get(x)
        .local_get(y)
        .f64_div()
        .local_tee(z)
        .f64_store(MemArg {
            offset: 8,
            align: 3,
            memory_index: MEM_TAPE_ALIGN_8,
        })
        .local_get(z)
        .end();
    f
}

fn func_f64_div_bwd() -> Function {
    let [dz, dx, i] = [0, 1, 2];
    let mut f = Function::new([(1, ValType::F64), (1, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_8,
        global: GLOBAL_TAPE_ALIGN_8,
        local: i,
    }
    .shrink(&mut f, 16);
    f.instructions()
        .local_get(dz)
        .local_get(i)
        .f64_load(MemArg {
            offset: 0,
            align: 3,
            memory_index: MEM_TAPE_ALIGN_8,
        })
        .f64_div()
        .local_tee(dx)
        .local_get(dx)
        .local_get(i)
        .f64_load(MemArg {
            offset: 8,
            align: 3,
            memory_index: MEM_TAPE_ALIGN_8,
        })
        .f64_neg()
        .f64_mul()
        .end();
    f
}

fn func_f64_min_fwd() -> Function {
    let [x, y, i, n] = [0, 1, 2, 3];
    let mut f = Function::new([(2, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_1,
        global: GLOBAL_TAPE_ALIGN_1,
        local: i,
    }
    .grow(&mut f, n, 1);
    f.instructions()
        .local_get(i)
        .local_get(x)
        .local_get(y)
        .f64_gt()
        .i32_store8(MemArg {
            offset: 0,
            align: 0,
            memory_index: MEM_TAPE_ALIGN_1,
        })
        .local_get(x)
        .local_get(y)
        .f64_min()
        .end();
    f
}

fn func_f64_min_bwd() -> Function {
    let [dz, i] = [0, 1];
    let mut f = Function::new([(1, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_1,
        global: GLOBAL_TAPE_ALIGN_1,
        local: i,
    }
    .shrink(&mut f, 1);
    f.instructions()
        .local_get(i)
        .i32_load8_u(MemArg {
            offset: 0,
            align: 0,
            memory_index: MEM_TAPE_ALIGN_1,
        })
        .if_(BlockType::FunctionType(TYPE_F64_PAIR))
        .f64_const(0.)
        .local_get(dz)
        .else_()
        .local_get(dz)
        .f64_const(0.)
        .end()
        .end();
    f
}

fn func_f64_max_fwd() -> Function {
    let [x, y, i, n] = [0, 1, 2, 3];
    let mut f = Function::new([(2, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_1,
        global: GLOBAL_TAPE_ALIGN_1,
        local: i,
    }
    .grow(&mut f, n, 1);
    f.instructions()
        .local_get(i)
        .local_get(x)
        .local_get(y)
        .f64_lt()
        .i32_store8(MemArg {
            offset: 0,
            align: 0,
            memory_index: MEM_TAPE_ALIGN_1,
        })
        .local_get(x)
        .local_get(y)
        .f64_max()
        .end();
    f
}

fn func_f64_max_bwd() -> Function {
    let [dz, i] = [0, 1];
    let mut f = Function::new([(1, ValType::I32)]);
    Tape {
        memory: MEM_TAPE_ALIGN_1,
        global: GLOBAL_TAPE_ALIGN_1,
        local: i,
    }
    .shrink(&mut f, 1);
    f.instructions()
        .local_get(i)
        .i32_load8_u(MemArg {
            offset: 0,
            align: 0,
            memory_index: MEM_TAPE_ALIGN_1,
        })
        .if_(BlockType::FunctionType(TYPE_F64_PAIR))
        .f64_const(0.)
        .local_get(dz)
        .else_()
        .local_get(dz)
        .f64_const(0.)
        .end()
        .end();
    f
}
