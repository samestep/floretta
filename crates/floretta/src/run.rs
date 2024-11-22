use std::iter;

use wasm_encoder::{
    reencode::{Reencode, RoundtripReencoder},
    CodeSection, Encode, ExportKind, ExportSection, Function, FunctionSection, GlobalSection,
    Instruction, MemorySection, Module, TypeSection,
};
use wasmparser::{FunctionBody, Operator, Parser, Payload};

use crate::{
    helper::{
        helpers, FUNC_F64_MUL_BWD, FUNC_F64_MUL_FWD, OFFSET_FUNCTIONS, OFFSET_GLOBALS,
        OFFSET_MEMORIES, OFFSET_TYPES, TYPE_F32_BIN_BWD, TYPE_F32_BIN_FWD, TYPE_F64_BIN_BWD,
        TYPE_F64_BIN_FWD,
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
                    // Forward pass: same type as the original function. For integers, all the
                    // adjoint values are assumed to be equal to the primal values (e.g.
                    // pointers, because of our multi-memory strategy), and for floating point,
                    // all the adjoint values are assumed to be zero.
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
    let mut fwd = Function::new_with_locals_types(locals.iter().skip(sig.params().len()).copied());
    let mut bwd = ReverseFunction::new(num_results);
    for &local in &locals {
        bwd.local(local);
    }
    bwd.instruction(&Instruction::End);
    for i in 0..num_params {
        bwd.instruction(&Instruction::LocalGet(num_results + i));
    }
    let mut stack = Vec::new();
    let mut operators_reader = body.get_operators_reader()?;
    while !operators_reader.eof() {
        let (op, offset) = operators_reader.read_with_offset()?;
        validator.op(offset, &op)?;
        match op {
            Operator::End => {
                fwd.instruction(&Instruction::End);
            }
            Operator::LocalGet { local_index } => {
                fwd.instruction(&Instruction::LocalGet(local_index));
                let ty = locals[u32_to_usize(local_index)];
                match ty {
                    wasm_encoder::ValType::F64 => {
                        let i = num_results + local_index;
                        bwd.instruction(&Instruction::LocalSet(i));
                        bwd.instruction(&Instruction::F64Add);
                        bwd.instruction(&Instruction::LocalGet(i));
                    }
                    _ => todo!(),
                }
                stack.push(ty);
            }
            Operator::F64Mul => {
                fwd.instruction(&Instruction::Call(FUNC_F64_MUL_FWD));
                bwd.instruction(&Instruction::Call(FUNC_F64_MUL_BWD));
            }
            _ => todo!(),
        }
    }
    validator.finish(operators_reader.original_position())?;
    for i in 0..num_results {
        bwd.instruction(&Instruction::LocalGet(i));
    }
    Ok((fwd.into_raw_body(), bwd.into_raw_body()))
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

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

struct ReverseFunction {
    locals: Locals,
    bytes: Vec<u8>,
}

impl ReverseFunction {
    fn new(params: u32) -> Self {
        Self {
            locals: Locals::new(params),
            bytes: Vec::new(),
        }
    }

    fn local(&mut self, ty: wasm_encoder::ValType) -> u32 {
        self.locals.local(ty)
    }

    fn instruction(&mut self, instruction: &Instruction) {
        let n = self.bytes.len();
        instruction.encode(&mut self.bytes);
        self.bytes[n..].reverse();
    }

    fn into_raw_body(mut self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.locals.blocks().encode(&mut bytes);
        bytes.append(&mut self.locals.into_bytes());
        self.bytes.reverse();
        bytes.append(&mut self.bytes);
        bytes
    }
}
