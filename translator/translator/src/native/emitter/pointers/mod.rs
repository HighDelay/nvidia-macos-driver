use super::*;
mod gep_linearize;
mod phi_compare;
mod phi_merge;
mod phi_provenance;
mod raw_select;
mod select;
mod select_gep;

pub(super) fn is_scalar_pointee(ty: &LlType) -> bool {
    matches!(
        ty,
        LlType::Int(_) | LlType::Float | LlType::Half | LlType::BFloat | LlType::Bool
    )
}

fn scalar_bit_width(ty: &LlType) -> Option<u32> {
    match ty {
        LlType::Float => Some(32),
        LlType::Half | LlType::BFloat => Some(16),
        LlType::Int(w) => Some(*w),
        _ => None,
    }
}

fn whole_part_widens(carrier: &LlType, scalar_pointee: &LlType) -> bool {
    match carrier {
        LlType::Vector(elem, _) | LlType::Array(elem, _) => elem.as_ref() == scalar_pointee,
        _ => false,
    }
}

fn reinterp_compatible(carrier: &LlType, scalar_pointee: &LlType) -> bool {
    is_scalar_pointee(carrier)
        && carrier != scalar_pointee
        && match (scalar_bit_width(carrier), scalar_bit_width(scalar_pointee)) {
            (Some(cw), Some(pw)) => cw == pw,
            _ => false,
        }
}

fn pointer_arithmetic_access_chain_op_for_storage(
    storage: StorageClass,
    base_is_indexed_container: bool,
    pointee: &LlType,
    indices: &[TypedValue],
) -> Op {
    let scalar_pointee = !matches!(pointee, LlType::Array(_, _) | LlType::Struct(_));
    if scalar_pointee
        && ptr_access_chain_allowed_storage(storage)
        && !base_is_indexed_container
        && !indices.is_empty()
        && indices.first().and_then(|idx| const_index(Some(idx))) != Some(0)
    {
        Op::PtrAccessChain
    } else {
        Op::InBoundsAccessChain
    }
}

fn structural_pointer_index(
    position: usize,
    selected_ty: &mut LlType,
    index: &TypedValue,
    first_index_is_pointer_arithmetic: bool,
) -> Option<bool> {
    if position == 0 && first_index_is_pointer_arithmetic {
        return Some(false);
    }
    if position == 0 && const_index(Some(index)) == Some(0) {
        return Some(true);
    }
    if let LlType::Struct(fields) = selected_ty {
        let member = const_index(Some(index))? as usize;
        *selected_ty = fields.get(member)?.clone();
        return Some(true);
    }
    *selected_ty = match selected_ty.clone() {
        LlType::Array(element, _) | LlType::Vector(element, _) => *element,
        other => other,
    };
    Some(false)
}

#[cfg(test)]
mod pointee_upgrade_tests {
    use super::{
        is_scalar_pointee, reinterp_compatible, scalar_bit_width, structural_pointer_index,
        whole_part_widens,
    };
    use crate::native::ir::{LlType, LlValue, TypedValue};

    fn vec(elem: LlType, n: u32) -> LlType {
        LlType::Vector(Box::new(elem), n)
    }
    fn arr(elem: LlType, n: u32) -> LlType {
        LlType::Array(Box::new(elem), n)
    }

