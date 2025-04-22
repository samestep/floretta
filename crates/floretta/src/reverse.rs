#[cfg(test)]
mod tests;

use std::ops::Sub;

use wasm_encoder::{
    reencode::{Reencode, RoundtripReencoder},
    CodeSection, Encode, ExportKind, ExportSection, Function, FunctionSection, GlobalSection,
    ImportSection, InstructionSink, MemorySection, Module, TypeSection,
};
use wasmparser::{FunctionBody, Global, Import, Operator, Parser, Payload, TypeRef};

use crate::{
    helper::{
        helper_functions, helper_globals, helper_memories, helper_types, FuncOffsets,
        OFFSET_FUNCTIONS, OFFSET_GLOBALS, OFFSET_MEMORIES, OFFSET_TYPES, TYPE_DISPATCH,
    },
    util::{u32_to_usize, BlockType, FuncTypes, LocalMap, NumImports, TwoStrs, TypeMap, ValType},
    validate::{FunctionValidator, ModuleValidator},
    Autodiff, ErrorImpl,
};

pub fn transform(
    mut validator: impl ModuleValidator,
    config: &Autodiff,
    wasm_module: &[u8],
) -> crate::Result<Vec<u8>> {
    let mut types = TypeSection::new();
    let mut imports = ImportSection::new();
    let mut functions = FunctionSection::new();
    let mut memories = MemorySection::new();
    let mut globals = GlobalSection::new();
    let mut exports = ExportSection::new();
    let mut code = CodeSection::new();
    for (_, ty) in helper_types() {
        types.ty().func_type(&ty);
    }
    for (_, memory) in helper_memories() {
        memories.memory(memory);
    }
    for (_, ty, init) in helper_globals() {
        globals.global(ty, &init);
    }
    for (_, i, f) in helper_functions() {
        functions.function(i);
        code.function(&f);
    }
    assert_eq!(types.len(), OFFSET_TYPES);
    assert_eq!(memories.len(), OFFSET_MEMORIES);
    assert_eq!(globals.len(), OFFSET_GLOBALS);
    assert_eq!(functions.len(), OFFSET_FUNCTIONS);
    assert_eq!(code.len(), OFFSET_FUNCTIONS);
    let mut type_sigs = FuncTypes::new();
    let mut num_imports = NumImports::default();
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
            Payload::ImportSection(section) => {
                validator.import_section(&section)?;
                for import in section {
                    let Import { module, name, ty } = import?;
                    let (module_bwd, name_bwd) = config
                        .imports
                        .get(&TwoStrs(module, name))
                        .ok_or_else(|| ErrorImpl::Import(module.to_string(), name.to_string()))?;
                    match ty {
                        TypeRef::Func(typeidx) => {
                            num_imports.func += 1;
                            let mapped = OFFSET_TYPES + 2 * typeidx;
                            let fwd = wasm_encoder::EntityType::Function(mapped);
                            let bwd = wasm_encoder::EntityType::Function(mapped + 1);
                            imports.import(module, name, fwd);
                            imports.import(module_bwd, name_bwd, bwd);
                            func_types.push(typeidx);
                            func_infos.push(FunctionInfo {
                                typeidx,
                                locals: LocalMap::new(type_map()),
                                stack_locals: StackHeight::new(),
                                branch_locals: StackHeight::new(),
                            });
                        }
                        TypeRef::Table(_) => unimplemented!(),
                        TypeRef::Memory(_) => unimplemented!(),
                        TypeRef::Global(_) => unimplemented!(),
                        TypeRef::Tag(_) => unimplemented!(),
                    }
                }
            }
            Payload::FunctionSection(section) => {
                validator.function_section(&section)?;
                for type_index in section {
                    let typeidx = type_index?;
                    // Index arithmetic to account for the fact that we split each original
                    // function type into two; similarly, we also split each actual function
                    // into two.
                    functions.function(OFFSET_TYPES + 2 * typeidx);
                    functions.function(OFFSET_TYPES + 2 * typeidx + 1);
                    func_types.push(typeidx);
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
                    let Global { ty, init_expr } = global?;
                    if ty.mutable {
                        unimplemented!("mutable globals");
                    }
                    if ty.shared {
                        unimplemented!("shared globals");
                    }
                    let mut ce = wasm_encoder::ConstExpr::empty();
                    let mut reader = init_expr.get_operators_reader();
                    while !reader.is_end_then_eof() {
                        match reader.read()? {
                            Operator::I32Const { value } => ce = ce.with_i32_const(value),
                            Operator::I64Const { value } => ce = ce.with_i64_const(value),
                            Operator::F32Const { value } => ce = ce.with_f32_const(value.into()),
                            Operator::F64Const { value } => ce = ce.with_f64_const(value.into()),
                            op => unimplemented!("{op:?}"),
                        };
                    }
                    globals.global(
                        wasm_encoder::GlobalType {
                            val_type: ValType::try_from(ty.content_type)?.into(),
                            mutable: false,
                            shared: false,
                        },
                        &ce,
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
                            let mut funcidx = 2 * e.index;
                            if e.index >= num_imports.func {
                                funcidx += OFFSET_FUNCTIONS;
                            }
                            exports.export(e.name, kind, funcidx);
                            if let Some(name) = config.exports.get(e.name) {
                                exports.export(name, kind, funcidx + 1);
                            }
                        }
                        ExportKind::Memory => {
                            let memidx = OFFSET_MEMORIES + 2 * e.index;
                            exports.export(e.name, kind, memidx);
                            if let Some(name) = config.exports.get(e.name) {
                                exports.export(name, kind, memidx + 1);
                            }
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
                let (info, fwd, bwd) =
                    function(func, &type_sigs, num_imports, &func_types, index, body)?;
                func_infos.push(info);
                code.raw(&fwd);
                code.raw(&bwd);
            }

            #[cfg(feature = "names")]
            Payload::CustomSection(section) => {
                if let wasmparser::KnownCustom::Name(reader) = section.as_known() {
                    if config.names {
                        names = Some(crate::name::Names::new(
                            (&type_sigs, num_imports, func_infos.as_slice()),
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
    module.section(&imports);
    module.section(&functions);
    module.section(&memories);
    module.section(&globals);
    module.section(&exports);
    module.section(&code);

    #[cfg(feature = "names")]
    if config.names {
        module.section(&crate::name::name_section(
            (&type_sigs, num_imports, func_infos.as_slice()),
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

fn type_map() -> TypeMap<u32> {
    TypeMap {
        i32: 0,
        i64: 0,
        f32: 1,
        f64: 1,
    }
}
// When the `names` feature is disabled, this gets marked as dead code.
#[allow(dead_code)]
struct FunctionInfo {
    typeidx: u32,
    locals: LocalMap,
    stack_locals: StackHeight,
    branch_locals: StackHeight,
}

#[cfg(feature = "names")]
impl crate::name::FuncInfo for (&FuncTypes, NumImports, &[FunctionInfo]) {
    fn num_imports(&self) -> NumImports {
        self.1
    }

    fn num_functions(&self) -> u32 {
        self.2.len().try_into().unwrap()
    }

    fn num_float_results(&self, funcidx: u32) -> u32 {
        self.0
            .results(self.2[u32_to_usize(funcidx)].typeidx)
            .iter()
            .filter(|ty| ty.is_float())
            .count()
            .try_into()
            .unwrap()
    }

    fn locals(&self, funcidx: u32) -> &LocalMap {
        &self.2[u32_to_usize(funcidx)].locals
    }

    fn stack_locals(&self, funcidx: u32) -> StackHeight {
        self.2[u32_to_usize(funcidx)].stack_locals
    }

    fn branch_locals(&self, funcidx: u32) -> StackHeight {
        self.2[u32_to_usize(funcidx)].branch_locals
    }
}

fn function(
    mut validator: impl FunctionValidator,
    type_sigs: &FuncTypes,
    num_imports: NumImports,
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
    let mut locals = LocalMap::new(type_map());
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
    let (tmp_f32_fwd, tmp_f32_bwd) = (locals.count_keys(), num_float_results + locals.count_vals());
    locals.push(1, ValType::F32);
    let (tmp_f64_fwd, tmp_f64_bwd) = (locals.count_keys(), num_float_results + locals.count_vals());
    locals.push(1, ValType::F64);
    let tmp_i32_fwd = locals.count_keys();
    locals.push(1, ValType::I32);
    // We added a single-local entry for each parameter from the original function type, so when we
    // encode the rest of the locals, we need to skip over the parameters.
    let fwd = Function::new(locals.keys().skip(params.len()));
    let mut bwd = ReverseFunction::new(num_imports, num_float_results);
    for (count, ty) in locals.vals() {
        bwd.locals(count, ty);
    }
    let tmp_i32_bwd = bwd.local(ValType::I32);
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
        type_sigs,
        num_imports,
        func_types,
        num_float_results,
        locals,
        offset: 0, // This initial value should be unused; to be set before each instruction.
        operand_stack: Vec::new(),
        operand_stack_height: StackHeight::new(),
        operand_stack_height_min: 0,
        control_stack: vec![Control::Block(BlockType::Func(typeidx))],
        fwd,
        bwd,
        tmp_i32_fwd,
        tmp_f32_fwd,
        tmp_f64_fwd,
        tmp_i32_bwd,
        tmp_f32_bwd,
        tmp_f64_bwd,
    };
    validator.check_operand_stack_height(0);
    validator.check_control_stack_height(1);
    let mut operators_reader = body.get_operators_reader()?;
    while !operators_reader.eof() {
        let (op, offset) = operators_reader.read_with_offset()?;
        validator.op(offset, &op)?;
        func.offset = offset.try_into().unwrap();
        func.instruction(op)?;
        let operand_stack_height = func.operand_stack.len().try_into().unwrap();
        let control_stack_height = func.control_stack.len().try_into().unwrap();
        validator.check_operand_stack_height(operand_stack_height);
        validator.check_control_stack_height(control_stack_height);
        assert_eq!(func.operand_stack_height.sum(), operand_stack_height);
    }
    validator.finish(operators_reader.original_position())?;
    Ok((
        FunctionInfo {
            typeidx,
            locals: func.locals,
            stack_locals: func.bwd.max_stack_values,
            branch_locals: func.bwd.max_branch_values,
        },
        func.fwd.into_raw_body(),
        func.bwd.into_raw_body(&func.operand_stack),
    ))
}

struct Func<'a> {
    /// All type signatures in the module.
    type_sigs: &'a FuncTypes,

    /// Number of imports in the module.
    num_imports: NumImports,

    /// Type indices for all the functions in the module.
    func_types: &'a [u32],

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

    /// Local index for an `f32` in the forward pass.
    tmp_f32_fwd: u32,

    /// Local index for an `f64` in the forward pass.
    tmp_f64_fwd: u32,

    /// Local index for an `i32` in the forward pass.
    tmp_i32_fwd: u32,

    /// Local index for an `f32` in the backward pass.
    tmp_f32_bwd: u32,

    /// Local index for an `f64` in the backward pass.
    tmp_f64_bwd: u32,

    /// Local index for an `i32` in the backward pass.
    tmp_i32_bwd: u32,
}

impl<'a> Func<'a> {
    /// Process an instruction.
    fn instruction(&mut self, op: Operator<'_>) -> crate::Result<()> {
        let helper = self.helpers();
        match op {
            Operator::Loop { blockty } => {
                let block_type = BlockType::try_from(blockty)?;
                self.control_stack.push(Control::Loop(block_type));
                self.fwd_control_store();
                let reencoded = self.blockty(block_type);
                self.fwd.instructions().loop_(reencoded);
                self.split_basic_block_with_params(block_type);
            }
            Operator::If { blockty } => {
                self.pop();
                let block_type = BlockType::try_from(blockty)?;
                let control = Control::If {
                    block_type,
                    stack_height: self.operand_stack_height.sum(),
                };
                self.control_stack.push(control);
                self.fwd_control_store();
                let reencoded = self.blockty(block_type);
                self.fwd.instructions().if_(reencoded);
                self.split_basic_block_with_params(block_type);
            }
            Operator::Else => {
                self.fwd_control_store();
                self.fwd.instructions().else_();
                match self.control_stack.last().unwrap() {
                    &Control::If {
                        block_type,
                        stack_height,
                    } => {
                        let branch_values = self.blockty_results(block_type);
                        let branch_values_next = self.blockty_params(block_type);
                        self.split_basic_block(branch_values, stack_height, branch_values_next);
                    }
                    _ => unreachable!(),
                }
            }
            Operator::End => match self.control_stack.pop().unwrap() {
                Control::Block(block_type) => {
                    self.fwd_control_store();
                    self.fwd.instructions().end();
                    if self.control_stack.is_empty() {
                        // This means we've reached the end of the function body, so we need to not
                        // try to start another basic block after this one.
                        let branch_values = self.blockty_results(block_type);
                        let current_stack_height = self.operand_stack_height.sum();
                        self.split_basic_block(branch_values, current_stack_height, &[]);
                    } else {
                        self.split_basic_block_with_results(block_type);
                    }
                }
                Control::Loop(_) => {
                    self.fwd.instructions().end();
                }
                Control::If {
                    block_type,
                    stack_height: _,
                } => {
                    self.fwd_control_store();
                    self.fwd.instructions().end();
                    self.split_basic_block_with_results(block_type);
                }
            },
            Operator::Br { relative_depth } => {
                self.fwd_control_store();
                self.fwd.instructions().br(relative_depth);
                let branch_values = self.branch_values(relative_depth);
                let current_stack_height = self.operand_stack_height.sum();
                let stack_reset =
                    current_stack_height - u32::try_from(branch_values.len()).unwrap();
                self.split_basic_block(branch_values, stack_reset, &[]);
            }
            Operator::BrIf { relative_depth } => {
                self.pop();
                self.fwd_control_store();
                self.fwd.instructions().br_if(relative_depth);
                let branch_values = self.branch_values(relative_depth);
                self.split_basic_block_fallthrough(branch_values);
            }
            Operator::Call { function_index } => {
                let typeidx = self.func_types[u32_to_usize(function_index)];
                for _ in self.type_sigs.params(typeidx) {
                    self.pop();
                }
                for &result in self.type_sigs.results(typeidx) {
                    self.push(result);
                }
                let (fwd, bwd) = self.func(function_index);
                self.fwd.instructions().call(fwd);
                self.bwd.instructions(|insn| insn.call(bwd));
            }
            Operator::Drop => {
                let ty = self.pop();
                self.fwd.instructions().drop();
                match ty {
                    ValType::I32 | ValType::I64 => {}
                    ValType::F32 => self.bwd.instructions(|insn| insn.f32_const(0.)),
                    ValType::F64 => self.bwd.instructions(|insn| insn.f64_const(0.)),
                }
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
            Operator::LocalSet { local_index } => {
                let (ty, i) = self.local(local_index);
                self.pop();
                self.fwd.instructions().local_set(local_index);
                match ty {
                    ValType::I32 | ValType::I64 => {}
                    ValType::F32 => {
                        let i = i.unwrap();
                        self.bwd
                            .instructions(|insn| insn.local_get(i).f32_const(0.).local_set(i));
                    }
                    ValType::F64 => {
                        let i = i.unwrap();
                        self.bwd
                            .instructions(|insn| insn.local_get(i).f64_const(0.).local_set(i));
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
            Operator::F32Load { memarg } => {
                self.pop();
                self.push_f32();
                let (fwd, bwd) = self.memarg(memarg);
                self.fwd
                    .instructions()
                    .local_tee(self.tmp_i32_fwd)
                    .call(helper.tape_i32())
                    .local_get(self.tmp_i32_fwd)
                    .f32_load(fwd);
                self.bwd.instructions(|insn| {
                    insn.local_set(self.tmp_f32_bwd)
                        .call(helper.tape_i32_bwd())
                        .local_tee(self.tmp_i32_bwd)
                        .local_get(self.tmp_i32_bwd)
                        .f32_load(bwd)
                        .local_get(self.tmp_f32_bwd)
                        .f32_add()
                        .f32_store(bwd)
                });
            }
            Operator::F64Load { memarg } => {
                self.pop();
                self.push_f64();
                let (fwd, bwd) = self.memarg(memarg);
                self.fwd
                    .instructions()
                    .local_tee(self.tmp_i32_fwd)
                    .call(helper.tape_i32())
                    .local_get(self.tmp_i32_fwd)
                    .f64_load(fwd);
                self.bwd.instructions(|insn| {
                    insn.local_set(self.tmp_f64_bwd)
                        .call(helper.tape_i32_bwd())
                        .local_tee(self.tmp_i32_bwd)
                        .local_get(self.tmp_i32_bwd)
                        .f64_load(bwd)
                        .local_get(self.tmp_f64_bwd)
                        .f64_add()
                        .f64_store(bwd)
                });
            }
            Operator::F32Store { memarg } => {
                self.pop2();
                let (fwd, bwd) = self.memarg(memarg);
                self.fwd
                    .instructions()
                    .local_set(self.tmp_f32_fwd)
                    .local_tee(self.tmp_i32_fwd)
                    .call(helper.tape_i32())
                    .local_get(self.tmp_i32_fwd)
                    .local_get(self.tmp_f32_fwd)
                    .f32_store(fwd);
                self.bwd.instructions(|insn| {
                    insn.call(helper.tape_i32_bwd())
                        .local_tee(self.tmp_i32_bwd)
                        .f32_load(bwd)
                        .local_get(self.tmp_i32_bwd)
                        .f32_const(0.)
                        .f32_store(bwd)
                });
            }
            Operator::F64Store { memarg } => {
                self.pop2();
                let (fwd, bwd) = self.memarg(memarg);
                self.fwd
                    .instructions()
                    .local_set(self.tmp_f64_fwd)
                    .local_tee(self.tmp_i32_fwd)
                    .call(helper.tape_i32())
                    .local_get(self.tmp_i32_fwd)
                    .local_get(self.tmp_f64_fwd)
                    .f64_store(fwd);
                self.bwd.instructions(|insn| {
                    insn.call(helper.tape_i32_bwd())
                        .local_tee(self.tmp_i32_bwd)
                        .f64_load(bwd)
                        .local_get(self.tmp_i32_bwd)
                        .f64_const(0.)
                        .f64_store(bwd)
                });
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
            Operator::F32Neg => {
                self.pop();
                self.push_f32();
                self.fwd.instructions().f32_neg();
                self.bwd.instructions(|insn| insn.f32_neg());
            }
            Operator::F32Sqrt => {
                self.pop();
                self.push_f32();
                self.fwd.instructions().call(helper.f32_sqrt_fwd());
                self.bwd
                    .instructions(|insn| insn.call(helper.f32_sqrt_bwd()));
            }
            Operator::F32Add => {
                self.pop2();
                self.push_f32();
                self.fwd.instructions().f32_add();
                self.bwd.instructions(|insn| {
                    insn.local_tee(self.tmp_f32_bwd).local_get(self.tmp_f32_bwd)
                });
            }
            Operator::F32Sub => {
                self.pop2();
                self.push_f32();
                self.fwd.instructions().f32_sub();
                self.bwd.instructions(|insn| {
                    insn.local_tee(self.tmp_f32_bwd)
                        .local_get(self.tmp_f32_bwd)
                        .f32_neg()
                });
            }
            Operator::F32Mul => {
                self.pop2();
                self.push_f32();
                self.fwd.instructions().call(helper.f32_mul_fwd());
                self.bwd
                    .instructions(|insn| insn.call(helper.f32_mul_bwd()));
            }
            Operator::F32Div => {
                self.pop2();
                self.push_f32();
                self.fwd.instructions().call(helper.f32_div_fwd());
                self.bwd
                    .instructions(|insn| insn.call(helper.f32_div_bwd()));
            }
            Operator::F32Min => {
                self.pop2();
                self.push_f32();
                self.fwd.instructions().call(helper.f32_min_fwd());
                self.bwd
                    .instructions(|insn| insn.call(helper.f32_min_bwd()));
            }
            Operator::F32Max => {
                self.pop2();
                self.push_f32();
                self.fwd.instructions().call(helper.f32_max_fwd());
                self.bwd
                    .instructions(|insn| insn.call(helper.f32_max_bwd()));
            }
            Operator::F32Copysign => {
                self.pop2();
                self.push_f32();
                self.fwd.instructions().call(helper.f32_copysign_fwd());
                self.bwd
                    .instructions(|insn| insn.call(helper.f32_copysign_bwd()));
            }
            Operator::F64Neg => {
                self.pop();
                self.push_f64();
                self.fwd.instructions().f64_neg();
                self.bwd.instructions(|insn| insn.f64_neg());
            }
            Operator::F64Sqrt => {
                self.pop();
                self.push_f64();
                self.fwd.instructions().call(helper.f64_sqrt_fwd());
                self.bwd
                    .instructions(|insn| insn.call(helper.f64_sqrt_bwd()));
            }
            Operator::F64Add => {
                self.pop2();
                self.push_f64();
                self.fwd.instructions().f64_add();
                self.bwd.instructions(|insn| {
                    insn.local_tee(self.tmp_f64_bwd).local_get(self.tmp_f64_bwd)
                });
            }
            Operator::F64Sub => {
                self.pop2();
                self.push_f64();
                self.fwd.instructions().f64_sub();
                self.bwd.instructions(|insn| {
                    insn.local_tee(self.tmp_f64_bwd)
                        .local_get(self.tmp_f64_bwd)
                        .f64_neg()
                });
            }
            Operator::F64Mul => {
                self.pop2();
                self.push_f64();
                self.fwd.instructions().call(helper.f64_mul_fwd());
                self.bwd
                    .instructions(|insn| insn.call(helper.f64_mul_bwd()));
            }
            Operator::F64Div => {
                self.pop2();
                self.push_f64();
                self.fwd.instructions().call(helper.f64_div_fwd());
                self.bwd
                    .instructions(|insn| insn.call(helper.f64_div_bwd()));
            }
            Operator::F64Min => {
                self.pop2();
                self.push_f64();
                self.fwd.instructions().call(helper.f64_min_fwd());
                self.bwd
                    .instructions(|insn| insn.call(helper.f64_min_bwd()));
            }
            Operator::F64Max => {
                self.pop2();
                self.push_f64();
                self.fwd.instructions().call(helper.f64_max_fwd());
                self.bwd
                    .instructions(|insn| insn.call(helper.f64_max_bwd()));
            }
            Operator::F64Copysign => {
                self.pop2();
                self.push_f64();
                self.fwd.instructions().call(helper.f64_copysign_fwd());
                self.bwd
                    .instructions(|insn| insn.call(helper.f64_copysign_bwd()));
            }
            Operator::F32ConvertI32S => {
                self.pop();
                self.push_f32();
                self.fwd.instructions().f32_convert_i32_s();
                self.bwd.instructions(|insn| insn.drop());
            }
            Operator::F32ConvertI32U => {
                self.pop();
                self.push_f32();
                self.fwd.instructions().f32_convert_i32_u();
                self.bwd.instructions(|insn| insn.drop());
            }
            Operator::F32ConvertI64S => {
                self.pop();
                self.push_f32();
                self.fwd.instructions().f32_convert_i64_s();
                self.bwd.instructions(|insn| insn.drop());
            }
            Operator::F32ConvertI64U => {
                self.pop();
                self.push_f32();
                self.fwd.instructions().f32_convert_i64_u();
                self.bwd.instructions(|insn| insn.drop());
            }
            Operator::F64ConvertI32S => {
                self.pop();
                self.push_f64();
                self.fwd.instructions().f64_convert_i32_s();
                self.bwd.instructions(|insn| insn.drop());
            }
            Operator::F64ConvertI32U => {
                self.pop();
                self.push_f64();
                self.fwd.instructions().f64_convert_i32_u();
                self.bwd.instructions(|insn| insn.drop());
            }
            Operator::F64ConvertI64S => {
                self.pop();
                self.push_f64();
                self.fwd.instructions().f64_convert_i64_s();
                self.bwd.instructions(|insn| insn.drop());
            }
            Operator::F64ConvertI64U => {
                self.pop();
                self.push_f64();
                self.fwd.instructions().f64_convert_i64_u();
                self.bwd.instructions(|insn| insn.drop());
            }
            _ => unimplemented!("{op:?}"),
        }
        Ok(())
    }

    fn blockty_params(&self, block_type: BlockType) -> &'a [ValType] {
        match block_type {
            BlockType::Empty | BlockType::Result(_) => &[],
            BlockType::Func(typeidx) => self.type_sigs.params(typeidx),
        }
    }

    fn blockty_results(&self, block_type: BlockType) -> &'a [ValType] {
        match block_type {
            BlockType::Empty => &[],
            BlockType::Result(val_type) => val_type.singleton(),
            BlockType::Func(typeidx) => self.type_sigs.results(typeidx),
        }
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

    fn pop(&mut self) -> ValType {
        let ty = self.operand_stack.pop().unwrap();
        self.operand_stack_height.pop(ty);
        let n = self.operand_stack.len();
        if n < self.operand_stack_height_min {
            assert_eq!(self.operand_stack_height_min, n + 1);
            self.bwd.deepen_stack(ty);
            self.operand_stack_height_min = n;
        }
        ty
    }

    fn pop2(&mut self) {
        self.pop();
        self.pop();
    }

    fn blockty(&self, block_type: BlockType) -> wasm_encoder::BlockType {
        match block_type {
            BlockType::Empty => wasm_encoder::BlockType::Empty,
            BlockType::Result(val_type) => wasm_encoder::BlockType::Result(val_type.into()),
            BlockType::Func(typeidx) => wasm_encoder::BlockType::FunctionType(2 * typeidx),
        }
    }

    fn helpers(&self) -> FuncOffsets {
        FuncOffsets::new(self.num_imports)
    }

    fn func(&self, funcidx: u32) -> (u32, u32) {
        let mut fwd = 2 * funcidx;
        if funcidx >= self.num_imports.func {
            fwd += OFFSET_FUNCTIONS;
        }
        let bwd = fwd + 1;
        (fwd, bwd)
    }

    fn local(&self, index: u32) -> (ValType, Option<u32>) {
        let (ty, mapped) = self.locals.get(index);
        (ty, mapped.map(|i| self.num_float_results + i))
    }

    fn memarg(&self, memarg: wasmparser::MemArg) -> (wasm_encoder::MemArg, wasm_encoder::MemArg) {
        let mut fwd = RoundtripReencoder.mem_arg(memarg);
        fwd.memory_index = OFFSET_MEMORIES + 2 * fwd.memory_index;
        let mut bwd = fwd;
        bwd.memory_index += 1;
        (fwd, bwd)
    }

    /// In the forward pass, store the current basic block index on the tape.
    fn fwd_control_store(&mut self) {
        let helper = self.helpers();
        self.fwd
            .instructions()
            .i32_const(self.bwd.basic_block_index())
            .call(helper.tape_i32());
    }

    fn branch_values(&self, relative_depth: u32) -> &'a [ValType] {
        match self.control_stack[self.control_stack.len() - 1 - u32_to_usize(relative_depth)] {
            Control::Block(block_type) => self.blockty_results(block_type),
            Control::Loop(block_type) => self.blockty_params(block_type),
            Control::If {
                block_type,
                stack_height: _,
            } => self.blockty_results(block_type),
        }
    }

    fn split_basic_block(
        &mut self,
        branch_values: &[ValType],
        stack_reset: u32,
        branch_values_next: &[ValType],
    ) {
        for _ in branch_values {
            self.pop();
        }
        for &ty in branch_values {
            self.push(ty);
        }
        let stack_end = &self.operand_stack[self.operand_stack_height_min..];
        let stack_height_end = self.operand_stack_height;
        let branch_end_count = branch_values.len().try_into().unwrap();
        let branch_start_count = branch_values_next.len().try_into().unwrap();
        self.bwd.split_basic_block(
            stack_end,
            stack_height_end,
            branch_end_count,
            branch_start_count,
        );
        while self.operand_stack.len() > u32_to_usize(stack_reset) {
            let ty = self.operand_stack.pop().unwrap();
            self.operand_stack_height.pop(ty);
        }
        self.operand_stack_height_min = self.operand_stack.len();
        for _ in branch_values_next {
            self.pop();
        }
        for &ty in branch_values_next {
            self.push(ty);
        }
    }

    fn split_basic_block_fallthrough(&mut self, branch_values: &[ValType]) {
        let current_stack_height = self.operand_stack_height.sum();
        self.split_basic_block(branch_values, current_stack_height, branch_values);
    }

    fn split_basic_block_with_params(&mut self, block_type: BlockType) {
        let branch_values = self.blockty_params(block_type);
        self.split_basic_block_fallthrough(branch_values);
    }

    fn split_basic_block_with_results(&mut self, block_type: BlockType) {
        let branch_values = self.blockty_results(block_type);
        self.split_basic_block_fallthrough(branch_values);
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

impl Sub for StackHeight {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            i32: self.i32 - rhs.i32,
            i64: self.i64 - rhs.i64,
            f32: self.f32 - rhs.f32,
            f64: self.f64 - rhs.f64,
        }
    }
}

#[derive(Clone, Copy)]
enum Control {
    Block(BlockType),
    Loop(BlockType),
    If {
        block_type: BlockType,
        stack_height: u32,
    },
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

#[derive(Clone, Copy, Debug)]
struct BasicBlock {
    /// Offset of the first body instruction byte for this basic block.
    start_offset: u32,

    /// Start index of the list of the types of operands that were on the stack before this basic
    /// block but popped from the stack during this basic block.
    stack_start_offset: u32,

    /// Start index of the list of the types of operands on the stack after this basic block that
    /// were pushed to the stack during this basic block.
    stack_end_offset: u32,

    /// Stack heights at the end of this basic block.
    stack_height_end: StackHeight,

    /// Number of values on the top of the stack at the end of this basic block that could have come
    /// from a branch instruction, from the perspective of the following block. This depends on the
    /// instruction at the end of the basic block.
    ///
    /// - `end` of a `block`, or branch to a `block` or `if`: number of results in the block type.
    /// - `loop`, or branch to a `loop`: number of parameters in the block type.
    branch_start_count: u32,

    /// Number of values on the top of the stack at the end of this basic block that could have come
    /// from a branch instruction, from the perspective of the following block. This depends on the
    /// instruction at the end of the basic block.
    ///
    /// - `end` of a `block`, or branch to a `block` or `if`: number of results in the block type.
    /// - `loop`, or branch to a `loop`: number of parameters in the block type.
    branch_end_count: u32,
}

struct ReverseFunction {
    num_imports: NumImports,
    locals: Locals,
    body: Vec<u8>,
    stacks: Vec<ValType>,
    basic_blocks: Vec<BasicBlock>,
    block_start_offset: usize,
    block_stack_offset: usize,
    branch_start_count: u32,
    max_stack_values: StackHeight,
    max_branch_values: StackHeight,
}

impl ReverseFunction {
    fn new(num_imports: NumImports, params: u32) -> Self {
        Self {
            num_imports,
            locals: Locals::new(params),
            body: Vec::new(),
            stacks: Vec::new(),
            basic_blocks: Vec::new(),
            block_start_offset: 0,
            block_stack_offset: 0,
            branch_start_count: 0,
            max_stack_values: StackHeight::new(),
            max_branch_values: StackHeight::new(),
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

    fn split_basic_block(
        &mut self,
        stack_end: &[ValType],
        stack_height_end: StackHeight,
        branch_end_count: u32,
        branch_start_count: u32,
    ) {
        self.body[self.block_start_offset..].reverse();
        let stack_end_offset = self.stacks.len().try_into().unwrap();
        self.stacks.extend_from_slice(stack_end);
        self.basic_blocks.push(BasicBlock {
            start_offset: self.block_start_offset.try_into().unwrap(),
            stack_start_offset: self.block_stack_offset.try_into().unwrap(),
            stack_end_offset,
            stack_height_end,
            branch_start_count: self.branch_start_count,
            branch_end_count,
        });
        self.block_start_offset = self.body.len();
        self.block_stack_offset = self.stacks.len();
        self.branch_start_count = branch_start_count;
        let mut branch_values = StackHeight::new();
        for &ty in &stack_end[stack_end.len() - u32_to_usize(branch_end_count)..] {
            branch_values.push(ty);
        }
        // We keep track of the maximum stack values so that we can later allocate enough locals for
        // all of them, but these "branch values" at the top of the stack are going to use a
        // different set of locals. So, we subtract them off before folding this into our running
        // maximum.
        self.max_stack_values
            .take_max(stack_height_end - branch_values);
        self.max_branch_values.take_max(branch_values);
    }

    fn into_raw_body(mut self, operand_stack: &[ValType]) -> Vec<u8> {
        let stack_local_offset = self.locals.count();
        // When we cross a basic block boundary in the backward pass, all floating-point values on
        // the stack need to be put into locals so that they can be retrieved after the `loop`
        // dispatches to a given basic block. We've kept track of the maximum number of values in
        // the stack for each type at each basic block boundary, so now we allocate enough locals to
        // store them all.
        self.locals.locals(self.max_stack_values.f32, ValType::F32);
        self.locals.locals(self.max_stack_values.f64, ValType::F64);
        let branch_local_offset = self.locals.count();
        // Typically stack values just go into the stack locals we just created, but for
        // branch-related instructions involving block types, some values need to go into these
        // branch locals instead. Conceptually, the stack locals represent the "bottom of the stack"
        // since they stay put as things may change above them, whereas the branch locals represent
        // the "top of the stack" since they may be passed around by branch instructions but their
        // absolute position in the stack depends on control flow.
        self.locals.locals(self.max_branch_values.f32, ValType::F32);
        self.locals.locals(self.max_branch_values.f64, ValType::F64);
        let mut body = Vec::new();
        self.locals.blocks().encode(&mut body);
        body.extend_from_slice(self.locals.bytes());
        ReverseReverseFunction {
            func: self,
            stack_local_offset,
            branch_local_offset,
            body,
        }
        .consume(operand_stack)
    }
}

struct ReverseReverseFunction {
    func: ReverseFunction,
    stack_local_offset: u32,
    branch_local_offset: u32,
    body: Vec<u8>,
}

impl ReverseReverseFunction {
    fn consume(mut self, operand_stack: &[ValType]) -> Vec<u8> {
        let helper = FuncOffsets::new(self.func.num_imports);
        let mut return_values = StackHeight::new();
        // Integers disappear in the backward pass.
        for (i, &ty) in (0..).zip(operand_stack.iter().filter(|&ty| ty.is_float())) {
            self.instructions().local_get(i);
            let j = self.branch_local_index(return_values, ty).unwrap();
            self.instructions().local_set(j);
            return_values.push(ty);
        }
        let n = self.func.basic_blocks.len();
        // The forward pass stores the basic block index before any implicit or explicit return, so
        // we load it here to determine which basic block to start with in the backward pass.
        self.instructions().call(helper.tape_i32_bwd());
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
                .call(helper.tape_i32_bwd()) // Load basic block index.
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
        let mut stack_values = bb.stack_height_end;
        // Some number of values on the top of the stack may need to be specially treated due to
        // branch instructions; when we finished processing this basic block earlier, we stored the
        // number of such values. Unlike our operand stack height bookkeeping which measures from
        // the bottom of the stack, these more ephemeral values measure from the top of the stack,
        // so they can just be initialized to zero here.
        let mut branch_values = StackHeight::new();
        for &ty in self.func.stacks[stack_mid..stack_end].iter().rev() {
            stack_values.pop(ty);
            let local_index = if branch_values.sum() < bb.branch_end_count {
                let li = self.branch_local_index(branch_values, ty);
                branch_values.push(ty);
                li
            } else {
                self.stack_local_index(stack_values, ty)
            };
            // Integers disappear in the backward pass.
            if let Some(i) = local_index {
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
        let mut branch_values = StackHeight::new();
        for &ty in self.func.stacks[stack_start..stack_mid].iter().rev() {
            let local_index = if branch_values.sum() < bb.branch_start_count {
                let li = self.branch_local_index(branch_values, ty);
                branch_values.push(ty);
                li
            } else {
                self.stack_local_index(stack_values, ty)
            };
            // Integers disappear in the backward pass.
            if let Some(i) = local_index {
                reverse_encode(&mut self.body, |insn| insn.local_set(i));
            }
            stack_values.push(ty);
        }
        self.body[n..].reverse();
    }

    fn instructions(&mut self) -> InstructionSink {
        InstructionSink::new(&mut self.body)
    }

    fn stack_local_index(&self, stack_values: StackHeight, ty: ValType) -> Option<u32> {
        let i = match ty {
            ValType::I32 | ValType::I64 => return None,
            ValType::F32 => stack_values.f32,
            ValType::F64 => stack_values.f64 + self.func.max_stack_values.f32,
        };
        Some(self.stack_local_offset + i)
    }

    fn branch_local_index(&self, branch_values: StackHeight, ty: ValType) -> Option<u32> {
        let i = match ty {
            ValType::I32 | ValType::I64 => return None,
            ValType::F32 => branch_values.f32,
            ValType::F64 => branch_values.f64 + self.func.max_branch_values.f32,
        };
        Some(self.branch_local_offset + i)
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
