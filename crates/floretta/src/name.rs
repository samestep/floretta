use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use regex::Regex;
use wasm_encoder::NameSection;
use wasmparser::{IndirectNaming, Name, NameSectionReader, Naming};

use crate::{
    helper::{
        FUNC_CONTROL_LOAD, FUNC_CONTROL_STORE, FUNC_F32_DIV_BWD, FUNC_F32_DIV_FWD,
        FUNC_F32_MUL_BWD, FUNC_F32_MUL_FWD, FUNC_F64_DIV_BWD, FUNC_F64_DIV_FWD, FUNC_F64_MUL_BWD,
        FUNC_F64_MUL_FWD, OFFSET_FUNCTIONS,
    },
    run::StackHeight,
    Error,
};

struct NameNumbers {
    base_available: bool,
    taken: HashSet<u32>,
    mex: u32,
}

impl NameNumbers {
    fn new() -> Self {
        Self {
            base_available: true,
            taken: HashSet::new(),
            mex: 2,
        }
    }

    fn insert_base(&mut self) -> Option<u32> {
        if self.base_available {
            self.base_available = false;
            None
        } else {
            Some(self.insert_number(self.mex))
        }
    }

    fn insert_number(&mut self, number: u32) -> u32 {
        if self.taken.insert(number) {
            number
        } else {
            while self.taken.contains(&self.mex) {
                self.mex += 1;
            }
            self.mex
        }
    }

    fn insert(&mut self, number: Option<u32>) -> Option<u32> {
        match number {
            Some(n) => Some(self.insert_number(n)),
            None => self.insert_base(),
        }
    }
}

struct Decomposition<'a> {
    name: &'a str,
    base: &'a str,
    number: Option<u32>,
}

impl<'a> Decomposition<'a> {
    fn recompose(&self, numbers: &mut NameNumbers) -> Cow<'a, str> {
        let number = numbers.insert(self.number);
        if number == self.number {
            Cow::Borrowed(self.name)
        } else {
            let n = number.unwrap();
            Cow::Owned(format!("{}_{n}", self.base))
        }
    }
}

/// A set of names that can efficiently give a name not in the set but similar to a name in the set.
pub struct NameSet<'a> {
    re: Regex,
    names: HashMap<&'a str, NameNumbers>,
}

impl Default for NameSet<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> NameSet<'a> {
    /// Empty set of names.
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"^(.*)_(\d+)$").unwrap(),
            names: HashMap::new(),
        }
    }

    fn decompose<'b>(&self, name: &'b str) -> Decomposition<'b> {
        if let Some(caps) = self.re.captures(name) {
            if let Ok(n) = caps[2].parse() {
                return Decomposition {
                    name,
                    base: caps.get(1).unwrap().as_str(),
                    number: Some(n),
                };
            }
        }
        Decomposition {
            name,
            base: name,
            number: None,
        }
    }

    /// Insert a name into the set.
    ///
    /// If `name` was not already in the set, it is returned as [`Cow::Borrowed`]. Otherwise, a
    /// similar name that wasn't previously in the set is returned as [`Cow::Owned`].
    pub fn insert(&mut self, name: &'a str) -> Cow<'a, str> {
        let decomp = self.decompose(name);
        let numbers = self
            .names
            .entry(decomp.base)
            .or_insert_with(NameNumbers::new);
        decomp.recompose(numbers)
    }

    /// Done adding names from the original source; transition to the next phase.
    pub fn done(self) -> NameGen<'a> {
        NameGen { inner: self }
    }
}

#[derive(Default)]
pub struct NameGen<'a> {
    inner: NameSet<'a>,
}

