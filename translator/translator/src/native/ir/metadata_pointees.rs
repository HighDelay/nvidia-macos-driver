use super::*;

impl LlModule {
    pub(in crate::native) fn infer_metadata_buffer_pointees(
        &mut self,
        kern: Option<&meta::KernMeta>,
        entry_name: Option<&str>,
    ) {
        let Some(kern) = kern else {
            return;
        };
        let Some(entry_name) = entry_name else {
            return;
        };
        let Some(entry) = self.functions.iter().find(|f| f.name == entry_name) else {
            return;
        };
        for (idx, (name, ty)) in entry.params.iter().enumerate() {
            if !matches!(ty, LlType::Ptr(1..=3)) {
                continue;
            }
            let Some(layout) = kern.layout_of(idx as u32) else {
                continue;
            };
            let key = (entry.name.clone(), name.clone());
            if self.air_metadata_requires_byte_view(layout) {
                self.raw_buffer_params.insert(key);
                continue;
            }
            let Some(declared_size) = kern.buffer_type_size(idx as u32) else {
                continue;
            };
            let pointee = ll_type_from_air_type(layout);
            let Some((size, _align)) = self.air_metadata_type_size_align(layout) else {
                continue;
            };
            if size != u64::from(declared_size) {
                continue;
            }
            if !self.ptr_pointees.contains_key(&key) {
                self.ptr_pointees.insert(key.clone(), pointee);
                self.metadata_pointee_sizes
                    .insert(key.clone(), u64::from(declared_size));
                self.metadata_pointee_params.insert(key);
            }
        }
    }

    pub(in crate::native) fn infer_metadata_primitive_buffer_pointees(
        &mut self,
        kern: Option<&meta::KernMeta>,
        entry_name: Option<&str>,
    ) {
        let Some(kern) = kern else {
            return;
        };
        let Some(entry_name) = entry_name else {
            return;
        };
        let Some(entry) = self.functions.iter().find(|f| f.name == entry_name) else {
            return;
        };
        let phi_gep_sources = cross_buffer_pointer_phi_gep_sources(entry);
        for (idx, (name, ty)) in entry.params.iter().enumerate() {
            if !matches!(ty, LlType::Ptr(1..=3)) {
                continue;
            }
            let key = (entry.name.clone(), name.clone());
            if self.ptr_pointees.contains_key(&key) {
                continue;
            }
            let Some(type_name) = kern.buffer_type_name(idx as u32) else {
                continue;
            };
            let Some(layout) = meta::primitive_air_type_from_name(type_name) else {
                continue;
            };
            let pointee = ll_type_from_air_type(&layout);
            if !phi_gep_sources.get(name).is_some_and(|sources| {
                sources
                    .iter()
                    .any(|source| self.resolve_known_type(source) == pointee)
            }) {
                continue;
            }
            let Some(declared_size) = kern.buffer_type_size(idx as u32) else {
                continue;
            };
            let Some((size, _align)) = self.air_metadata_type_size_align(&layout) else {
                continue;
            };
            if size != u64::from(declared_size) {
                continue;
            }
            self.ptr_pointees.insert(key.clone(), pointee);
            self.metadata_pointee_sizes
                .insert(key.clone(), u64::from(declared_size));
            self.metadata_pointee_params.insert(key);
        }
    }
}
