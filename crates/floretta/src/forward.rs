use wasm_encoder::{
    CodeSection, ExportSection, Function, FunctionSection, InstructionSink, Module, TypeSection,
    reencode::{Reencode, RoundtripReencoder},
};
use wasmparser::{FunctionBody, Operator, Parser, Payload, Validator, WasmFeatures};

use crate::{
    NoValidate, Validate,
    util::u32_to_usize,
    validate::{FunctionValidator, ModuleValidator},
};

#[derive(Debug, Default)]
pub struct Config {}

pub trait ForwardTransform {
    fn transform(&self, config: &Config, wasm_module: &[u8]) -> crate::Result<Vec<u8>>;
}

// We make `ForwardTransform` a `trait` instead of just an `enum`, to facilitate dead code elimination
// when validation is not needed.

impl ForwardTransform for Validate {
    fn transform(&self, config: &Config, wasm_module: &[u8]) -> crate::Result<Vec<u8>> {
        let features = WasmFeatures::empty() | WasmFeatures::FLOATS;
        let validator = Validator::new_with_features(features);
        transform(validator, config, wasm_module)
    }
}

impl ForwardTransform for NoValidate {
    fn transform(&self, config: &Config, wasm_module: &[u8]) -> crate::Result<Vec<u8>> {
        transform((), config, wasm_module)
    }
}

fn transform(
    mut validator: impl ModuleValidator,
    _: &Config,
    wasm_module: &[u8],
) -> crate::Result<Vec<u8>> {
    let mut types = TypeSection::new();
    let mut functions = FunctionSection::new();
    let mut exports = ExportSection::new();
    let mut code = CodeSection::new();
    let mut type_sigs = Vec::new();
    let mut func_types = Vec::new();
    let mut num_bodies = 0;
    for payload in Parser::new(0).parse_all(wasm_module) {
        match payload? {
            Payload::TypeSection(section) => {
                validator.type_section(&section)?;
                for func_ty in section.into_iter_err_on_gc_types() {
                    let ty = func_ty?;
                    types
                        .ty()
                        .function(tuple(ty.params())?, tuple(ty.results())?);
                    type_sigs.push(ty);
                }
            }
            Payload::FunctionSection(section) => {
                validator.function_section(&section)?;
                for type_index in section {
                    let t = type_index?;
                    functions.function(t);
                    func_types.push(t);
                }
            }
            Payload::ExportSection(section) => {
                validator.export_section(&section)?;
                RoundtripReencoder.parse_export_section(&mut exports, section)?;
            }
            Payload::CodeSectionEntry(body) => {
                let func = validator.code_section_entry(&body)?;
                code.function(&function(func, &type_sigs[num_bodies], body)?);
                num_bodies += 1;
            }
            other => validator.payload(&other)?,
        }
    }
    let mut module = Module::new();
    module.section(&types);
    module.section(&functions);
    module.section(&exports);
    module.section(&code);
    Ok(module.finish())
}

fn tuple(val_types: &[wasmparser::ValType]) -> crate::Result<Vec<wasm_encoder::ValType>> {
    let mut types = Vec::new();
    for ty in val_types {
        match ty {
            wasmparser::ValType::I32 | wasmparser::ValType::I64 => {
                types.push(RoundtripReencoder.val_type(*ty)?);
            }
            wasmparser::ValType::F32 | wasmparser::ValType::F64 => {
                let reencoded = RoundtripReencoder.val_type(*ty)?;
                types.push(reencoded);
                types.push(reencoded);
            }
            wasmparser::ValType::V128 => unimplemented!(),
            wasmparser::ValType::Ref(_) => unimplemented!(),
        }
    }
    Ok(types)
}

fn function(
    mut validator: impl FunctionValidator,
    sig: &wasmparser::FuncType,
    body: FunctionBody,
) -> crate::Result<Function> {
    let mut local_indices = Vec::new();
    let mut local_index = 0;
    for ty in sig.params() {
        match ty {
            wasmparser::ValType::I32 | wasmparser::ValType::I64 => {
                local_indices.push(local_index);
                local_index += 1;
            }
            wasmparser::ValType::F32 | wasmparser::ValType::F64 => {
                local_indices.push(local_index);
                local_index += 2;
            }
            wasmparser::ValType::V128 => unimplemented!(),
            wasmparser::ValType::Ref(_) => unimplemented!(),
        }
    }
    assert_eq!(body.get_locals_reader()?.get_count(), 0); // TODO: Handle locals.
    let mut func = Func {
        local_types: sig.params().to_vec(),
        local_indices,
        tmp_f64: (
            local_index,
            local_index + 1,
            local_index + 2,
            local_index + 3,
        ),
        body: Function::new([(4, wasm_encoder::ValType::F64)]),
    };
    let mut operators_reader = body.get_operators_reader()?;
    while !operators_reader.eof() {
        let (op, offset) = operators_reader.read_with_offset()?;
        validator.op(offset, &op)?;
        func.op(op)?;
    }
    validator.finish(operators_reader.original_position())?;
    Ok(func.body)
}

struct Func {
    local_types: Vec<wasmparser::ValType>,
    local_indices: Vec<u32>,
    tmp_f64: (u32, u32, u32, u32),
    body: Function,
}

impl Func {
    fn op(&mut self, op: Operator) -> crate::Result<()> {
        match op {
            Operator::End => {
                self.instructions().end();
            }
            Operator::LocalGet { local_index } => {
                let i = self.local_index(local_index);
                self.instructions().local_get(i);
                if let wasmparser::ValType::F32 | wasmparser::ValType::F64 =
                    self.local_type(local_index)
                {
                    self.instructions().local_get(i + 1);
                }
            }
            Operator::F64Mul => {
                let (x, dx, y, dy) = self.tmp_f64;
                self.instructions()
                    .local_set(dy)
                    .local_set(y)
                    .local_set(dx)
                    .local_tee(x)
                    .local_get(y)
                    .f64_mul()
                    .local_get(dx)
                    .local_get(y)
                    .f64_mul()
                    .local_get(x)
                    .local_get(dy)
                    .f64_mul()
                    .f64_add();
            }
            _ => unimplemented!("{op:?}"),
        }
        Ok(())
    }

    fn local_type(&self, index: u32) -> wasmparser::ValType {
        self.local_types[u32_to_usize(index)]
    }

    fn local_index(&self, index: u32) -> u32 {
        self.local_indices[u32_to_usize(index)]
    }

    fn instructions(&mut self) -> InstructionSink {
        self.body.instructions()
    }
}

#[cfg(test)]
mod tests {
    use wasmtime::{Engine, Instance, Module, Store};

    #[test]
    fn test_square() {
        let input = wat::parse_str(include_str!("wat/square.wat")).unwrap();

        let ad = crate::Forward::new();
        let output = ad.transform(&input).unwrap();

        let engine = Engine::default();
        let mut store = Store::new(&engine, ());
        let module = Module::new(&engine, &output).unwrap();
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        let square = instance
            .get_typed_func::<(f64, f64), (f64, f64)>(&mut store, "square")
            .unwrap();

        assert_eq!(square.call(&mut store, (3., 1.)).unwrap(), (9., 6.));
    }
}
