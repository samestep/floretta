use wasm_encoder::{
    reencode::{Reencode, RoundtripReencoder},
    CodeSection, ExportKind, ExportSection, Function, FunctionSection, GlobalSection, Instruction,
    MemorySection, Module, TypeSection,
};
use wasmparser::{FunctionBody, Operator, Parser, Payload};

use crate::{
    helper::{
        helpers, OFFSET_FUNCTIONS, OFFSET_GLOBALS, OFFSET_MEMORIES, OFFSET_TYPES, TYPE_F32_BIN_BWD,
        TYPE_F32_BIN_FWD, TYPE_F64_BIN_BWD, TYPE_F64_BIN_FWD,
    },
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
                let (fwd, bwd) = function(func, body)?;
                code.function(&fwd);
                code.function(&bwd);
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
    body: FunctionBody,
) -> Result<(Function, Function), Error> {
    let mut locals = Vec::new();
    let mut locals_reader = body.get_locals_reader()?;
    for _ in 0..locals_reader.get_count() {
        let offset = locals_reader.original_position();
        let (count, ty) = locals_reader.read()?;
        validator.define_locals(offset, count, ty)?;
        locals.push((count, RoundtripReencoder.val_type(ty)?));
    }
    let mut fwd = Function::new(locals);
    let mut bwd = Function::new([]);
    let mut operators_reader = body.get_operators_reader()?;
    while !operators_reader.eof() {
        let (op, offset) = operators_reader.read_with_offset()?;
        validator.op(offset, &op)?;
        match op {
            Operator::End => {
                fwd.instruction(&Instruction::End);
                bwd.instruction(&Instruction::End);
            }
            Operator::LocalGet { .. } => {
                // TODO: Don't just hardcode constant return values.
            }
            Operator::F64Mul => {
                fwd.instruction(&Instruction::Call(FUNC_F64_MUL_FWD));
                bwd.instruction(&Instruction::F64Const(6.));
            }
            _ => todo!(),
        }
    }
    validator.finish(operators_reader.original_position())?;
    Ok((fwd, bwd))
}
