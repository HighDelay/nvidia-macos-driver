use super::*;

mod loop_continue;

#[cfg(test)]
mod tests {
    #[test]
    fn typed_terminator_strips_loop_metadata_off_branch_target() {
        use crate::native::tir::{parse_terminator, TirTerminator};
        assert_eq!(
            parse_terminator("br label %59, !llvm.loop !47"),
            Some(TirTerminator::Br("%59".to_string()))
        );
        assert_eq!(
            parse_terminator("br i1 %c, label %t, label %f, !llvm.loop !9"),
            Some(TirTerminator::BrCond {
                cond: "%c".to_string(),
                t: "%t".to_string(),
                f: "%f".to_string(),
            })
        );
    }
}
