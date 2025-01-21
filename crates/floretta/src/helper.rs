use wasm_encoder::{Function, Instruction};

pub const OFFSET_TYPES: u32 = 6;
pub const TYPE_CONTROL_STORE: u32 = 0;
pub const TYPE_CONTROL_LOAD: u32 = 1;
pub const TYPE_F32_BIN_FWD: u32 = 2;
pub const TYPE_F64_BIN_FWD: u32 = 3;
pub const TYPE_F32_BIN_BWD: u32 = 4;
pub const TYPE_F64_BIN_BWD: u32 = 5;

pub const OFFSET_FUNCTIONS: u32 = 10;
pub const FUNC_CONTROL_STORE: u32 = 0;
pub const FUNC_CONTROL_LOAD: u32 = 1;
pub const FUNC_F32_MUL_FWD: u32 = 2;
pub const FUNC_F32_DIV_FWD: u32 = 3;
pub const FUNC_F64_MUL_FWD: u32 = 4;
pub const FUNC_F64_DIV_FWD: u32 = 5;
pub const FUNC_F32_MUL_BWD: u32 = 6;
pub const FUNC_F32_DIV_BWD: u32 = 7;
pub const FUNC_F64_MUL_BWD: u32 = 8;
pub const FUNC_F64_DIV_BWD: u32 = 9;

pub const OFFSET_MEMORIES: u32 = 2;
const MEM_32_TAPE: u32 = 0;
const MEM_64_TAPE: u32 = 1;

pub const OFFSET_GLOBALS: u32 = 2;
const GLOBAL_32_TAPE: u32 = 0;
const GLOBAL_64_TAPE: u32 = 1;

pub fn helpers() -> impl Iterator<Item = Function> {
    [
        func_control_store(),
        func_control_load(),
        func_f32_mul_fwd(),
        func_f32_div_fwd(),
        func_f64_mul_fwd(),
        func_f64_div_fwd(),
        func_f32_mul_bwd(),
        func_f32_div_bwd(),
        func_f64_mul_bwd(),
        func_f64_div_bwd(),
    ]
    .into_iter()
}

struct Tape {
    memory: u32,
    global: u32,
    local: u32,
}

impl Tape {
    fn grow(self, f: &mut Function, local: u32, bytes: i32) {
        f.instruction(&Instruction::GlobalGet(self.global));
        f.instruction(&Instruction::LocalTee(self.local));
        f.instruction(&Instruction::I32Const(bytes + 65535));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(16));
        f.instruction(&Instruction::I32ShrU);
        f.instruction(&Instruction::MemorySize(self.memory));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalTee(local));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(local));
        f.instruction(&Instruction::MemoryGrow(self.memory));
        f.instruction(&Instruction::Drop);
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::LocalGet(self.local));
        f.instruction(&Instruction::I32Const(bytes));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(self.global));
    }

    fn shrink(self, f: &mut Function, bytes: i32) {
        f.instruction(&Instruction::GlobalGet(self.global));
        f.instruction(&Instruction::I32Const(bytes));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalTee(self.local));
        f.instruction(&Instruction::GlobalSet(self.global));
    }
}

fn func_control_store() -> Function {
    let (k, i, n) = (0, 1, 2);
    let mut f = Function::new([(2, wasm_encoder::ValType::I32)]);
    Tape {
        memory: MEM_32_TAPE,
        global: GLOBAL_32_TAPE,
        local: i,
    }
    .grow(&mut f, n, 4);
    f.instruction(&Instruction::LocalGet(i));
    f.instruction(&Instruction::LocalGet(k));
    f.instruction(&Instruction::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: MEM_32_TAPE,
    }));
    f.instruction(&Instruction::End);
    f
}

fn func_control_load() -> Function {
    let (i,) = (0,);
    let mut f = Function::new([(1, wasm_encoder::ValType::I32)]);
    Tape {
        memory: MEM_32_TAPE,
        global: GLOBAL_32_TAPE,
        local: i,
    }
    .shrink(&mut f, 4);
    f.instruction(&Instruction::LocalGet(i));
    f.instruction(&Instruction::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: MEM_32_TAPE,
    }));
    f.instruction(&Instruction::End);
    f
}

