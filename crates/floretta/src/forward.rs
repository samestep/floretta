use wasm_encoder::{
    CodeSection, ExportSection, Function, FunctionSection, InstructionSink, Module, TypeSection,
    reencode::{Reencode, RoundtripReencoder},
};
use wasmparser::{FunctionBody, Operator, Parser, Payload};

use crate::{
    Autodiff,
    util::{FuncTypes, ValType, u32_to_usize},
    validate::{FunctionValidator, ModuleValidator},
};

pub fn transform(
    mut validator: impl ModuleValidator,
    _: &Autodiff,
    wasm_module: &[u8],
) -> crate::Result<Vec<u8>> {
    let mut types = TypeSection::new();
    let mut functions = FunctionSection::new();
    let mut exports = ExportSection::new();
    let mut code = CodeSection::new();
    let mut type_sigs = FuncTypes::new();
    let mut func_types = Vec::new();
    let mut num_bodies = 0;
    for payload in Parser::new(0).parse_all(wasm_module) {
        match payload? {
            Payload::TypeSection(section) => {
                validator.type_section(&section)?;
                for ty in section.into_iter_err_on_gc_types() {
                    let typeidx = type_sigs.push(ty?)?;
                    types.ty().function(
                        tuple(type_sigs.params(typeidx))?,
                        tuple(type_sigs.results(typeidx))?,
                    );
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
                code.function(&function(func, &type_sigs, func_types[num_bodies], body)?);
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

fn tuple(val_types: &[ValType]) -> crate::Result<Vec<wasm_encoder::ValType>> {
    let mut types = Vec::new();
    for &ty in val_types {
        match ty {
            ValType::I32 | ValType::I64 => types.push(ty.into()),
            ValType::F32 | ValType::F64 => {
                let reencoded = ty.into();
                types.push(reencoded);
                types.push(reencoded);
            }
        }
    }
    Ok(types)
}

fn function(
    mut validator: impl FunctionValidator,
    type_sigs: &FuncTypes,
    typeidx: u32,
    body: FunctionBody,
) -> crate::Result<Function> {
    let mut local_indices = Vec::new();
    let mut local_index = 0;
    for ty in type_sigs.params(typeidx) {
        match ty {
            ValType::I32 | ValType::I64 => {
                local_indices.push(local_index);
                local_index += 1;
            }
            ValType::F32 | ValType::F64 => {
                local_indices.push(local_index);
                local_index += 2;
            }
        }
    }
    assert_eq!(body.get_locals_reader()?.get_count(), 0); // TODO: Handle locals.
    let mut func = Func {
        local_types: type_sigs.params(typeidx).to_vec(),
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
    local_types: Vec<ValType>,
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
                if let ValType::F32 | ValType::F64 = self.local_type(local_index) {
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

    fn local_type(&self, index: u32) -> ValType {
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

    use crate::Autodiff;

    #[test]
    fn test_square() {
        let input = wat::parse_str(include_str!("wat/square.wat")).unwrap();

        let output = Autodiff::new().forward(&input).unwrap();

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
