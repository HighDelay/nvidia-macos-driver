pub(crate) const AIR_STATIC_INIT_SECTION: &str = "air.static_init";

pub(crate) fn define_line_declares_static_initializer(line: &str) -> bool {
    attribute_tail(line).is_some_and(tail_declares_static_init_section)
}

pub(crate) fn tail_declares_static_init_section(tail: &str) -> bool {
    let mut rest = tail;
    while let Some(at) = rest.find("section ") {
        let after = rest[at + "section ".len()..].trim_start();
        if let Some(quoted) = after.strip_prefix('"') {
            if let Some(end) = quoted.find('"') {
                if &quoted[..end] == AIR_STATIC_INIT_SECTION {
                    return true;
                }
            }
        }
        rest = &rest[at + "section ".len()..];
    }
    false
}

fn attribute_tail(line: &str) -> Option<&str> {
    let mut depth = 0usize;
    let mut in_quotes = false;
    let mut escaped = false;
    for (offset, ch) in line.char_indices() {
        if in_quotes {
            match ch {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_quotes = false,
                _ => {}
            }
            continue;
        }
        match ch {
            '"' => in_quotes = true,
            '(' => depth += 1,
            ')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(&line[offset + 1..]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_air_section_is_what_marks_an_initializer() {
        assert!(define_line_declares_static_initializer(
            "define internal void @_GLOBAL__sub_I_x() #0 section \"air.static_init\" {"
        ));
        assert!(
            define_line_declares_static_initializer(
                "define internal void @ctor() section \"air.static_init\" {"
            ),
            "the section, not the Itanium name, is the marker"
        );
        assert!(
            !define_line_declares_static_initializer(
                "define internal void @_GLOBAL__sub_I_x() #0 {"
            ),
            "the Itanium name alone must not stand in for the section"
        );
        assert!(!define_line_declares_static_initializer(
            "define internal void @f() section \"air.fc_initializer\" {"
        ));
    }

    #[test]
    fn the_marker_is_only_read_from_the_attribute_tail() {
        assert!(
            !define_line_declares_static_initializer(
                "define void @\"section \\\"air.static_init\\\"\"(i32 %x) {"
            ),
            "a quoted symbol name is not an attribute"
        );
        assert!(
            !define_line_declares_static_initializer(
                "define void @f(ptr %p, ptr %section \"air.static_init\") {"
            ),
            "the parameter list is not the attribute tail"
        );
        assert!(
            define_line_declares_static_initializer(
                "define void @\"odd(name)\"() section \"air.static_init\" {"
            ),
            "parentheses inside a quoted symbol must not unbalance the scan"
        );
    }

    #[test]
    fn a_line_without_a_balanced_parameter_list_declares_nothing() {
        assert!(!define_line_declares_static_initializer(
            "define void @f( section \"air.static_init\""
        ));
        assert!(!define_line_declares_static_initializer(""));
    }
}
