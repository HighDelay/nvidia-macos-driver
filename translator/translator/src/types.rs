use crate::native::ir::LlType;
use spirv::{StorageClass, Word};
use std::collections::HashMap;

pub(crate) struct TypeInterner {
    pub(crate) types: HashMap<LlType, Word>,
    pub(crate) signed_int_types: HashMap<LlType, Word>,
    pub(crate) ptr_types: HashMap<(StorageClass, LlType), Word>,
    pub(crate) function_types: HashMap<Vec<Word>, Word>,
    pub(crate) uint_constants: HashMap<u32, Word>,
    pub(crate) int_constants: HashMap<(u32, u64), Word>,
    pub(crate) bool_constants: HashMap<bool, Word>,
    pub(crate) float32_constants: HashMap<u32, Word>,
    pub(crate) float16_constants: HashMap<u16, Word>,
    pub(crate) null_constants: HashMap<LlType, Word>,
    pub(crate) composite_constants: HashMap<(LlType, Vec<Word>), Word>,
    pub(crate) undefs: HashMap<LlType, Word>,
}

impl TypeInterner {
    pub(crate) fn new() -> Self {
        Self {
            types: HashMap::new(),
            signed_int_types: HashMap::new(),
            ptr_types: HashMap::new(),
            function_types: HashMap::new(),
            uint_constants: HashMap::new(),
            int_constants: HashMap::new(),
            bool_constants: HashMap::new(),
            float32_constants: HashMap::new(),
            float16_constants: HashMap::new(),
            null_constants: HashMap::new(),
            composite_constants: HashMap::new(),
            undefs: HashMap::new(),
        }
    }
}
