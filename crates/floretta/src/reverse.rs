#[cfg(test)]
mod tests;

use wasm_encoder::{
    CodeSection, Encode, ExportKind, ExportSection, Function, FunctionSection, GlobalSection,
    InstructionSink, MemorySection, Module, TypeSection,
    reencode::{Reencode, RoundtripReencoder},
};
use wasmparser::{FunctionBody, Operator, Parser, Payload};

use crate::{
    Autodiff,
    helper::{
        FUNC_F32_DIV_BWD, FUNC_F32_DIV_FWD, FUNC_F32_MUL_BWD, FUNC_F32_MUL_FWD, FUNC_F64_DIV_BWD,
        FUNC_F64_DIV_FWD, FUNC_F64_MUL_BWD, FUNC_F64_MUL_FWD, FUNC_TAPE_I32, FUNC_TAPE_I32_BWD,
        OFFSET_FUNCTIONS, OFFSET_GLOBALS, OFFSET_MEMORIES, OFFSET_TYPES, TYPE_DISPATCH,
        TYPE_F32_BIN_BWD, TYPE_F32_BIN_FWD, TYPE_F64_BIN_BWD, TYPE_F64_BIN_FWD, TYPE_TAPE_I32,
        TYPE_TAPE_I32_BWD, helpers,
    },
    util::{FuncTypes, LocalMap, TypeMap, ValType, u32_to_usize},
    validate::{FunctionValidator, ModuleValidator},
};