fn func_f32_mul_fwd() -> Function {
    let (x, y, i, n) = (0, 1, 2, 3);
    let mut f = Function::new([(2, wasm_encoder::ValType::I32)]);
    Tape {
        memory: MEM_32_TAPE,
        global: GLOBAL_32_TAPE,
        local: i,
    }
    .grow(&mut f, n, 8);
    f.instruction(&Instruction::LocalGet(i));
    f.instruction(&Instruction::LocalGet(x));
    f.instruction(&Instruction::F32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: MEM_32_TAPE,
    }));
    f.instruction(&Instruction::LocalGet(i));
    f.instruction(&Instruction::LocalGet(y));
    f.instruction(&Instruction::F32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: MEM_32_TAPE,
    }));
    f.instruction(&Instruction::LocalGet(x));
    f.instruction(&Instruction::LocalGet(y));
    f.instruction(&Instruction::F32Mul);
    f.instruction(&Instruction::End);
    f
}

fn func_f32_div_fwd() -> Function {
    let (x, y, z, i, n) = (0, 1, 2, 3, 4);
    let mut f = Function::new([
        (1, wasm_encoder::ValType::F32),
        (2, wasm_encoder::ValType::I32),
    ]);
    Tape {
        memory: MEM_32_TAPE,
        global: GLOBAL_32_TAPE,
        local: i,
    }
    .grow(&mut f, n, 8);
    f.instruction(&Instruction::LocalGet(i));
    f.instruction(&Instruction::LocalGet(y));
    f.instruction(&Instruction::F32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: MEM_32_TAPE,
    }));
    f.instruction(&Instruction::LocalGet(i));
    f.instruction(&Instruction::LocalGet(x));
    f.instruction(&Instruction::LocalGet(y));
    f.instruction(&Instruction::F32Div);
    f.instruction(&Instruction::LocalTee(z));
    f.instruction(&Instruction::F32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: MEM_32_TAPE,
    }));
    f.instruction(&Instruction::LocalGet(z));
    f.instruction(&Instruction::End);
    f
}

fn func_f64_mul_fwd() -> Function {
    let (x, y, i, n) = (0, 1, 2, 3);
    let mut f = Function::new([(2, wasm_encoder::ValType::I32)]);
    Tape {
        memory: MEM_64_TAPE,
        global: GLOBAL_64_TAPE,
        local: i,
    }
    .grow(&mut f, n, 16);
    f.instruction(&Instruction::LocalGet(i));
    f.instruction(&Instruction::LocalGet(x));
    f.instruction(&Instruction::F64Store(wasm_encoder::MemArg {
        offset: 0,
        align: 3,
        memory_index: MEM_64_TAPE,
    }));
    f.instruction(&Instruction::LocalGet(i));
    f.instruction(&Instruction::LocalGet(y));
    f.instruction(&Instruction::F64Store(wasm_encoder::MemArg {
        offset: 8,
        align: 3,
        memory_index: MEM_64_TAPE,
    }));
    f.instruction(&Instruction::LocalGet(x));
    f.instruction(&Instruction::LocalGet(y));
    f.instruction(&Instruction::F64Mul);
    f.instruction(&Instruction::End);
    f
}

fn func_f64_div_fwd() -> Function {
    let (x, y, z, i, n) = (0, 1, 2, 3, 4);
    let mut f = Function::new([
        (1, wasm_encoder::ValType::F64),
        (2, wasm_encoder::ValType::I32),
    ]);
    Tape {
        memory: MEM_64_TAPE,
        global: GLOBAL_64_TAPE,
        local: i,
    }
    .grow(&mut f, n, 16);
    f.instruction(&Instruction::LocalGet(i));
    f.instruction(&Instruction::LocalGet(y));
    f.instruction(&Instruction::F64Store(wasm_encoder::MemArg {
        offset: 0,
        align: 3,
        memory_index: MEM_64_TAPE,
    }));
    f.instruction(&Instruction::LocalGet(i));
    f.instruction(&Instruction::LocalGet(x));
    f.instruction(&Instruction::LocalGet(y));
    f.instruction(&Instruction::F64Div);
    f.instruction(&Instruction::LocalTee(z));
    f.instruction(&Instruction::F64Store(wasm_encoder::MemArg {
        offset: 8,
        align: 3,
        memory_index: MEM_64_TAPE,
    }));
    f.instruction(&Instruction::LocalGet(z));
    f.instruction(&Instruction::End);
    f
}

