use crate::ErrorImpl;

pub fn u32_to_usize(n: u32) -> usize {
    n.try_into()
        .expect("pointer size is assumed to be at least 32 bits")
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ValType {
    I32,
    I64,
    F32,
    F64,
}

impl TryFrom<wasmparser::ValType> for ValType {
    type Error = ErrorImpl;

    fn try_from(value: wasmparser::ValType) -> Result<Self, Self::Error> {
        match value {
            wasmparser::ValType::I32 => Ok(ValType::I32),
            wasmparser::ValType::I64 => Ok(ValType::I64),
            wasmparser::ValType::F32 => Ok(ValType::F32),
            wasmparser::ValType::F64 => Ok(ValType::F64),
            wasmparser::ValType::V128 => Err(ErrorImpl::Transform("SIMD is unsupported")),
            wasmparser::ValType::Ref(_) => {
                Err(ErrorImpl::Transform("reference types are unsupported"))
            }
        }
    }
}

impl From<ValType> for wasm_encoder::ValType {
    fn from(value: ValType) -> Self {
        match value {
            ValType::I32 => wasm_encoder::ValType::I32,
            ValType::I64 => wasm_encoder::ValType::I64,
            ValType::F32 => wasm_encoder::ValType::F32,
            ValType::F64 => wasm_encoder::ValType::F64,
        }
    }
}

/// A list of function types, parsed from a Wasm type section.
pub struct FuncTypes {
    val_types: Vec<ValType>,
    offsets: Vec<(u32, u32)>,
}

impl FuncTypes {
    /// Create an empty list of function types.
    pub fn new() -> Self {
        Self {
            val_types: Vec::new(),
            offsets: Vec::new(),
        }
    }

    /// Push a function type parsed from a Wasm type section, returning its index.
    pub fn push(&mut self, ty: wasmparser::FuncType) -> crate::Result<u32> {
        // We know that the type index can be represented as a `u32` because the type section is a
        // vector of function types, and vectors in the Wasm spec encode their length as a `u32`.
        let typeidx = u32::try_from(self.offsets.len()).unwrap();
        // We know that any offset into our flattened `val_types` can be represented as a `u32`
        // because each parameter and result type in a Wasm type section must be listed
        // individually, and each value type takes at least one byte, and every Wasm section encodes
        // its number of bytes as a `u32`.
        let offset_params = u32::try_from(self.val_types.len()).unwrap();
        for &param in ty.params() {
            self.val_types.push(ValType::try_from(param)?);
        }
        let offset_results = u32::try_from(self.val_types.len()).unwrap();
        for &result in ty.results() {
            self.val_types.push(ValType::try_from(result)?);
        }
        self.offsets.push((offset_params, offset_results));
        Ok(typeidx)
    }

    /// Get the parameters of a function type.
    pub fn params(&self, typeidx: u32) -> &[ValType] {
        let t = u32_to_usize(typeidx);
        let (offset_params, offset_results) = self.offsets[t];
        let i = u32_to_usize(offset_params);
        let j = u32_to_usize(offset_results);
        &self.val_types[i..j]
    }

    /// Get the results of a function type.
    pub fn results(&self, typeidx: u32) -> &[ValType] {
        let t = u32_to_usize(typeidx);
        let (_, offset_results) = self.offsets[t];
        let i = u32_to_usize(offset_results);
        match self.offsets.get(t + 1) {
            Some(&(offset_params, _)) => {
                let j = u32_to_usize(offset_params);
                &self.val_types[i..j]
            }
            None => &self.val_types[i..],
        }
    }
}
