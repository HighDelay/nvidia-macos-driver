const ROLE: &str = ", !\"air.constant\", !\"air.location_index\", i32 ";

pub(crate) fn lower_cl_constant_args(san_ll: &str) -> std::borrow::Cow<'_, str> {
    match lower_cl_constant_nodes(san_ll) {
        std::borrow::Cow::Borrowed(borrowed) => lower_cl_int24(borrowed),
        std::borrow::Cow::Owned(owned) => match lower_cl_int24(&owned) {
            std::borrow::Cow::Borrowed(_) => std::borrow::Cow::Owned(owned),
            std::borrow::Cow::Owned(lowered) => std::borrow::Cow::Owned(lowered),
        },
    }
}

fn lower_cl_constant_nodes(san_ll: &str) -> std::borrow::Cow<'_, str> {
    if !san_ll.contains(ROLE) {
        return std::borrow::Cow::Borrowed(san_ll);
    }
    let mut out = String::with_capacity(san_ll.len() + 256);
    for line in san_ll.split_inclusive('\n') {
        match rewrite_node(line) {
            Some(rewritten) => out.push_str(&rewritten),
            None => out.push_str(line),
        }
    }
    std::borrow::Cow::Owned(out)
}

fn rewrite_node(line: &str) -> Option<String> {
    if !line.trim_start().starts_with('!') {
        return None;
    }
    let at = line.find(ROLE)?;
    let after = &line[at + ROLE.len()..];
    let (index, rest) = after.split_once(", ")?;
    if index.is_empty() || !index.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let rest = rest.strip_prefix("i32 1")?;
    Some(format!(
        "{}, !\"air.buffer\", !\"air.location_index\", i32 {index}, i32 1, !\"air.read\", !\"air.address_space\", i32 2{rest}",
        &line[..at]
    ))
}

pub(crate) fn lower_cl_int24(san_ll: &str) -> std::borrow::Cow<'_, str> {
    if !san_ll.contains("@air.mul24.") && !san_ll.contains("@air.mad24.") {
        return std::borrow::Cow::Borrowed(san_ll);
    }
    let mut out = String::with_capacity(san_ll.len());
    for line in san_ll.split_inclusive('\n') {
        match rewrite_int24(line) {
            Some(rewritten) => out.push_str(&rewritten),
            None => out.push_str(line),
        }
    }
    std::borrow::Cow::Owned(out)
}

fn rewrite_int24(line: &str) -> Option<String> {
    let (mad, at) = match (line.find("@air.mul24."), line.find("@air.mad24.")) {
        (Some(at), _) => (false, at),
        (None, Some(at)) => (true, at),
        _ => return None,
    };
    let (lhs, rhs) = line.split_once(" = ")?;
    let indent: String = lhs.chars().take_while(|c| c.is_whitespace()).collect();
    let res = lhs.trim();
    let call = rhs.find("call ")?;
    let ty = rhs[call + 5..at - (line.len() - rhs.len())].trim();
    let open = line[at..].find('(')? + at;
    let close = line.rfind(')')?;
    let args: Vec<String> = line[open + 1..close]
        .split(',')
        .map(|arg| arg.split_whitespace().last().unwrap_or("").to_string())
        .collect();
    let nl = if line.ends_with('\n') { "\n" } else { "" };
    match (mad, args.as_slice()) {
        (false, [a, b]) => Some(format!("{indent}{res} = mul {ty} {a}, {b}{nl}")),
        (true, [a, b, c]) => Some(format!(
            "{indent}{res}.m24 = mul {ty} {a}, {b}\n{indent}{res} = add {ty} {res}.m24, {c}{nl}"
        )),
        _ => None,
    }
}

#[cfg(test)]
mod int24_tests {
    use super::lower_cl_int24;

    #[test]
    fn a_cl_mul24_is_a_plain_multiply() {
        let src = "  %7 = tail call i32 @air.mul24.u.i32(i32 %5, i32 noundef %6) #3\n";
        assert_eq!(lower_cl_int24(src), "  %7 = mul i32 %5, %6\n");
    }

    #[test]
    fn a_vector_mad24_is_a_multiply_then_add() {
        let src = "  %r = call <4 x i32> @air.mad24.s.v4i32(<4 x i32> %a, <4 x i32> %b, <4 x i32> %c)\n";
        assert_eq!(
            lower_cl_int24(src),
            "  %r.m24 = mul <4 x i32> %a, %b\n  %r = add <4 x i32> %r.m24, %c\n"
        );
    }

    #[test]
    fn the_declaration_is_left_alone() {
        let src = "declare i32 @air.mul24.u.i32(i32, i32)\n";
        assert_eq!(lower_cl_int24(src), src);
    }
}

#[cfg(test)]
mod tests {
    use super::lower_cl_constant_args;

    #[test]
    fn a_cl_by_value_argument_becomes_a_read_only_constant_buffer_at_its_index() {
        let src = "!20 = !{i32 3, !\"air.constant\", !\"air.location_index\", i32 0, i32 1, !\"air.arg_type_size\", i32 4, !\"air.arg_type_align_size\", i32 4, !\"air.arg_type_name\", !\"int\", !\"air.arg_name\", !\"kernelWidth\"}\n";
        let out = lower_cl_constant_args(src);
        assert_eq!(
            out,
            "!20 = !{i32 3, !\"air.buffer\", !\"air.location_index\", i32 0, i32 1, !\"air.read\", !\"air.address_space\", i32 2, !\"air.arg_type_size\", i32 4, !\"air.arg_type_align_size\", i32 4, !\"air.arg_type_name\", !\"int\", !\"air.arg_name\", !\"kernelWidth\"}\n"
        );
    }

    #[test]
    fn a_module_without_cl_arguments_is_borrowed() {
        let src = "!3 = !{i32 0, !\"air.buffer\", !\"air.location_index\", i32 0, i32 1, !\"air.read\"}\n";
        assert!(matches!(lower_cl_constant_args(src), std::borrow::Cow::Borrowed(_)));
    }

    #[test]
    fn code_that_mentions_the_role_is_untouched() {
        let src = "  ; , !\"air.constant\", !\"air.location_index\", i32 0, i32 1\n";
        assert_eq!(lower_cl_constant_args(src), src);
    }
}
