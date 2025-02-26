use std::collections::HashMap;

use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, Function, FunctionSection, IndirectNameMap,
    InstructionSink, Module, NameMap, NameSection, TypeSection, ValType,
};

pub fn to_wasm(input: &str) -> Vec<u8> {
    let mut parser = VmParser::new();
    for line in input.lines() {
        parser.line(line);
    }
    parser.finish()
}

const PARAMS: u32 = 2;

#[derive(Default)]
struct VmParser<'a> {
    body: Vec<u8>,
    params: HashMap<&'a str, u32>,
    nodes: HashMap<&'a str, u32>,
    names: NameMap,
}

impl<'a> VmParser<'a> {
    fn new() -> Self {
        Self::default()
    }

    fn index(&self) -> u32 {
        PARAMS + u32::try_from(self.nodes.len()).unwrap()
    }

    fn get(&self, name: Option<&str>) -> u32 {
        let name = name.unwrap();
        match self.nodes.get(name) {
            Some(&i) => i,
            None => self.params[name],
        }
    }

    fn unop<F>(&mut self, i: u32, a: Option<&str>, f: F)
    where
        for<'b, 'c> F: FnOnce(&'b mut InstructionSink<'c>) -> &'b mut InstructionSink<'c>,
    {
        let x = self.get(a);
        let mut insn = InstructionSink::new(&mut self.body);
        insn.local_get(x);
        f(&mut insn);
        insn.local_set(i);
    }

    fn binop<F>(&mut self, i: u32, a: Option<&str>, b: Option<&str>, f: F)
    where
        for<'b, 'c> F: FnOnce(&'b mut InstructionSink<'c>) -> &'b mut InstructionSink<'c>,
    {
        let x = self.get(a);
        let y = self.get(b);
        let mut insn = InstructionSink::new(&mut self.body);
        insn.local_get(x);
        insn.local_get(y);
        f(&mut insn);
        insn.local_set(i);
    }

    fn line(&mut self, line: &'a str) {
        if line.starts_with('#') {
            return;
        }
        let mut parts = line.split_whitespace();
        let id = parts.next().unwrap();
        let op = parts.next().unwrap();
        if op.starts_with("var-") {
            let i = u32::try_from(self.params.len()).unwrap();
            self.params.insert(id, i);
            self.names.append(i, id);
        } else {
            let i = self.index();
            self.nodes.insert(id, i);
            self.names.append(i, id);
            let a = parts.next();
            let b = parts.next();
            match op {
                "const" => {
                    InstructionSink::new(&mut self.body)
                        .f32_const(a.unwrap().parse().unwrap())
                        .local_set(i);
                }
                "square" => {
                    let x = self.get(a);
                    InstructionSink::new(&mut self.body)
                        .local_get(x)
                        .local_get(x)
                        .f32_mul()
                        .local_set(i);
                }
                "neg" => self.unop(i, a, |insn| insn.f32_neg()),
                "sqrt" => self.unop(i, a, |insn| insn.f32_sqrt()),
                "add" => self.binop(i, a, b, |insn| insn.f32_add()),
                "sub" => self.binop(i, a, b, |insn| insn.f32_sub()),
                "mul" => self.binop(i, a, b, |insn| insn.f32_mul()),
                "min" => self.binop(i, a, b, |insn| insn.f32_min()),
                "max" => self.binop(i, a, b, |insn| insn.f32_max()),
                _ => unimplemented!("{op}"),
            };
        }
    }

    fn finish(mut self) -> Vec<u8> {
        assert_eq!(u32::try_from(self.params.len()).unwrap(), PARAMS);
        let last = self.index() - 1;
        InstructionSink::new(&mut self.body).local_get(last).end();
        let mut module = Module::new();
        let mut types = TypeSection::new();
        types
            .ty()
            .function([ValType::F32, ValType::F32], [ValType::F32]);
        module.section(&types);
        let mut functions = FunctionSection::new();
        functions.function(0);
        module.section(&functions);
        let mut exports = ExportSection::new();
        exports.export("main", ExportKind::Func, 0);
        module.section(&exports);
        let mut code = CodeSection::new();
        let function = Function::new([(u32::try_from(self.nodes.len()).unwrap(), ValType::F32)]);
        let mut raw = function.into_raw_body();
        raw.extend_from_slice(&self.body);
        code.raw(&raw);
        module.section(&code);
        let mut name_section = NameSection::new();
        let mut indirect = IndirectNameMap::new();
        indirect.append(0, &self.names);
        name_section.locals(&indirect);
        module.section(&name_section);
        module.finish()
    }
}