    #[test]
    fn struct_pointer_indices_remain_literals() {
        let mut selected = LlType::Struct(vec![arr(LlType::Float, 4)]);
        let zero = TypedValue {
            ty: LlType::Int(32),
            value: LlValue::Int(0),
        };
        assert_eq!(
            structural_pointer_index(0, &mut selected, &zero, false),
            Some(true)
        );
        assert_eq!(
            structural_pointer_index(1, &mut selected, &zero, false),
            Some(true)
        );
        assert_eq!(selected, arr(LlType::Float, 4));

        let dynamic = TypedValue {
            ty: LlType::Int(64),
            value: LlValue::Local("%index".into()),
        };
        assert_eq!(
            structural_pointer_index(2, &mut selected, &dynamic, false),
            Some(false)
        );
        assert_eq!(selected, LlType::Float);

        let mut structure = LlType::Struct(vec![LlType::Float]);
        assert_eq!(
            structural_pointer_index(1, &mut structure, &dynamic, false),
            None
        );

        let member_one = TypedValue {
            ty: LlType::Int(32),
            value: LlValue::Int(1),
        };
        let mut normalized_structure = LlType::Struct(vec![LlType::Int(32), LlType::Float]);
        assert_eq!(
            structural_pointer_index(0, &mut normalized_structure, &member_one, false),
            Some(true)
        );
        assert_eq!(normalized_structure, LlType::Float);
        let mut normalized_structure = LlType::Struct(vec![LlType::Float]);
        assert_eq!(
            structural_pointer_index(0, &mut normalized_structure, &dynamic, false),
            None
        );
    }

    #[test]
    fn scalars_are_scalar_composites_are_not() {
        for s in [
            LlType::Float,
            LlType::Half,
            LlType::BFloat,
            LlType::Bool,
            LlType::Int(32),
            LlType::Int(16),
            LlType::Int(8),
        ] {
            assert!(is_scalar_pointee(&s), "{s:?} should be scalar");
        }
        for c in [
            vec(LlType::Float, 4),
            arr(LlType::Float, 8),
            LlType::Struct(vec![LlType::Half, LlType::Half]),
            LlType::Ptr(1),
            LlType::Void,
            LlType::Named("x".into()),
        ] {
            assert!(!is_scalar_pointee(&c), "{c:?} should not be scalar");
        }
    }

    #[test]
    fn bit_widths() {
        assert_eq!(scalar_bit_width(&LlType::Float), Some(32));
        assert_eq!(scalar_bit_width(&LlType::Int(32)), Some(32));
        assert_eq!(scalar_bit_width(&LlType::Half), Some(16));
        assert_eq!(scalar_bit_width(&LlType::BFloat), Some(16));
        assert_eq!(scalar_bit_width(&LlType::Int(16)), Some(16));
        assert_eq!(scalar_bit_width(&LlType::Int(8)), Some(8));
        assert_eq!(scalar_bit_width(&LlType::Bool), None);
        assert_eq!(scalar_bit_width(&vec(LlType::Float, 4)), None);
        assert_eq!(scalar_bit_width(&LlType::Ptr(1)), None);
    }

    #[test]
    fn whole_part_upgrades_only_matching_element_composite() {
        assert!(whole_part_widens(&vec(LlType::Float, 4), &LlType::Float));
        assert!(whole_part_widens(&arr(LlType::Half, 8), &LlType::Half));
        assert!(whole_part_widens(
            &vec(LlType::Int(32), 2),
            &LlType::Int(32)
        ));
        assert!(!whole_part_widens(&vec(LlType::Int(32), 4), &LlType::Float));
        assert!(!whole_part_widens(&LlType::Float, &LlType::Float));
        assert!(!whole_part_widens(
            &LlType::Struct(vec![LlType::Float]),
            &LlType::Float
        ));
    }

    #[test]
    fn reinterp_requires_same_width_different_scalar() {
        assert!(reinterp_compatible(&LlType::Int(32), &LlType::Float));
        assert!(reinterp_compatible(&LlType::Float, &LlType::Int(32)));
        assert!(reinterp_compatible(&LlType::Int(16), &LlType::Half));
        assert!(reinterp_compatible(&LlType::Half, &LlType::Int(16)));
        assert!(!reinterp_compatible(&LlType::Float, &LlType::Float));
        assert!(!reinterp_compatible(&LlType::Int(16), &LlType::Float));
        assert!(!reinterp_compatible(&LlType::Half, &LlType::Int(32)));
        assert!(!reinterp_compatible(&vec(LlType::Float, 4), &LlType::Float));
        assert!(!reinterp_compatible(&LlType::Bool, &LlType::Int(8)));
    }
}
