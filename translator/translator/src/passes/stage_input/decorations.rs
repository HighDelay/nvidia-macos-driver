use super::*;
use crate::meta::{VaryingInterpolation, VaryingSampling};

pub(in crate::passes) fn decorate_location(module: &mut Module, id: Word, loc: u32) {
    module.annotations.push(Instruction::new(
        Op::Decorate,
        None,
        None,
        vec![
            Operand::IdRef(id),
            Operand::Decoration(Decoration::Location),
            Operand::LiteralBit32(loc),
        ],
    ));
}

pub(in crate::passes) fn decorate_flat(module: &mut Module, id: Word) {
    module.annotations.push(Instruction::new(
        Op::Decorate,
        None,
        None,
        vec![Operand::IdRef(id), Operand::Decoration(Decoration::Flat)],
    ));
}

pub(in crate::passes) fn decorate_interpolation(
    module: &mut Module,
    id: Word,
    interpolation: VaryingInterpolation,
    must_flat: bool,
) {
    if must_flat || interpolation.flat {
        decorate_flat(module, id);
        return;
    }
    if interpolation.no_perspective {
        decorate_with(module, id, Decoration::NoPerspective);
    }
    match interpolation.sampling {
        VaryingSampling::Center => {}
        VaryingSampling::Centroid => decorate_with(module, id, Decoration::Centroid),
        VaryingSampling::Sample => decorate_with(module, id, Decoration::Sample),
    }
}

pub(in crate::passes) fn decorate_with(module: &mut Module, id: Word, decoration: Decoration) {
    module.annotations.push(Instruction::new(
        Op::Decorate,
        None,
        None,
        vec![Operand::IdRef(id), Operand::Decoration(decoration)],
    ));
}

pub(in crate::passes) fn decorate_patch(module: &mut Module, id: Word) {
    module.annotations.push(Instruction::new(
        Op::Decorate,
        None,
        None,
        vec![Operand::IdRef(id), Operand::Decoration(Decoration::Patch)],
    ));
}

pub(in crate::passes) fn decorate_builtin(module: &mut Module, id: Word, b: BuiltIn) {
    module.annotations.push(Instruction::new(
        Op::Decorate,
        None,
        None,
        vec![
            Operand::IdRef(id),
            Operand::Decoration(Decoration::BuiltIn),
            Operand::BuiltIn(b),
        ],
    ));
}
