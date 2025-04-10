use crate::ErrorImpl;

pub fn u32_to_usize(n: u32) -> usize {
    n.try_into()
        .expect("pointer size is assumed to be at least 32 bits")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ValType {
    I32,
    I64,
    F32,
    F64,
}

impl ValType {
    pub fn is_float(self) -> bool {
        matches!(self, ValType::F32 | ValType::F64)
    }

    pub fn singleton(self) -> &'static [Self] {
        match self {
            ValType::I32 => &[ValType::I32],
            ValType::I64 => &[ValType::I64],
            ValType::F32 => &[ValType::F32],
            ValType::F64 => &[ValType::F64],
        }
    }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BlockType {
    Empty,
    Result(ValType),
    Func(u32),
}

impl TryFrom<wasmparser::BlockType> for BlockType {
    type Error = ErrorImpl;

    fn try_from(value: wasmparser::BlockType) -> Result<Self, Self::Error> {
        match value {
            wasmparser::BlockType::Empty => Ok(BlockType::Empty),
            wasmparser::BlockType::Type(val_type) => Ok(BlockType::Result(val_type.try_into()?)),
            wasmparser::BlockType::FuncType(typeidx) => Ok(BlockType::Func(typeidx)),
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

/// A map whose keys are Wasm types.
#[derive(Clone, Copy, Debug, Default)]
pub struct TypeMap<T> {
    pub i32: T,
    pub i64: T,
    pub f32: T,
    pub f64: T,
}

impl<T> TypeMap<T> {
    /// Get a reference to the value associated with a Wasm type.
    pub fn get(&self, ty: ValType) -> &T {
        match ty {
            ValType::I32 => &self.i32,
            ValType::I64 => &self.i64,
            ValType::F32 => &self.f32,
            ValType::F64 => &self.f64,
        }
    }

    /// Get a mutable reference to the value associated with a Wasm type.
    pub fn get_mut(&mut self, ty: ValType) -> &mut T {
        match ty {
            ValType::I32 => &mut self.i32,
            ValType::I64 => &mut self.i64,
            ValType::F32 => &mut self.f32,
            ValType::F64 => &mut self.f64,
        }
    }
}

/// Map local indices in a source function to local indices in a transformed function.
pub struct LocalMap {
    /// This type assumes that the mapping is simple: for each local as you iterate through the
    /// locals from the source function in order, you allocate a constant number of locals in the
    /// transformed function. This `type_map` says what that constant is for each type.
    type_map: TypeMap<u32>,

    /// Wasm locals are given in _entries_, each of which holds some _count_ of locals that all
    /// share the same type. For each such entry, this `ends` vector holds the smallest index
    /// greater than all the indices of that entry in both the source function and the transformed
    /// function.
    ends: Vec<(u32, u32)>,

    /// For each entry, this vector holds the type of all the locals in that entry.
    types: Vec<ValType>,
}

impl LocalMap {
    /// Create a new map of locals.
    pub fn new(type_map: TypeMap<u32>) -> Self {
        Self {
            type_map,
            ends: Vec::new(),
            types: Vec::new(),
        }
    }

    /// Add an entry to the local map.
    pub fn push(&mut self, count: u32, ty: ValType) {
        let &(k, v) = self.ends.last().unwrap_or(&(0, 0));
        let multiplier = *self.type_map.get(ty);
        self.ends.push((k + count, v + multiplier * count));
        self.types.push(ty);
    }

    /// Get the number of locals in the source function.
    pub fn count_keys(&self) -> u32 {
        let &(end, _) = self.ends.last().unwrap_or(&(0, 0));
        end
    }

    /// Get the number of locals in the transformed function.
    pub fn count_vals(&self) -> u32 {
        let &(_, end) = self.ends.last().unwrap_or(&(0, 0));
        end
    }

    /// Get the type and mapped index of a local, given a local's `index` in the source function.
    pub fn get(&self, index: u32) -> (ValType, Option<u32>) {
        let i = self.ends.partition_point(|&(end, _)| end <= index);
        let ty = self.types[i];
        let (k, v) = match i.checked_sub(1) {
            Some(j) => self.ends[j],
            None => (0, 0),
        };
        let mapped = match self.type_map.get(ty) {
            0 => None,
            n => Some(v + n * (index - k)),
        };
        (ty, mapped)
    }

    /// Return an iterator over the source entries of the local map.
    pub fn keys(&self) -> impl ExactSizeIterator<Item = (u32, wasm_encoder::ValType)> + '_ {
        let mut start = 0;
        self.ends
            .iter()
            .zip(self.types.iter())
            .map(move |(&(end, _), &ty)| {
                let count = end - start;
                start = end;
                (count, ty.into())
            })
    }

    /// Return an iterator over the transformed entries of the local map.
    pub fn vals(&self) -> impl ExactSizeIterator<Item = (u32, ValType)> + '_ {
        let mut start = 0;
        self.ends
            .iter()
            .zip(self.types.iter())
            .map(move |(&(_, end), &ty)| {
                let count = end - start;
                start = end;
                (count, ty)
            })
    }
}

#[cfg(test)]
mod tests {

    use crate::util::{LocalMap, TypeMap, ValType};

    fn ones() -> TypeMap<u32> {
        TypeMap {
            i32: 1,
            i64: 1,
            f32: 1,
            f64: 1,
        }
    }

    #[test]
    fn test_locals_map_zero() {
        let mut locals = LocalMap::new(TypeMap { i32: 0, ..ones() });
        locals.push(1, ValType::I32);
        locals.push(1, ValType::F64);
        assert_eq!(locals.get(0), (ValType::I32, None));
        assert_eq!(locals.get(1), (ValType::F64, Some(0)));
    }

    #[test]
    fn test_locals_entry_zero() {
        let mut locals = LocalMap::new(ones());
        locals.push(1, ValType::I32);
        locals.push(0, ValType::I64);
        locals.push(0, ValType::F32);
        locals.push(1, ValType::F64);
        assert_eq!(locals.get(0), (ValType::I32, Some(0)));
        assert_eq!(locals.get(1), (ValType::F64, Some(1)));
    }

    #[test]
    fn test_locals_entry_multiple() {
        let mut type_map = ones();
        type_map.i32 = 2;
        let mut locals = LocalMap::new(type_map);
        locals.push(3, ValType::F64);
        locals.push(5, ValType::I32);
        assert_eq!(locals.get(0), (ValType::F64, Some(0)));
        assert_eq!(locals.get(1), (ValType::F64, Some(1)));
        assert_eq!(locals.get(2), (ValType::F64, Some(2)));
        assert_eq!(locals.get(3), (ValType::I32, Some(3)));
        assert_eq!(locals.get(4), (ValType::I32, Some(5)));
        assert_eq!(locals.get(5), (ValType::I32, Some(7)));
        assert_eq!(locals.get(6), (ValType::I32, Some(9)));
        assert_eq!(locals.get(7), (ValType::I32, Some(11)));
    }
}