pub fn transform(
    mut validator: impl ModuleValidator,
    config: &Autodiff,
    wasm_module: &[u8],
) -> crate::Result<Vec<u8>> {
    let mut types = TypeSection::new();
    // Type for control flow dispatch loop in the backward pass.
    types.ty().func_type(&wasm_encoder::FuncType::new(
        [wasm_encoder::ValType::I32],
        [],
    ));
    // Type for storing a basic block index on the tape.
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
    functions.function(TYPE_TAPE_I32);
    functions.function(TYPE_TAPE_I32_BWD);
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
    let mut type_sigs = FuncTypes::new();
    let mut func_types = Vec::new();
    let mut func_infos = Vec::new();

    #[cfg(feature = "names")]
    let mut names = None;

    for payload in Parser::new(0).parse_all(wasm_module) {
        match payload? {
            Payload::TypeSection(section) => {
                validator.type_section(&section)?;
                for ty in section.into_iter_err_on_gc_types() {
                    let typeidx = type_sigs.push(ty?)?;
                    // Forward pass: same type as the original function. All the adjoint values are
                    // assumed to be zero.
                    types.ty().function(
                        type_sigs.params(typeidx).iter().map(|&ty| ty.into()),
                        type_sigs.results(typeidx).iter().map(|&ty| ty.into()),
                    );
                    // Backward pass: results become parameters, and parameters become results.
                    // Also, integers disappear from function types in the backward pass.
                    types.ty().function(
                        tuple(type_sigs.results(typeidx)),
                        tuple(type_sigs.params(typeidx)),
                    );
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
                    func_types.push(t);
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
                let index = func_infos.len().try_into().unwrap();
                let (info, fwd, bwd) = function(func, &type_sigs, &func_types, index, body)?;
                func_infos.push(info);
                code.raw(&fwd);
                code.raw(&bwd);
            }

            #[cfg(feature = "names")]
            Payload::CustomSection(section) => {
                if let wasmparser::KnownCustom::Name(reader) = section.as_known() {
                    if config.names {
                        names = Some(crate::name::Names::new(
                            (&type_sigs, func_infos.as_slice()),
                            reader,
                        )?);
                    }
                }
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

    #[cfg(feature = "names")]
    if config.names {
        module.section(&crate::name::name_section(
            (&type_sigs, func_infos.as_slice()),
            names,
        ));
    }

    Ok(module.finish())
}

/// Remove all integer types for the backward pass.
fn tuple(val_types: &[ValType]) -> Vec<wasm_encoder::ValType> {
    val_types
        .iter()
        .filter_map(|&ty| if ty.is_float() { Some(ty.into()) } else { None })
        .collect()
}

// When the `names` feature is disabled, this gets marked as dead code.
#[allow(dead_code)]
struct FunctionInfo {
    typeidx: u32,
    locals: LocalMap,
    stack_locals: StackHeight,
}

#[cfg(feature = "names")]
impl crate::name::FuncInfo for (&FuncTypes, &[FunctionInfo]) {
    fn num_functions(&self) -> u32 {
        self.1.len().try_into().unwrap()
    }

    fn num_float_results(&self, funcidx: u32) -> u32 {
        self.0
            .results(self.1[u32_to_usize(funcidx)].typeidx)
            .iter()
            .filter(|ty| ty.is_float())
            .count()
            .try_into()
            .unwrap()
    }

    fn locals(&self, funcidx: u32) -> &LocalMap {
        &self.1[u32_to_usize(funcidx)].locals
    }

    fn stack_locals(&self, funcidx: u32) -> StackHeight {
        self.1[u32_to_usize(funcidx)].stack_locals
    }
}

fn function(
    mut validator: impl FunctionValidator,
    type_sigs: &FuncTypes,
    func_types: &[u32],
    funcidx: u32,
    body: FunctionBody,
) -> crate::Result<(FunctionInfo, Vec<u8>, Vec<u8>)> {
    let typeidx = func_types[u32_to_usize(funcidx)];
    let params = type_sigs.params(typeidx);
    let num_params: u32 = params.len().try_into().unwrap();
    let num_float_results: u32 = type_sigs
        .results(typeidx)
        .iter()
        .filter(|ty| ty.is_float())
        .count()
        .try_into()
        .unwrap();
    let mut locals = LocalMap::new(TypeMap {
        i32: 0,
        i64: 0,
        f32: 1,
        f64: 1,
    });
    for &param in params {
        locals.push(1, param);
    }
    let mut locals_reader = body.get_locals_reader()?;
    for _ in 0..locals_reader.get_count() {
        let offset = locals_reader.original_position();
        let (count, ty) = locals_reader.read()?;
        validator.define_locals(offset, count, ty)?;
        locals.push(count, ValType::try_from(ty)?);
    }
    // We added a single-local entry for each parameter from the original function type, so when we
    // encode the rest of the locals, we need to skip over the parameters.
    let fwd = Function::new(locals.keys().skip(params.len()));
    let mut bwd = ReverseFunction::new(num_float_results);
    for (count, ty) in locals.vals() {
        bwd.locals(count, ty);
    }
    let tmp_f64 = bwd.local(ValType::F64);
    // The first basic block in the forward pass corresponds to the last basic block in the backward
    // pass, and because each basic block will be reversed, the first instructions we write will
    // become the last instructions in the function body of the backward pass. Because Wasm
    // parameters appear in locals but Wasm results appear on the stack, when we reach the end of
    // the backward pass, we'll have stored all the parameter adjoints in locals corresponding to
    // those from the forward pass. So, we need to push the values of those locals onto the stack in
    // order; and because everything will be reversed, that means that here we need to start with
    // pushing the local corresponding to the last parameter first, then work downward to pushing
    // the first parameter on the bottom of the stack.
    for i in (0..num_params).rev() {
        // Integer parameters disappear in the backward pass, so we skip them here.
        if let (_, Some(j)) = locals.get(i) {
            bwd.instructions(|insn| insn.local_get(num_float_results + j));
        }
    }
    let mut func = Func {
        num_float_results,
        locals,
        offset: 0, // This initial value should be unused; to be set before each instruction.
        operand_stack: Vec::new(),
        operand_stack_height: StackHeight::new(),
        operand_stack_height_min: 0,
        control_stack: Vec::new(),
        fwd,
        bwd,
        tmp_f64,
    };
    validator.check_operand_stack_height(0);
    let mut operators_reader = body.get_operators_reader()?;
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
        FunctionInfo {
            typeidx,
            locals: func.locals,
            stack_locals: func.bwd.max_stack_heights,
        },
        func.fwd.into_raw_body(),
        func.bwd.into_raw_body(&func.operand_stack),
    ))
}

struct Func {
    /// Number of floating-point results in the original function type.
    num_float_results: u32,

    /// Types of locals from the original function, and indices in the backward pass that have been
    /// mapped except for accounting for `num_results`.
    locals: LocalMap,

    /// The current byte offset in the original function body.
    offset: u32,

    operand_stack: Vec<ValType>,

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

impl Func {
    /// Process an instruction.
    fn instruction(&mut self, op: Operator<'_>) -> crate::Result<()> {
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
                    .instructions()
                    .loop_(RoundtripReencoder.block_type(blockty)?);
                self.end_basic_block();
            }
            Operator::End => match self.control_stack.pop() {
                Some(Control::Loop) => {
                    self.fwd.instructions().end();
                }
                None => {
                    self.fwd.instructions().end();
                    self.end_basic_block();
                }
            },
            Operator::BrIf { relative_depth } => {
                self.pop();
                self.fwd_control_store();
                self.end_basic_block();
                self.fwd.instructions().br_if(relative_depth);
            }
            Operator::LocalGet { local_index } => {
                let (ty, i) = self.local(local_index);
                self.push(ty);
                self.fwd.instructions().local_get(local_index);
                match ty {
                    ValType::I32 | ValType::I64 => {}
                    ValType::F32 => {
                        let i = i.unwrap();
                        self.bwd
                            .instructions(|insn| insn.local_get(i).f32_add().local_set(i));
                    }
                    ValType::F64 => {
                        let i = i.unwrap();
                        self.bwd
                            .instructions(|insn| insn.local_get(i).f64_add().local_set(i));
                    }
                }
            }
            Operator::LocalTee { local_index } => {
                let (ty, i) = self.local(local_index);
                self.pop();
                self.push(ty);
                self.fwd.instructions().local_tee(local_index);
                match ty {
                    ValType::I32 | ValType::I64 => {}
                    ValType::F32 => {
                        let i = i.unwrap();
                        self.bwd.instructions(|insn| {
                            insn.local_get(i).f32_add().f32_const(0.).local_set(i)
                        });
                    }
                    ValType::F64 => {
                        let i = i.unwrap();
                        self.bwd.instructions(|insn| {
                            insn.local_get(i).f64_add().f64_const(0.).local_set(i)
                        });
                    }
                }
            }
            Operator::I32Const { value } => {
                self.push_i32();
                self.fwd.instructions().i32_const(value);
            }
            Operator::I64Const { value } => {
                self.push_i64();
                self.fwd.instructions().i64_const(value);
            }
            Operator::F32Const { value } => {
                self.push_f32();
                self.fwd.instructions().f32_const(value.into());
                self.bwd.instructions(|insn| insn.drop());
            }
            Operator::F64Const { value } => {
                self.push_f64();
                self.fwd.instructions().f64_const(value.into());
                self.bwd.instructions(|insn| insn.drop());
            }
            Operator::I32Eqz => {
                self.pop();
                self.push_i32();
                self.fwd.instructions().i32_eqz();
            }
            Operator::I32Eq => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_eq();
            }
            Operator::I32Ne => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_ne();
            }
            Operator::I32LtS => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_lt_s();
            }
            Operator::I32LtU => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_lt_u();
            }
            Operator::I32GtS => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_gt_s();
            }
            Operator::I32GtU => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_gt_u();
            }
            Operator::I32LeS => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_le_s();
            }
            Operator::I32LeU => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_le_u();
            }
            Operator::I32GeS => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_ge_s();
            }
            Operator::I32GeU => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_ge_u();
            }
            Operator::I64Eqz => {
                self.pop();
                self.push_i64();
                self.fwd.instructions().i64_eqz();
            }
            Operator::I64Eq => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_eq();
            }
            Operator::I64Ne => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_ne();
            }
            Operator::I64LtS => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_lt_s();
            }
            Operator::I64LtU => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_lt_u();
            }
            Operator::I64GtS => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_gt_s();
            }
            Operator::I64GtU => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_gt_u();
            }
            Operator::I64LeS => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_le_s();
            }
            Operator::I64LeU => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_le_u();
            }
            Operator::I64GeS => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_ge_s();
            }
            Operator::I64GeU => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_ge_u();
            }
            Operator::F32Eq => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().f32_eq();
                self.bwd
                    .instructions(|insn| insn.f32_const(0.).f32_const(0.));
            }
            Operator::F32Ne => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().f32_ne();
                self.bwd
                    .instructions(|insn| insn.f32_const(0.).f32_const(0.));
            }
            Operator::F32Lt => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().f32_lt();
                self.bwd
                    .instructions(|insn| insn.f32_const(0.).f32_const(0.));
            }
            Operator::F32Gt => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().f32_gt();
                self.bwd
                    .instructions(|insn| insn.f32_const(0.).f32_const(0.));
            }
            Operator::F32Le => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().f32_le();
                self.bwd
                    .instructions(|insn| insn.f32_const(0.).f32_const(0.));
            }
            Operator::F32Ge => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().f32_ge();
                self.bwd
                    .instructions(|insn| insn.f32_const(0.).f32_const(0.));
            }
            Operator::F64Eq => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().f64_eq();
                self.bwd
                    .instructions(|insn| insn.f64_const(0.).f64_const(0.));
            }
            Operator::F64Ne => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().f64_ne();
                self.bwd
                    .instructions(|insn| insn.f64_const(0.).f64_const(0.));
            }
            Operator::F64Lt => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().f64_lt();
                self.bwd
                    .instructions(|insn| insn.f64_const(0.).f64_const(0.));
            }
            Operator::F64Gt => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().f64_gt();
                self.bwd
                    .instructions(|insn| insn.f64_const(0.).f64_const(0.));
            }
            Operator::F64Le => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().f64_le();
                self.bwd
                    .instructions(|insn| insn.f64_const(0.).f64_const(0.));
            }
            Operator::F64Ge => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().f64_ge();
                self.bwd
                    .instructions(|insn| insn.f64_const(0.).f64_const(0.));
            }
            Operator::I32Clz => {
                self.pop();
                self.push_i32();
                self.fwd.instructions().i32_clz();
            }
            Operator::I32Ctz => {
                self.pop();
                self.push_i32();
                self.fwd.instructions().i32_ctz();
            }
            Operator::I32Popcnt => {
                self.pop();
                self.push_i32();
                self.fwd.instructions().i32_popcnt();
            }
            Operator::I32Add => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_add();
            }
            Operator::I32Sub => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_sub();
            }
            Operator::I32Mul => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_mul();
            }
            Operator::I32DivS => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_div_s();
            }
            Operator::I32DivU => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_div_u();
            }
            Operator::I32RemS => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_rem_s();
            }
            Operator::I32RemU => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_rem_u();
            }
            Operator::I32And => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_and();
            }
            Operator::I32Or => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_or();
            }
            Operator::I32Xor => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_xor();
            }
            Operator::I32Shl => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_shl();
            }
            Operator::I32ShrS => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_shr_s();
            }
            Operator::I32ShrU => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_shr_u();
            }
            Operator::I32Rotl => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_rotl();
            }
            Operator::I32Rotr => {
                self.pop2();
                self.push_i32();
                self.fwd.instructions().i32_rotr();
            }
            Operator::I64Clz => {
                self.pop();
                self.push_i64();
                self.fwd.instructions().i64_clz();
            }
            Operator::I64Ctz => {
                self.pop();
                self.push_i64();
                self.fwd.instructions().i64_ctz();
            }
            Operator::I64Popcnt => {
                self.pop();
                self.push_i64();
                self.fwd.instructions().i64_popcnt();
            }
            Operator::I64Add => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_add();
            }
            Operator::I64Sub => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_sub();
            }
            Operator::I64Mul => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_mul();
            }
            Operator::I64DivS => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_div_s();
            }
            Operator::I64DivU => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_div_u();
            }
            Operator::I64RemS => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_rem_s();
            }
            Operator::I64RemU => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_rem_u();
            }
            Operator::I64And => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_and();
            }
            Operator::I64Or => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_or();
            }
            Operator::I64Xor => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_xor();
            }
            Operator::I64Shl => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_shl();
            }
            Operator::I64ShrS => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_shr_s();
            }
            Operator::I64ShrU => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_shr_u();
            }
            Operator::I64Rotl => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_rotl();
            }
            Operator::I64Rotr => {
                self.pop2();
                self.push_i64();
                self.fwd.instructions().i64_rotr();
            }
            Operator::F32Mul => {
                self.pop2();
                self.push_f32();
                self.fwd.instructions().call(FUNC_F32_MUL_FWD);
                self.bwd.instructions(|insn| insn.call(FUNC_F32_MUL_BWD));
            }
            Operator::F32Div => {
                self.pop2();
                self.push_f32();
                self.fwd.instructions().call(FUNC_F32_DIV_FWD);
                self.bwd.instructions(|insn| insn.call(FUNC_F32_DIV_BWD));
            }
            Operator::F64Sub => {
                self.pop2();
                self.push_f64();
                self.fwd.instructions().f64_sub();
                self.bwd.instructions(|insn| {
                    insn.local_tee(self.tmp_f64)
                        .local_get(self.tmp_f64)
                        .f64_neg()
                });
            }
            Operator::F64Mul => {
                self.pop2();
                self.push_f64();
                self.fwd.instructions().call(FUNC_F64_MUL_FWD);
                self.bwd.instructions(|insn| insn.call(FUNC_F64_MUL_BWD));
            }
            Operator::F64Div => {
                self.pop2();
                self.push_f64();
                self.fwd.instructions().call(FUNC_F64_DIV_FWD);
                self.bwd.instructions(|insn| insn.call(FUNC_F64_DIV_BWD));
            }
            _ => unimplemented!("{op:?}"),
        }
        Ok(())
    }

    fn push(&mut self, ty: ValType) {
        self.operand_stack.push(ty);
        self.operand_stack_height.push(ty);
    }

    fn push_i32(&mut self) {
        self.push(ValType::I32);
    }

    fn push_i64(&mut self) {
        self.push(ValType::I64);
    }

    fn push_f32(&mut self) {
        self.push(ValType::F32);
    }

    fn push_f64(&mut self) {
        self.push(ValType::F64);
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

    fn local(&self, index: u32) -> (ValType, Option<u32>) {
        let (ty, mapped) = self.locals.get(index);
        (ty, mapped.map(|i| self.num_float_results + i))
    }

    /// In the forward pass, store the current basic block index on the tape.
    fn fwd_control_store(&mut self) {
        self.fwd
            .instructions()
            .i32_const(self.bwd.basic_block_index())
            .call(FUNC_TAPE_I32);
    }

    fn end_basic_block(&mut self) {
        self.bwd.end_basic_block(
            self.operand_stack_height,
            &self.operand_stack[self.operand_stack_height_min..],
        );
        self.operand_stack_height_min = self.operand_stack.len();
    }
}