impl NameGen<'_> {
    /// Insert a name into the set if possible.
    ///
    /// If `name` was not already in the set, it is returned as [`Cow::Borrowed`]. Otherwise, a
    /// similar name that wasn't previously in the set is returned as [`Cow::Owned`].
    ///
    /// If `name` is composed of some base string followed by an underscore and then a nonnegative
    /// integer less than [`u32::MAX`], the returned name is inserted if and only if some other name
    /// with the same base string was already present. Otherwise, the returned name is inserted if
    /// and only if `name` was already present.
    pub fn insert<'b>(&mut self, name: &'b str) -> Cow<'b, str> {
        let decomp = self.inner.decompose(name);
        match self.inner.names.get_mut(decomp.base) {
            Some(numbers) => decomp.recompose(numbers),
            None => Cow::Borrowed(name),
        }
    }
}

pub trait FuncInfo {
    fn num_functions(&self) -> u32;

    fn num_results(&self, funcidx: u32) -> u32;

    fn num_locals(&self, funcidx: u32) -> u32;

    fn stack_locals(&self, funcidx: u32) -> StackHeight;
}

#[derive(Default)]
pub struct Names<'a> {
    section: NameSection,
    function_map: wasm_encoder::NameMap,
    function_gen: NameGen<'a>,
    locals_map: wasm_encoder::IndirectNameMap,
    locals_maps: HashMap<u32, (wasm_encoder::NameMap, NameGen<'a>)>,
}

impl<'a> Names<'a> {
    pub fn new(functions: impl FuncInfo, reader: NameSectionReader<'a>) -> Result<Self, Error> {
        let mut section = NameSection::new();
        let mut function_map = wasm_encoder::NameMap::new();
        let mut function_set = Some(NameSet::new());
        let mut function_gen = None;
        let mut locals_map = wasm_encoder::IndirectNameMap::new();
        let mut locals_maps = HashMap::new();
        for entry in reader {
            match entry? {
                Name::Module {
                    name,
                    name_range: _,
                } => section.module(name),
                Name::Function(functions_in) => {
                    let mut function_names = function_set.take().unwrap();
                    for function in functions_in.clone() {
                        let Naming { index, name } = function?;
                        function_map.append(OFFSET_FUNCTIONS + 2 * index, name);
                        function_names.insert(name);
                    }
                    let mut function_names = function_names.done();
                    for function in functions_in {
                        let Naming { index, name } = function?;
                        function_map.append(
                            OFFSET_FUNCTIONS + 2 * index + 1,
                            &function_names.insert(&format!("{name}_bwd")),
                        );
                    }
                    function_gen = Some(function_names);
                }
                Name::Local(functions_in) => {
                    for function in functions_in {
                        let mut locals_fwd = wasm_encoder::NameMap::new();
                        let mut locals_bwd = wasm_encoder::NameMap::new();
                        let mut local_names = NameSet::new();
                        let IndirectNaming {
                            index,
                            names: locals_in,
                        } = function?;
                        let num_results = functions.num_results(index);
                        for local in locals_in {
                            let Naming { index, name } = local?;
                            locals_fwd.append(index, name);
                            locals_bwd.append(num_results + index, name);
                            local_names.insert(name);
                        }
                        locals_map.append(OFFSET_FUNCTIONS + 2 * index, &locals_fwd);
                        locals_maps.insert(index, (locals_bwd, local_names.done()));
                    }
                }
                _ => {} // TODO
            }
        }
        Ok(Self {
            section,
            function_map,
            function_gen: function_gen.unwrap_or_default(),
            locals_map,
            locals_maps,
        })
    }
}