fn func_f32_mul_bwd() -> Function {
    let (dz, i) = (0, 1);
    let mut f = Function::new([(1, wasm_encoder::ValType::I32)]);
    Tape {
        memory: MEM_32_TAPE,
        global: GLOBAL_32_TAPE,
        local: i,
    }
    .shrink(&mut f, 8);
    f.instruction(&Instruction::LocalGet(dz));
    f.instruction(&Instruction::LocalGet(i));
    f.instruction(&Instruction::F32Load(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: MEM_32_TAPE,
    }));
    f.instruction(&Instruction::F32Mul);
    f.instruction(&Instruction::LocalGet(dz));
    f.instruction(&Instruction::LocalGet(i));
    f.instruction(&Instruction::F32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: MEM_32_TAPE,
    }));
    f.instruction(&Instruction::F32Mul);
    f.instruction(&Instruction::End);
    f
}

fn func_f32_div_bwd() -> Function {
    let (dz, dx, i) = (0, 1, 2);
    let mut f = Function::new([
        (1, wasm_encoder::ValType::F32),
        (1, wasm_encoder::ValType::I32),
    ]);
    Tape {
        memory: MEM_32_TAPE,
        global: GLOBAL_32_TAPE,
        local: i,
    }
    .shrink(&mut f, 8);
    f.instruction(&Instruction::LocalGet(dz));
    f.instruction(&Instruction::LocalGet(i));
    f.instruction(&Instruction::F32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: MEM_32_TAPE,
    }));
    f.instruction(&Instruction::F32Div);
    f.instruction(&Instruction::LocalTee(dx));
    f.instruction(&Instruction::LocalGet(dx));
    f.instruction(&Instruction::LocalGet(i));
    f.instruction(&Instruction::F32Load(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: MEM_32_TAPE,
    }));
    f.instruction(&Instruction::F32Neg);
    f.instruction(&Instruction::F32Mul);
    f.instruction(&Instruction::End);
    f
}

fn func_f64_mul_bwd() -> Function {
    let (dz, i) = (0, 1);
    let mut f = Function::new([(1, wasm_encoder::ValType::I32)]);
    Tape {
        memory: MEM_64_TAPE,
        global: GLOBAL_64_TAPE,
        local: i,
    }
    .shrink(&mut f, 16);
    f.instruction(&Instruction::LocalGet(dz));
    f.instruction(&Instruction::LocalGet(i));
    f.instruction(&Instruction::F64Load(wasm_encoder::MemArg {
        offset: 8,
        align: 3,
        memory_index: MEM_64_TAPE,
    }));
    f.instruction(&Instruction::F64Mul);
    f.instruction(&Instruction::LocalGet(dz));
    f.instruction(&Instruction::LocalGet(i));
    f.instruction(&Instruction::F64Load(wasm_encoder::MemArg {
        offset: 0,
        align: 3,
        memory_index: MEM_64_TAPE,
    }));
    f.instruction(&Instruction::F64Mul);
    f.instruction(&Instruction::End);
    f
}

fn func_f64_div_bwd() -> Function {
    let (dz, dx, i) = (0, 1, 2);
    let mut f = Function::new([
        (1, wasm_encoder::ValType::F64),
        (1, wasm_encoder::ValType::I32),
    ]);
    Tape {
        memory: MEM_64_TAPE,
        global: GLOBAL_64_TAPE,
        local: i,
    }
    .shrink(&mut f, 16);
    f.instruction(&Instruction::LocalGet(dz));
    f.instruction(&Instruction::LocalGet(i));
    f.instruction(&Instruction::F64Load(wasm_encoder::MemArg {
        offset: 0,
        align: 3,
        memory_index: MEM_64_TAPE,
    }));
    f.instruction(&Instruction::F64Div);
    f.instruction(&Instruction::LocalTee(dx));
    f.instruction(&Instruction::LocalGet(dx));
    f.instruction(&Instruction::LocalGet(i));
    f.instruction(&Instruction::F64Load(wasm_encoder::MemArg {
        offset: 8,
        align: 3,
        memory_index: MEM_64_TAPE,
    }));
    f.instruction(&Instruction::F64Neg);
    f.instruction(&Instruction::F64Mul);
    f.instruction(&Instruction::End);
    f
}