pub type StackHeight = TypeMap<u32>;

impl StackHeight {
    fn new() -> Self {
        Self::default()
    }

    fn counter(&mut self, ty: ValType) -> &mut u32 {
        self.get_mut(ty)
    }

    fn push(&mut self, ty: ValType) {
        *self.counter(ty) += 1;
    }

    fn pop(&mut self, ty: ValType) {
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

    fn locals(&mut self, count: u32, ty: ValType) {
        count.encode(&mut self.bytes);
        wasm_encoder::ValType::from(ty).encode(&mut self.bytes);
        self.blocks += 1;
        self.count += count;
    }

    fn local(&mut self, ty: ValType) -> u32 {
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
    stacks: Vec<ValType>,
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

    fn locals(&mut self, count: u32, ty: ValType) {
        self.locals.locals(count, ty);
    }

    fn local(&mut self, ty: ValType) -> u32 {
        self.locals.local(ty)
    }

    /// Extend the portion of the stack used by the current basic block.
    fn deepen_stack(&mut self, ty: ValType) {
        self.stacks.push(ty);
    }

    fn instructions<F>(&mut self, f: F)
    where
        for<'a, 'b> F: FnOnce(&'a mut InstructionSink<'b>) -> &'a mut InstructionSink<'b>,
    {
        reverse_encode(&mut self.body, f);
    }

    fn basic_block_index(&self) -> i32 {
        self.basic_blocks.len().try_into().unwrap()
    }

    fn end_basic_block(&mut self, height: StackHeight, stack: &[ValType]) {
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

    fn into_raw_body(mut self, operand_stack: &[ValType]) -> Vec<u8> {
        let local_count = self.locals.count();
        // When we cross a basic block boundary in the backward pass, all floating-point values on
        // the stack need to be put into locals so that they can be retrieved after the `loop`
        // dispatches to a given basic block. We've kept track of the maximum number of values in
        // the stack for each type at each basic block boundary, so now we allocate enough locals to
        // store them all.
        self.locals.locals(self.max_stack_heights.f32, ValType::F32);
        self.locals.locals(self.max_stack_heights.f64, ValType::F64);
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
    fn consume(mut self, operand_stack: &[ValType]) -> Vec<u8> {
        let mut operand_stack_height = StackHeight::new();
        // Integers disappear in the backward pass.
        for (i, &ty) in (0..).zip(operand_stack.iter().filter(|&ty| ty.is_float())) {
            self.instructions().local_get(i);
            let j = self.local_index_raw(operand_stack_height, ty).unwrap();
            self.instructions().local_set(j);
            operand_stack_height.push(ty);
        }
        let n = self.func.basic_blocks.len();
        // We don't yet support the explicit `return` instruction, so we know that the forward pass
        // exited from the last basic block with an implicit return; so, when we enter the state
        // machine for the backward pass, we know to enter at that last basic block.
        self.instructions().i32_const((n - 1).try_into().unwrap());
        let blockty = wasm_encoder::BlockType::FunctionType(TYPE_DISPATCH);
        self.instructions().loop_(blockty);
        for _ in 0..n {
            self.instructions().block(blockty);
        }
        // We insert one last `block` to give us a branch target for the error case where we somehow
        // got an invalid basic block index.
        self.instructions().block(blockty);
        // We'll put the reversed basic blocks of the backward pass in reverse order compared to the
        // original function, because the first basic block is the entrypoint to the original
        // function, so in the backward pass it becomes the sole exit point; by putting it at the
        // end, we can just do an implicit return instead of an explicit `return` instruction.
        let table: Vec<u32> = (1..=n.try_into().unwrap()).rev().collect();
        self.instructions().br_table(table, 0).end();
        // If we got an invalid basic block index, just trap immediately.
        self.instructions().unreachable();
        for i in (1..n).rev() {
            self.instructions().end();
            self.basic_block(i);
            self.instructions()
                .call(FUNC_TAPE_I32_BWD) // Load basic block index.
                .br(i.try_into().unwrap()); // Branch to the `loop`.
        }
        self.instructions().end().end();
        // First basic block goes outside the whole `loop`/`block` structure, to easily allow the
        // implicit `return`.
        self.basic_block(0);
        self.instructions().end();
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
            // Integers disappear in the backward pass.
            if let Some(i) = self.local_index(ty) {
                reverse_encode(&mut self.body, |insn| insn.local_set(i));
                // TODO: Only set stack locals to zero when they won't be overwritten later anyway.
                match ty {
                    ValType::I32 | ValType::I64 => unreachable!(),
                    ValType::F32 => reverse_encode(&mut self.body, |insn| insn.f32_const(0.)),
                    ValType::F64 => reverse_encode(&mut self.body, |insn| insn.f64_const(0.)),
                }
                reverse_encode(&mut self.body, |insn| insn.local_get(i));
            }
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
            // Integers disappear in the backward pass.
            if let Some(i) = self.local_index(ty) {
                reverse_encode(&mut self.body, |insn| insn.local_set(i));
            }
            self.operand_stack_height.push(ty);
        }
        self.body[n..].reverse();
    }

    fn instructions(&mut self) -> InstructionSink {
        InstructionSink::new(&mut self.body)
    }

    fn local_index(&self, ty: ValType) -> Option<u32> {
        self.local_index_raw(self.operand_stack_height, ty)
    }

    fn local_index_raw(&self, operand_stack_height: StackHeight, ty: ValType) -> Option<u32> {
        let i = match ty {
            ValType::I32 | ValType::I64 => return None,
            ValType::F32 => operand_stack_height.f32,
            ValType::F64 => operand_stack_height.f64 + self.func.max_stack_heights.f32,
        };
        Some(self.local_count + i)
    }
}

fn reverse_encode<F>(sink: &mut Vec<u8>, f: F)
where
    for<'a, 'b> F: FnOnce(&'a mut InstructionSink<'b>) -> &'a mut InstructionSink<'b>,
{
    let n = sink.len();
    f(&mut InstructionSink::new(sink));
    sink[n..].reverse();
}