pub fn name_section(functions: impl FuncInfo, names: Option<Names>) -> NameSection {
    let Names {
        mut section,
        mut function_map,
        mut function_gen,
        mut locals_map,
        mut locals_maps,
    } = names.unwrap_or_default();
    function_map.append(FUNC_CONTROL_STORE, &function_gen.insert("control_store"));
    function_map.append(FUNC_CONTROL_LOAD, &function_gen.insert("control_load"));
    function_map.append(FUNC_F32_MUL_FWD, &function_gen.insert("f32_mul"));
    function_map.append(FUNC_F32_DIV_FWD, &function_gen.insert("f32_div"));
    function_map.append(FUNC_F64_MUL_FWD, &function_gen.insert("f64_mul"));
    function_map.append(FUNC_F64_DIV_FWD, &function_gen.insert("f64_div"));
    function_map.append(FUNC_F32_MUL_BWD, &function_gen.insert("f32_mul_bwd"));
    function_map.append(FUNC_F32_DIV_BWD, &function_gen.insert("f32_div_bwd"));
    function_map.append(FUNC_F64_MUL_BWD, &function_gen.insert("f64_mul_bwd"));
    function_map.append(FUNC_F64_DIV_BWD, &function_gen.insert("f64_div_bwd"));
    section.functions(&function_map);
    for index in 0..functions.num_functions() {
        let (locals, local_names) = locals_maps
            .entry(index)
            .or_insert_with(|| (wasm_encoder::NameMap::new(), NameGen::default()));
        let num_results = functions.num_results(index);
        for i in 0..num_results {
            locals.append(i, &local_names.insert(&format!("result_{i}")));
        }
        let mut local_index = num_results + functions.num_locals(index);
        locals.append(local_index, &local_names.insert("tmp_f64"));
        local_index += 1;
        let stack_locals = functions.stack_locals(index);
        for i in 0..stack_locals.i32 {
            locals.append(local_index, &local_names.insert(&format!("stack_i32_{i}")));
            local_index += 1;
        }
        for i in 0..stack_locals.i64 {
            locals.append(local_index, &local_names.insert(&format!("stack_i64_{i}")));
            local_index += 1;
        }
        for i in 0..stack_locals.f32 {
            locals.append(local_index, &local_names.insert(&format!("stack_f32_{i}")));
            local_index += 1;
        }
        for i in 0..stack_locals.f64 {
            locals.append(local_index, &local_names.insert(&format!("stack_f64_{i}")));
            local_index += 1;
        }
        locals_map.append(OFFSET_FUNCTIONS + 2 * index + 1, locals);
    }
    section.locals(&locals_map);
    section
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::NameSet;

    #[test]
    fn test_no_number() {
        let mut names = NameSet::new();
        let output = names.insert("foo");
        assert_eq!(output, "foo");
    }

    #[test]
    fn test_duplicates() {
        let mut names = NameSet::new();
        let output1 = names.insert("foo");
        let output2 = names.insert("foo");
        let output3 = names.insert("foo");
        assert_eq!(output1, "foo");
        assert_eq!(output2, "foo_2");
        assert_eq!(output3, "foo_3");
    }

    #[test]
    fn test_base() {
        let mut names = NameSet::new();
        let output1 = names.insert("foo_1");
        let output2 = names.insert("foo");
        assert_eq!(output1, "foo_1");
        assert_eq!(output2, "foo");
    }

    #[test]
    fn test_gap() {
        let mut names = NameSet::new();
        let output1 = names.insert("foo");
        let output2 = names.insert("foo_3");
        let output3 = names.insert("foo_5");
        let output4 = names.insert("foo");
        let output5 = names.insert("foo");
        assert_eq!(output1, "foo");
        assert_eq!(output2, "foo_3");
        assert_eq!(output3, "foo_5");
        assert_eq!(output4, "foo_2");
        assert_eq!(output5, "foo_4");
    }

    #[test]
    fn test_big_number() {
        let mut names = NameSet::new();
        let input = format!("foo_{}", u64::from(u32::MAX) + 1);
        let output1 = names.insert(&input);
        let output2 = names.insert(&input);
        assert_eq!(output1, "foo_4294967296");
        assert_eq!(output2, "foo_4294967296_2");
    }

    #[test]
    fn test_number_borrowed() {
        let mut names = NameSet::new();
        match names.insert("foo_1") {
            Cow::Borrowed(_) => {}
            Cow::Owned(_) => panic!(),
        }
    }
}
