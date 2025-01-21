use wasmparser::{
    ExportSectionReader, FuncValidator, FuncValidatorAllocations, FunctionBody,
    FunctionSectionReader, GlobalSectionReader, MemorySectionReader, Operator, Payload,
    TypeSectionReader, Validator, ValidatorResources, WasmModuleResources,
};

/// Trait counterpart to [`wasmparser::Validator`].
pub trait ModuleValidator {
    type Func: FunctionValidator;

    fn payload(&mut self, payload: &Payload) -> wasmparser::Result<()>;

    fn type_section(&mut self, section: &TypeSectionReader) -> wasmparser::Result<()>;

    fn function_section(&mut self, section: &FunctionSectionReader) -> wasmparser::Result<()>;

    fn memory_section(&mut self, section: &MemorySectionReader) -> wasmparser::Result<()>;

    fn global_section(&mut self, section: &GlobalSectionReader) -> wasmparser::Result<()>;

    fn export_section(&mut self, section: &ExportSectionReader) -> wasmparser::Result<()>;

    fn code_section_entry(&mut self, body: &FunctionBody) -> wasmparser::Result<Self::Func>;
}

/// Trait counterpart to [`wasmparser::FuncValidator`].
pub trait FunctionValidator {
    fn define_locals(
        &mut self,
        offset: usize,
        count: u32,
        ty: wasmparser::ValType,
    ) -> wasmparser::Result<()>;

    /// For debugging purposes.
    fn check_operand_stack_height(&self, height: u32);

    fn op(&mut self, offset: usize, operator: &Operator) -> wasmparser::Result<()>;

    fn finish(&mut self, offset: usize) -> wasmparser::Result<()>;
}

impl ModuleValidator for () {
    type Func = ();

    fn payload(&mut self, _: &Payload) -> wasmparser::Result<()> {
        Ok(())
    }

    fn type_section(&mut self, _: &TypeSectionReader) -> wasmparser::Result<()> {
        Ok(())
    }

    fn function_section(&mut self, _: &FunctionSectionReader) -> wasmparser::Result<()> {
        Ok(())
    }

    fn memory_section(&mut self, _: &MemorySectionReader) -> wasmparser::Result<()> {
        Ok(())
    }

    fn global_section(&mut self, _: &GlobalSectionReader) -> wasmparser::Result<()> {
        Ok(())
    }

    fn export_section(&mut self, _: &ExportSectionReader) -> wasmparser::Result<()> {
        Ok(())
    }

    fn code_section_entry(&mut self, _: &FunctionBody) -> wasmparser::Result<Self::Func> {
        Ok(())
    }
}

impl FunctionValidator for () {
    fn define_locals(
        &mut self,
        _: usize,
        _: u32,
        _: wasmparser::ValType,
    ) -> wasmparser::Result<()> {
        Ok(())
    }

    fn check_operand_stack_height(&self, _: u32) {}

    fn op(&mut self, _: usize, _: &Operator) -> wasmparser::Result<()> {
        Ok(())
    }

    fn finish(&mut self, _: usize) -> wasmparser::Result<()> {
        Ok(())
    }
}

impl ModuleValidator for Validator {
    type Func = FuncValidator<ValidatorResources>;

    fn payload(&mut self, payload: &Payload) -> wasmparser::Result<()> {
        self.payload(payload)?;
        Ok(())
    }

    fn type_section(&mut self, section: &TypeSectionReader) -> wasmparser::Result<()> {
        self.type_section(section)
    }

    fn function_section(&mut self, section: &FunctionSectionReader) -> wasmparser::Result<()> {
        self.function_section(section)
    }

    fn memory_section(&mut self, section: &MemorySectionReader) -> wasmparser::Result<()> {
        self.memory_section(section)
    }

    fn global_section(&mut self, section: &GlobalSectionReader) -> wasmparser::Result<()> {
        self.global_section(section)
    }

    fn export_section(&mut self, section: &ExportSectionReader) -> wasmparser::Result<()> {
        self.export_section(section)
    }

    fn code_section_entry(&mut self, body: &FunctionBody) -> wasmparser::Result<Self::Func> {
        let func = self.code_section_entry(body)?;
        Ok(func.into_validator(FuncValidatorAllocations::default()))
    }
}

impl<T: WasmModuleResources> FunctionValidator for FuncValidator<T> {
    fn define_locals(
        &mut self,
        offset: usize,
        count: u32,
        ty: wasmparser::ValType,
    ) -> wasmparser::Result<()> {
        self.define_locals(offset, count, ty)
    }

    fn check_operand_stack_height(&self, height: u32) {
        let n = self.operand_stack_height();
        if n != height {
            panic!("operand stack height mismatch: expected {n}, got {height}");
        }
    }

    fn op(&mut self, offset: usize, operator: &Operator) -> wasmparser::Result<()> {
        self.op(offset, operator)
    }

    fn finish(&mut self, offset: usize) -> wasmparser::Result<()> {
        self.finish(offset)
    }
}
