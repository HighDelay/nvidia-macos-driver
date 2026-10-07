use std::borrow::Cow;

fn air_op(op: &str) -> Option<&'static str> {
    Some(match op {
        "add" => "add.u",
        "sub" => "sub.u",
        "and" => "and.u",
        "or" => "or.u",
        "xor" => "xor.u",
        "xchg" => "xchg",
        "max" => "max.s",
        "min" => "min.s",
        "umax" => "max.u",
        "umin" => "min.u",
        _ => return None,
    })
}

fn rewrite_line(line: &str) -> Option<(String, String, String)> {
    let indent = &line[..line.len() - line.trim_start().len()];
    let (result, rhs) = line.trim().split_once(" = atomicrmw ")?;
    if !result.starts_with('%') || result.contains(' ') {
        return None;
    }
    let rhs = rhs.strip_prefix("volatile ").unwrap_or(rhs);
    let (op, rest) = rhs.split_once(' ')?;
    let op = air_op(op)?;
    let (pty, space, rest) = if let Some(r) = rest.strip_prefix("i32 addrspace(3)* ") {
        ("i32 addrspace(3)*", "local", r)
    } else if let Some(r) = rest.strip_prefix("i32 addrspace(1)* ") {
        ("i32 addrspace(1)*", "global", r)
    } else if let Some(r) = rest.strip_prefix("ptr addrspace(3) ") {
        ("ptr addrspace(3)", "local", r)
    } else {
        let r = rest.strip_prefix("ptr addrspace(1) ")?;
        ("ptr addrspace(1)", "global", r)
    };
    let (ptr, rest) = rest.split_once(", i32 ")?;
    if ptr.contains(' ') && !ptr.starts_with("@\"") {
        return None;
    }
    let (value, tail) = rest.split_once(' ')?;
    let tail = tail.split(", align").next()?.trim();
    let order = tail.rsplit(' ').next()?;
    if !matches!(
        order,
        "monotonic" | "acquire" | "release" | "acq_rel" | "seq_cst"
    ) {
        return None;
    }
    if tail != order && !tail.starts_with("syncscope(") {
        return None;
    }
    let callee = format!("air.atomic.{space}.{op}.i32");
    Some((
        format!("{indent}{result} = call i32 @{callee}({pty} {ptr}, i32 {value}, i32 0, i32 2, i1 true)"),
        callee,
        pty.to_string(),
    ))
}

pub(crate) fn lower_raw_atomicrmw(san_ll: &str) -> Cow<'_, str> {
    if !san_ll.contains(" = atomicrmw ") {
        return Cow::Borrowed(san_ll);
    }
    let mut out = String::with_capacity(san_ll.len() + 256);
    let mut declares: Vec<(String, String)> = Vec::new();
    let mut changed = false;
    for line in san_ll.split_inclusive('\n') {
        let (body, nl) = match line.strip_suffix('\n') {
            Some(b) => (b, "\n"),
            None => (line, ""),
        };
        match rewrite_line(body) {
            Some((rewritten, callee, pty)) => {
                out.push_str(&rewritten);
                out.push_str(nl);
                if !declares.iter().any(|(c, _)| *c == callee) {
                    declares.push((callee, pty));
                }
                changed = true;
            }
            None => out.push_str(line),
        }
    }
    if !changed {
        return Cow::Borrowed(san_ll);
    }
    for (callee, pty) in declares {
        if !san_ll.contains(&format!("@{callee}(")) {
            out.push_str(&format!(
                "\ndeclare i32 @{callee}({pty}, i32, i32, i32, i1)\n"
            ));
        }
    }
    Cow::Owned(out)
}

pub(crate) fn lower_raw_atomicrmw_owned(san_ll: String) -> String {
    match lower_raw_atomicrmw(&san_ll) {
        Cow::Borrowed(_) => san_ll,
        Cow::Owned(owned) => owned,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_groupshared_interlocked_add_becomes_the_air_local_atomic() {
        let src = "  %5 = atomicrmw add i32 addrspace(3)* %4, i32 1 seq_cst, align 4\n";
        let out = lower_raw_atomicrmw(src);
        assert!(out.contains("  %5 = call i32 @air.atomic.local.add.u.i32(i32 addrspace(3)* %4, i32 1, i32 0, i32 2, i1 true)"), "{out}");
        assert!(
            out.contains(
                "declare i32 @air.atomic.local.add.u.i32(i32 addrspace(3)*, i32, i32, i32, i1)"
            ),
            "{out}"
        );
    }

    #[test]
    fn a_named_global_umin_keeps_its_quoted_symbol() {
        let src =
            "  %9 = atomicrmw umin i32 addrspace(3)* @\"\\01?s_MinDepth1@@3IA\", i32 %8 seq_cst\n";
        let out = lower_raw_atomicrmw(src);
        assert!(
            out.contains(
                "@air.atomic.local.min.u.i32(i32 addrspace(3)* @\"\\01?s_MinDepth1@@3IA\", i32 %8,"
            ),
            "{out}"
        );
    }

    #[test]
    fn a_device_memory_atomic_takes_the_global_abi() {
        let out = lower_raw_atomicrmw("%r = atomicrmw or ptr addrspace(1) %p, i32 %v monotonic\n");
        assert!(
            out.contains("@air.atomic.global.or.u.i32(ptr addrspace(1) %p, i32 %v,"),
            "{out}"
        );
    }

    #[test]
    fn a_64_bit_atomic_is_left_untouched() {
        let src = "  %5 = atomicrmw add i64 addrspace(3)* %4, i64 1 seq_cst\n";
        assert!(matches!(lower_raw_atomicrmw(src), Cow::Borrowed(_)));
    }

    #[test]
    fn a_float_atomic_is_left_untouched() {
        let src = "  %5 = atomicrmw fadd float addrspace(3)* %4, float 1.0 seq_cst\n";
        assert!(matches!(lower_raw_atomicrmw(src), Cow::Borrowed(_)));
    }

    #[test]
    fn a_threadgroup_atomic_in_another_address_space_is_left_untouched() {
        let src = "  %5 = atomicrmw add i32* %4, i32 1 seq_cst\n";
        assert!(matches!(lower_raw_atomicrmw(src), Cow::Borrowed(_)));
    }
}
