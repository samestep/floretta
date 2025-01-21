use std::{borrow::Cow, iter};

use wasm_encoder::{
    reencode::{Reencode, RoundtripReencoder},
    CodeSection, Encode, ExportKind, ExportSection, Function, FunctionSection, GlobalSection,
    Instruction, MemorySection, Module, TypeSection,
};
use wasmparser::{FunctionBody, Operator, Parser, Payload};

use crate::{
    helper::{
        helpers, FUNC_CONTROL_LOAD, FUNC_CONTROL_STORE, FUNC_F64_MUL_BWD, FUNC_F64_MUL_FWD,
        OFFSET_FUNCTIONS, OFFSET_GLOBALS, OFFSET_MEMORIES, OFFSET_TYPES, TYPE_CONTROL_LOAD,
        TYPE_CONTROL_STORE, TYPE_F32_BIN_BWD, TYPE_F32_BIN_FWD, TYPE_F64_BIN_BWD, TYPE_F64_BIN_FWD,
    },
    util::u32_to_usize,
    validate::{FunctionValidator, ModuleValidator},
    Config, Error,
};

pub fn transform(
    mut validator: impl ModuleValidator,
    config: Config,
    wasm_module: &[u8],
) -> Result<Vec<u8>, Error> {
    let mut types = TypeSection::new();
    // Type for functions and blocks that consume a basic block index for control flow purposes.
    types.ty().func_type(&wasm_encoder::FuncType::new(
        [wasm_encoder::ValType::I32],
        [],
    ));
    // Type for loading a basic block index from the tape.
    types.ty().func_type(&wasm_encoder::FuncType::new(
        [],
        [wasm_encoder::ValType::I32],
    ));
    // Forward-pass arithmetic helper function types to push floating-point values onto the tape.
    types.ty().func_type(&wasm_encoder::FuncType::new(
        [wasm_encoder::ValType::F32, wasm_encoder::ValType::F32],
        [wasm_encoder::ValType::F32],
    ));
    types.ty().func_type(&wasm_encoder::FuncType::new(
        [wasm_encoder::ValType::F64, wasm_encoder::ValType::F64],
        [wasm_encoder::ValType::F64],
    ));
    // Backward-pass arithmetic helper function types to pop floating-point values from the tape.
    types.ty().func_type(&wasm_encoder::FuncType::new(
        [wasm_encoder::ValType::F32],
        [wasm_encoder::ValType::F32, wasm_encoder::ValType::F32],
    ));
    types.ty().func_type(&wasm_encoder::FuncType::new(
        [wasm_encoder::ValType::F64],
        [wasm_encoder::ValType::F64, wasm_encoder::ValType::F64],
    ));
    assert_eq!(types.len(), OFFSET_TYPES);
    let mut functions = FunctionSection::new();
    // Type indices for the tape helper functions.
    functions.function(TYPE_CONTROL_STORE);
    functions.function(TYPE_CONTROL_LOAD);
    functions.function(TYPE_F32_BIN_FWD);
    functions.function(TYPE_F32_BIN_FWD);
    functions.function(TYPE_F64_BIN_FWD);
    functions.function(TYPE_F64_BIN_FWD);
    functions.function(TYPE_F32_BIN_BWD);
    functions.function(TYPE_F32_BIN_BWD);
    functions.function(TYPE_F64_BIN_BWD);
    functions.function(TYPE_F64_BIN_BWD);
    assert_eq!(functions.len(), OFFSET_FUNCTIONS);
    let mut memories = MemorySection::new();
    // The first two memories are always for the tape, so it is possible to translate function
    // bodies without knowing the total number of memories.
    memories.memory(wasm_encoder::MemoryType {
        minimum: 0,
        maximum: None,
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    memories.memory(wasm_encoder::MemoryType {
        minimum: 0,
        maximum: None,
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    assert_eq!(memories.len(), OFFSET_MEMORIES);
    let mut globals = GlobalSection::new();
    // The first two globals are always the tape pointers.
    globals.global(
        wasm_encoder::GlobalType {
            val_type: wasm_encoder::ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(0),
    );
    globals.global(
        wasm_encoder::GlobalType {
            val_type: wasm_encoder::ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(0),
    );
    assert_eq!(globals.len(), OFFSET_GLOBALS);
    let mut exports = ExportSection::new();
    let mut code = CodeSection::new();
    for f in helpers() {
        code.function(&f);
    }
    assert_eq!(code.len(), OFFSET_FUNCTIONS);
    let mut type_sigs = Vec::new();
    let mut func_sigs = Vec::new();
    let mut bodies = 0;
    for payload in Parser::new(0).parse_all(wasm_module) {
        match payload? {
            Payload::TypeSection(section) => {
                validator.type_section(&section)?;
                for func_ty in section.into_iter_err_on_gc_types() {
                    let ty = RoundtripReencoder.func_type(func_ty?)?;
                    // Forward pass: same type as the original function. All the adjoint values are
                    // assumed to be zero.
                    types.ty().func_type(&ty);
                    // Backward pass: results become parameters, and parameters become results.
                    types.ty().func_type(&wasm_encoder::FuncType::new(
                        ty.results().iter().copied(),
                        ty.params().iter().copied(),
                    ));
                    type_sigs.push(ty);
                }
            }
            Payload::FunctionSection(section) => {
                validator.function_section(&section)?;
                for type_index in section {
                    let t = type_index?;
                    // Index arithmetic to account for the fact that we split each original
                    // function type into two; similarly, we also split each actual function
                    // into two.
                    functions.function(OFFSET_TYPES + 2 * t);
                    functions.function(OFFSET_TYPES + 2 * t + 1);
                    // TODO: Finagle things to not have to clone these signatures.
                    func_sigs.push(type_sigs[u32_to_usize(t)].clone());
                }
            }
            Payload::MemorySection(section) => {
                validator.memory_section(&section)?;
                for memory_ty in section {
                    let memory_type = RoundtripReencoder.memory_type(memory_ty?);
                    memories.memory(memory_type);
                    // Duplicate the memory to store adjoint values.
                    memories.memory(memory_type);
                }
            }
            Payload::GlobalSection(section) => {
                validator.global_section(&section)?;
                for global in section {
                    let g = global?;
                    globals.global(
                        RoundtripReencoder.global_type(g.ty)?,
                        &RoundtripReencoder.const_expr(g.init_expr)?,
                    );
                }
            }
            Payload::ExportSection(section) => {
                validator.export_section(&section)?;
                for export in section {
                    let e = export?;
                    let kind = RoundtripReencoder.export_kind(e.kind);
                    match kind {
                        ExportKind::Func => {
                            // More index arithmetic because we split every function into a
                            // forward pass and a backward pass.
                            exports.export(e.name, kind, OFFSET_FUNCTIONS + 2 * e.index);
                            if let Some(name) = config.exports.get(e.name) {
                                // TODO: Should we check that no export with this name already
                                // exists?
                                exports.export(name, kind, OFFSET_FUNCTIONS + 2 * e.index + 1);
                            }
                        }
                        ExportKind::Memory => {
                            exports.export(e.name, kind, OFFSET_MEMORIES + 2 * e.index);
                        }
                        _ => {
                            exports.export(e.name, kind, e.index);
                        }
                    }
                }
            }
            Payload::CodeSectionEntry(body) => {
                let func = validator.code_section_entry(&body)?;
                let (fwd, bwd) = function(func, &func_sigs, bodies, body)?;
                code.raw(&fwd);
                code.raw(&bwd);
                bodies += 1;
            }
            other => validator.payload(&other)?,
        }
    }
    let mut module = Module::new();
    module.section(&types);
    module.section(&functions);
    module.section(&memories);
    module.section(&globals);
    module.section(&exports);
    module.section(&code);
    Ok(module.finish())
}

fn function(
    mut validator: impl FunctionValidator,
    signatures: &[wasm_encoder::FuncType],
    index: u32,
    body: FunctionBody,
) -> Result<(Vec<u8>, Vec<u8>), Error> {
    let sig = &signatures[u32_to_usize(index)];
    let num_params: u32 = sig.params().len().try_into().unwrap();
    let num_results: u32 = sig.results().len().try_into().unwrap();
    let mut locals = sig.params().to_vec();
    let mut locals_reader = body.get_locals_reader()?;
    for _ in 0..locals_reader.get_count() {
        let offset = locals_reader.original_position();
        let (count, ty) = locals_reader.read()?;
        validator.define_locals(offset, count, ty)?;
        locals.extend(iter::repeat_n(
            RoundtripReencoder.val_type(ty)?,
            u32_to_usize(count),
        ));
    }
    // TODO: Preserve compact encoding of locals from the original function.
    let fwd = Function::new_with_locals_types(locals.iter().skip(sig.params().len()).copied());
    let mut bwd = ReverseFunction::new(num_results);
    for &local in &locals {
        bwd.local(local);
    }
    let tmp_f64 = bwd.local(wasm_encoder::ValType::F64);
    for i in 0..num_params {
        bwd.instruction(&Instruction::LocalGet(num_results + i));
    }
    let mut operators_reader = body.get_operators_reader()?;
    let mut func = Func {
        num_results,
        locals: &locals,
        offset: 0, // this initial value should be unused; to be set before each instruction
        operand_stack: Vec::new(),
        operand_stack_height: StackHeight::new(),
        operand_stack_height_min: 0,
        control_stack: Vec::new(),
        fwd,
        bwd,
        tmp_f64,
    };
    validator.check_operand_stack_height(0);
    while !operators_reader.eof() {
        let (op, offset) = operators_reader.read_with_offset()?;
        validator.op(offset, &op)?;
        func.offset = offset.try_into().unwrap();
        func.instruction(op)?;
        let operand_stack_height = func.operand_stack.len().try_into().unwrap();
        validator.check_operand_stack_height(operand_stack_height);
        assert_eq!(func.operand_stack_height.sum(), operand_stack_height);
    }
    validator.finish(operators_reader.original_position())?;
    Ok((
        func.fwd.into_raw_body(),
        func.bwd.into_raw_body(&func.operand_stack),
    ))
}

struct Func<'a> {
    /// Number of results in the original function type.
    num_results: u32,

    /// Locals in the original function.
    locals: &'a [wasm_encoder::ValType], // TODO: use smaller `wasmparser::ValType` instead

    /// The current byte offset in the original function body.
    offset: u32,

    operand_stack: Vec<wasm_encoder::ValType>, // TODO: use smaller `wasmparser::ValType` instead

    operand_stack_height: StackHeight,

    /// The minimum operand stack height reached since this was last reset.
    operand_stack_height_min: usize,

    control_stack: Vec<Control>,

    /// The forward pass under construction.
    fwd: Function,

    /// The backward pass under construction.
    bwd: ReverseFunction,

    /// Local index for an `f64` in the backward pass.
    tmp_f64: u32,
}

impl Func<'_> {
    /// Process an instruction.
    fn instruction(&mut self, op: Operator<'_>) -> Result<(), Error> {
        match op {
            Operator::Loop { blockty } => {
                match blockty {
                    wasmparser::BlockType::Empty => {}
                    wasmparser::BlockType::Type(_) => {}
                    // Handling only the empty and single-result block types means that no data can
                    // be passed when branching.
                    wasmparser::BlockType::FuncType(_) => todo!(),
                }
                self.control_stack.push(Control::Loop);
                self.fwd_control_store();
                self.fwd
                    .instruction(&Instruction::Loop(RoundtripReencoder.block_type(blockty)?));
                self.end_basic_block();
            }
            Operator::End => match self.control_stack.pop() {
                Some(Control::Loop) => {
                    self.fwd.instruction(&Instruction::End);
                }
                None => {
                    self.fwd.instruction(&Instruction::End);
                    self.end_basic_block();
                }
            },
            Operator::BrIf { relative_depth } => {
                self.pop();
                self.fwd_control_store();
                self.bwd.instruction(&Instruction::I32Const(0));
                self.end_basic_block();
                self.fwd.instruction(&Instruction::BrIf(relative_depth));
            }
            Operator::LocalGet { local_index } => {
                let ty = self.local(local_index);
                self.push(ty);
                self.fwd.instruction(&Instruction::LocalGet(local_index));
                match ty {
                    wasm_encoder::ValType::F64 => {
                        let i = self.num_results + local_index;
                        self.bwd.instruction(&Instruction::LocalSet(i));
                        self.bwd.instruction(&Instruction::F64Add);
                        self.bwd.instruction(&Instruction::LocalGet(i));
                    }
                    _ => todo!(),
                }
            }
            Operator::LocalTee { local_index } => {
                let ty = self.local(local_index);
                self.pop();
                self.push(ty);
                self.fwd.instruction(&Instruction::LocalTee(local_index));
                match ty {
                    wasm_encoder::ValType::F64 => {
                        let i = self.num_results + local_index;
                        self.bwd.instruction(&Instruction::LocalSet(i));
                        self.bwd.instruction(&Instruction::F64Const(0.));
                        self.bwd.instruction(&Instruction::F64Add);
                        self.bwd.instruction(&Instruction::LocalGet(i));
                    }
                    _ => todo!(),
                }
            }
            Operator::F64Const { value } => {
                self.push_f64();
                self.fwd.instruction(&Instruction::F64Const(value.into()));
                self.bwd.instruction(&Instruction::Drop);
            }
            Operator::F64Ge => {
                self.pop2();
                self.push_i32();
                self.fwd.instruction(&Instruction::F64Ge);
                self.bwd.instruction(&Instruction::F64Const(0.));
                self.bwd.instruction(&Instruction::F64Const(0.));
                self.bwd.instruction(&Instruction::Drop);
            }
            Operator::F64Sub => {
                self.pop2();
                self.push_f64();
                self.fwd.instruction(&Instruction::F64Sub);
                self.bwd.instruction(&Instruction::F64Neg);
                self.bwd.instruction(&Instruction::LocalGet(self.tmp_f64));
                self.bwd.instruction(&Instruction::LocalTee(self.tmp_f64));
            }
            Operator::F64Mul => {
                self.pop2();
                self.push_f64();
                self.fwd.instruction(&Instruction::Call(FUNC_F64_MUL_FWD));
                self.bwd.instruction(&Instruction::Call(FUNC_F64_MUL_BWD));
            }
            _ => unimplemented!("{op:?}"),
        }
        Ok(())
    }

    fn push(&mut self, ty: wasm_encoder::ValType) {
        self.operand_stack.push(ty);
        self.operand_stack_height.push(ty);
    }

    fn push_i32(&mut self) {
        self.push(wasm_encoder::ValType::I32);
    }

    fn push_f64(&mut self) {
        self.push(wasm_encoder::ValType::F64);
    }

    fn pop(&mut self) {
        let ty = self.operand_stack.pop().unwrap();
        self.operand_stack_height.pop(ty);
        let n = self.operand_stack.len();
        if n < self.operand_stack_height_min {
            assert_eq!(self.operand_stack_height_min, n + 1);
            self.bwd.deepen_stack(ty);
            self.operand_stack_height_min = n;
        }
    }

    fn pop2(&mut self) {
        self.pop();
        self.pop();
    }

    fn local(&self, index: u32) -> wasm_encoder::ValType {
        self.locals[u32_to_usize(index)]
    }

    /// In the forward pass, store the current basic block index on the tape.
    fn fwd_control_store(&mut self) {
        self.fwd
            .instruction(&Instruction::I32Const(self.bwd.basic_block_index()));
        self.fwd.instruction(&Instruction::Call(FUNC_CONTROL_STORE));
    }

    fn end_basic_block(&mut self) {
        self.bwd.end_basic_block(
            self.operand_stack_height,
            &self.operand_stack[self.operand_stack_height_min..],
        );
        self.operand_stack_height_min = self.operand_stack.len();
    }
}

#[derive(Clone, Copy)]
struct StackHeight {
    i32: u32,
    i64: u32,
    f32: u32,
    f64: u32,
}

impl StackHeight {
    fn new() -> Self {
        Self {
            i32: 0,
            i64: 0,
            f32: 0,
            f64: 0,
        }
    }

    fn counter(&mut self, ty: wasm_encoder::ValType) -> &mut u32 {
        match ty {
            wasm_encoder::ValType::I32 => &mut self.i32,
            wasm_encoder::ValType::I64 => &mut self.i64,
            wasm_encoder::ValType::F32 => &mut self.f32,
            wasm_encoder::ValType::F64 => &mut self.f64,
            wasm_encoder::ValType::V128 => unimplemented!(),
            wasm_encoder::ValType::Ref(_) => unimplemented!(),
        }
    }

    fn push(&mut self, ty: wasm_encoder::ValType) {
        *self.counter(ty) += 1;
    }

    fn pop(&mut self, ty: wasm_encoder::ValType) {
        *self.counter(ty) -= 1;
    }

    fn sum(&self) -> u32 {
        self.i32 + self.i64 + self.f32 + self.f64
    }

    fn take_max(&mut self, other: Self) {
        self.i32 = self.i32.max(other.i32);
        self.i64 = self.i64.max(other.i64);
        self.f32 = self.f32.max(other.f32);
        self.f64 = self.f64.max(other.f64);
    }
}

enum Control {
    Loop,
}

struct Locals {
    blocks: u32,
    count: u32,
    bytes: Vec<u8>,
}

impl Locals {
    fn new(params: u32) -> Self {
        Self {
            blocks: 0,
            count: params,
            bytes: Vec::new(),
        }
    }

    fn blocks(&self) -> u32 {
        self.blocks
    }

    fn count(&self) -> u32 {
        self.count
    }

    fn locals(&mut self, count: u32, ty: wasm_encoder::ValType) {
        count.encode(&mut self.bytes);
        ty.encode(&mut self.bytes);
        self.blocks += 1;
        self.count += count;
    }

    fn local(&mut self, ty: wasm_encoder::ValType) -> u32 {
        let i = self.count;
        self.locals(1, ty);
        i
    }

    fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Clone, Copy)]
struct BasicBlock {
    /// Offset of the first body instruction byte for this basic block.
    start_offset: u32,

    /// Start index of the list of the types of operands that were on the stack before this basic
    /// block but popped from the stack during this basic block.
    stack_start_offset: u32,

    /// Start index of the list of the types of operands on the stack after this basic block that
    /// were pushed to the stack during this basic block.
    stack_end_offset: u32,
}

struct ReverseFunction {
    locals: Locals,
    body: Vec<u8>,
    stacks: Vec<wasm_encoder::ValType>, // TODO: use smaller `wasmparser::ValType` instead
    basic_blocks: Vec<BasicBlock>,
    block_start_offset: usize,
    block_stack_offset: usize,
    operand_stack_height: StackHeight,
    max_stack_heights: StackHeight,
}

impl ReverseFunction {
    fn new(params: u32) -> Self {
        Self {
            locals: Locals::new(params),
            body: Vec::new(),
            stacks: Vec::new(),
            basic_blocks: Vec::new(),
            block_start_offset: 0,
            block_stack_offset: 0,
            operand_stack_height: StackHeight::new(),
            max_stack_heights: StackHeight::new(),
        }
    }

    fn local(&mut self, ty: wasm_encoder::ValType) -> u32 {
        self.locals.local(ty)
    }

    /// Extend the portion of the stack used by the current basic block.
    fn deepen_stack(&mut self, ty: wasm_encoder::ValType) {
        self.stacks.push(ty);
    }

    fn instruction(&mut self, instruction: &Instruction) {
        reverse_encode(&mut self.body, instruction);
    }

    fn basic_block_index(&self) -> i32 {
        self.basic_blocks.len().try_into().unwrap()
    }

    fn end_basic_block(&mut self, height: StackHeight, stack: &[wasm_encoder::ValType]) {
        self.body[self.block_start_offset..].reverse();
        let stack_end_offset = self.stacks.len().try_into().unwrap();
        self.stacks.extend_from_slice(stack);
        self.basic_blocks.push(BasicBlock {
            start_offset: self.block_start_offset.try_into().unwrap(),
            stack_start_offset: self.block_stack_offset.try_into().unwrap(),
            stack_end_offset,
        });
        self.block_start_offset = self.body.len();
        self.block_stack_offset = self.stacks.len();
        self.operand_stack_height = height;
        self.max_stack_heights.take_max(height);
    }

    fn into_raw_body(mut self, operand_stack: &[wasm_encoder::ValType]) -> Vec<u8> {
        let local_count = self.locals.count();
        // When we cross a basic block boundary in the backward pass, everything on the stack needs
        // to be put into locals so that they can be retrieved after the `loop` dispatches to a
        // given basic block. We've kept track of the maximum number of values in the stack for each
        // type at each basic block boundary, so now we allocate enough locals to store them all.
        self.locals
            .locals(self.max_stack_heights.i32, wasm_encoder::ValType::I32);
        self.locals
            .locals(self.max_stack_heights.i64, wasm_encoder::ValType::I64);
        self.locals
            .locals(self.max_stack_heights.f32, wasm_encoder::ValType::F32);
        self.locals
            .locals(self.max_stack_heights.f64, wasm_encoder::ValType::F64);
        let mut body = Vec::new();
        self.locals.blocks().encode(&mut body);
        body.extend_from_slice(self.locals.bytes());
        let operand_stack_height = self.operand_stack_height;
        ReverseReverseFunction {
            func: self,
            local_count,
            body,
            operand_stack_height,
        }
        .consume(operand_stack)
    }
}

struct ReverseReverseFunction {
    func: ReverseFunction,
    local_count: u32,
    body: Vec<u8>,
    operand_stack_height: StackHeight,
}

impl ReverseReverseFunction {
    fn consume(mut self, operand_stack: &[wasm_encoder::ValType]) -> Vec<u8> {
        let mut operand_stack_height = StackHeight::new();
        for (i, &ty) in (0..).zip(operand_stack.iter()) {
            Instruction::LocalGet(i).encode(&mut self.body);
            let j = self.local_index_raw(operand_stack_height, ty);
            Instruction::LocalSet(j).encode(&mut self.body);
            operand_stack_height.push(ty);
        }
        let n = self.func.basic_blocks.len();
        // We don't yet support the explicit `return` instruction, so we know that the forward pass
        // exited from the last basic block with an implicit return; so, when we enter the state
        // machine for the backward pass, we know to enter at that last basic block.
        Instruction::I32Const((n - 1).try_into().unwrap()).encode(&mut self.body);
        // The dispatch loop block type signature consumes a basic block index; this happens to be
        // the same type signature for storing a basic block index in the forward pass.
        let blockty = wasm_encoder::BlockType::FunctionType(TYPE_CONTROL_STORE);
        Instruction::Loop(blockty).encode(&mut self.body);
        for _ in 0..n {
            Instruction::Block(blockty).encode(&mut self.body);
        }
        // We insert one last `block` to give us a branch target for the error case where we somehow
        // got an invalid basic block index.
        Instruction::Block(blockty).encode(&mut self.body);
        // We'll put the reversed basic blocks of the backward pass in reverse order compared to the
        // original function, because the first basic block is the entrypoint to the original
        // function, so in the backward pass it becomes the sole exit point; by putting it at the
        // end, we can just do an implicit return instead of an explicit `return` instruction.
        let table: Vec<u32> = (1..=n.try_into().unwrap()).rev().collect();
        Instruction::BrTable(Cow::from(&table), 0).encode(&mut self.body);
        Instruction::End.encode(&mut self.body);
        // If we got an invalid basic block index, just trap immediately.
        Instruction::Unreachable.encode(&mut self.body);
        for i in (1..n).rev() {
            Instruction::End.encode(&mut self.body);
            self.basic_block(i);
            Instruction::Call(FUNC_CONTROL_LOAD).encode(&mut self.body); // Load basic block index.
            Instruction::Br(i.try_into().unwrap()).encode(&mut self.body); // Branch to the `loop`.
        }
        Instruction::End.encode(&mut self.body);
        Instruction::End.encode(&mut self.body);
        // First basic block goes outside the whole `loop`/`block` structure, to easily allow the
        // implicit `return`.
        self.basic_block(0);
        Instruction::End.encode(&mut self.body);
        self.body
    }

    fn basic_block(&mut self, index: usize) {
        let bb = self.func.basic_blocks[index];
        let body_start = u32_to_usize(bb.start_offset);
        let stack_start = u32_to_usize(bb.stack_start_offset);
        let stack_mid = u32_to_usize(bb.stack_end_offset);
        let (body_end, stack_end) = match self.func.basic_blocks.get(index + 1) {
            Some(&next) => (
                u32_to_usize(next.start_offset),
                u32_to_usize(next.stack_start_offset),
            ),
            None => (self.func.body.len(), self.func.stacks.len()),
        };
        // We're traversing this basic block backward, so first we need to push the adjoints of the
        // values that were on the stack at the end of this basic block in the forward pass. They
        // appear in order from bottom to top, which is the order we want the `local.get`
        // instructions to appear, but for bookkeeping of the operand stack, we need to instead
        // pretend that we're popping from a "stack" represented by our locals. So, we use the same
        // trick as in the body of each basic block: reverse-encode each instruction, then go back
        // and re-reverse all the instructions we just encoded.
        let n = self.body.len();
        for &ty in self.func.stacks[stack_mid..stack_end].iter().rev() {
            self.operand_stack_height.pop(ty);
            let i = self.local_index(ty);
            reverse_encode(&mut self.body, &Instruction::LocalSet(i));
            // TODO: Only set stack locals to zero when they won't be overwritten later anyway.
            match ty {
                wasm_encoder::ValType::I32 => {
                    reverse_encode(&mut self.body, &Instruction::I32Const(0));
                }
                wasm_encoder::ValType::I64 => {
                    reverse_encode(&mut self.body, &Instruction::I64Const(0));
                }
                wasm_encoder::ValType::F32 => {
                    reverse_encode(&mut self.body, &Instruction::F32Const(0.));
                }
                wasm_encoder::ValType::F64 => {
                    reverse_encode(&mut self.body, &Instruction::F64Const(0.));
                }
                wasm_encoder::ValType::V128 => unimplemented!(),
                wasm_encoder::ValType::Ref(_) => unimplemented!(),
            }
            reverse_encode(&mut self.body, &Instruction::LocalGet(i));
        }
        self.body[n..].reverse();
        self.body
            .extend_from_slice(&self.func.body[body_start..body_end]);
        // Now we need to pop the adjoints of the values that were on the stack at the beginning of
        // this basic block in the forward pass. They appear in order from top to bottom, which
        // again happens to be the order we want, but once again we need to double-reverse
        // everything for operand stack bookkeeping.
        let n = self.body.len();
        for &ty in self.func.stacks[stack_start..stack_mid].iter().rev() {
            let i = self.local_index(ty);
            reverse_encode(&mut self.body, &Instruction::LocalSet(i));
            self.operand_stack_height.push(ty);
        }
        self.body[n..].reverse();
    }

    fn local_index(&self, ty: wasm_encoder::ValType) -> u32 {
        self.local_index_raw(self.operand_stack_height, ty)
    }

    fn local_index_raw(&self, operand_stack_height: StackHeight, ty: wasm_encoder::ValType) -> u32 {
        let i = match ty {
            wasm_encoder::ValType::I32 => operand_stack_height.i32,
            wasm_encoder::ValType::I64 => {
                operand_stack_height.i64 + self.func.max_stack_heights.i32
            }
            wasm_encoder::ValType::F32 => {
                operand_stack_height.f32
                    + self.func.max_stack_heights.i64
                    + self.func.max_stack_heights.i32
            }
            wasm_encoder::ValType::F64 => {
                operand_stack_height.f64
                    + self.func.max_stack_heights.f32
                    + self.func.max_stack_heights.i64
                    + self.func.max_stack_heights.i32
            }
            wasm_encoder::ValType::V128 => unimplemented!(),
            wasm_encoder::ValType::Ref(_) => unimplemented!(),
        };
        self.local_count + i
    }
}

fn reverse_encode(sink: &mut Vec<u8>, instruction: &Instruction) {
    let n = sink.len();
    instruction.encode(sink);
    sink[n..].reverse();
}
