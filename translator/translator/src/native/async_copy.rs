use super::air_text::{split_args, LineBuffer};

struct AsyncCopyCall {
    elem: u32,
    dst: String,
    dst_epr: String,
    dst_dims: String,
    src: String,
    src_epr: String,
    src_dims: String,
    offset: String,
    result: String,
}

pub(crate) fn lower_simdgroup_async_copy(san_ll: &str) -> std::borrow::Cow<'_, str> {
    let has_copy_call = san_ll.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.contains("air.simdgroup_async_copy_2d") && trimmed.contains("call")
    });
    if !has_copy_call {
        return std::borrow::Cow::Borrowed(san_ll);
    }

    let mut elem_sizes: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
    let rewrite_capacity = san_ll.len().saturating_add(san_ll.len() / 4);
    let mut out = LineBuffer::with_capacity(rewrite_capacity);
    let mut fresh = fresh_counter(san_ll);

    for line in san_ll.lines() {
        let trimmed = line.trim_start();
        if trimmed.contains("air.simdgroup_async_copy_2d") && trimmed.contains("call") {
            match parse_async_copy(trimmed) {
                Some(call) => {
                    elem_sizes.insert(call.elem);
                    let indent = &line[..line.len() - trimmed.len()];
                    emit_copy_call(&mut out, indent, &call, &mut fresh);
                    continue;
                }
                None => {
                    out.push(line);
                    continue;
                }
            }
        }
        if trimmed.contains("air.is_null_simdgroup_event") && trimmed.contains("call") {
            if let Some(res) = call_result_id(trimmed) {
                let indent = &line[..line.len() - trimmed.len()];
                out.push_fmt(format_args!("{indent}{res} = icmp ne i64 0, 0"));
                continue;
            }
        }
        if trimmed.contains("air.get_null_simdgroup_event") && trimmed.contains("call") {
            if let Some(res) = call_result_id(trimmed) {
                let indent = &line[..line.len() - trimmed.len()];
                out.push_fmt(format_args!("{indent}{res} = inttoptr i64 0 to ptr"));
                continue;
            }
        }
        if trimmed.contains("air.wait_simdgroup_events") && trimmed.contains("call") {
            let indent = &line[..line.len() - trimmed.len()];
            out.push_fmt(format_args!(
                "{indent}call void @air.wg.barrier(i32 2, i32 1)"
            ));
            continue;
        }
        out.push(line);
    }

    if san_ll.ends_with('\n') {
        out.text.push('\n');
    }
    for &elem in &elem_sizes {
        out.text.push_str(&helper_definition(elem));
    }
    if !san_ll.contains("declare void @air.wg.barrier") {
        out.text
            .push_str("\ndeclare void @air.wg.barrier(i32, i32)\n");
    }
    std::borrow::Cow::Owned(out.text)
}

pub(crate) fn lower_simdgroup_async_copy_owned(san_ll: String) -> String {
    match lower_simdgroup_async_copy(&san_ll) {
        std::borrow::Cow::Borrowed(_) => san_ll,
        std::borrow::Cow::Owned(lowered) => lowered,
    }
}

fn emit_copy_call(out: &mut LineBuffer, indent: &str, call: &AsyncCopyCall, fresh: &mut u64) {
    let mut id = || {
        let v = format!("%__ac{}", *fresh);
        *fresh += 1;
        v
    };
    let (dw, dh, sw, sh, ox, oy) = (id(), id(), id(), id(), id(), id());
    out.push_fmt(format_args!(
        "{indent}{dw} = extractelement <2 x i64> {}, i64 0",
        call.dst_dims
    ));
    out.push_fmt(format_args!(
        "{indent}{dh} = extractelement <2 x i64> {}, i64 1",
        call.dst_dims
    ));
    out.push_fmt(format_args!(
        "{indent}{sw} = extractelement <2 x i64> {}, i64 0",
        call.src_dims
    ));
    out.push_fmt(format_args!(
        "{indent}{sh} = extractelement <2 x i64> {}, i64 1",
        call.src_dims
    ));
    out.push_fmt(format_args!(
        "{indent}{ox} = extractelement <2 x i64> {}, i64 0",
        call.offset
    ));
    out.push_fmt(format_args!(
        "{indent}{oy} = extractelement <2 x i64> {}, i64 1",
        call.offset
    ));
    out.push_fmt(format_args!(
        "{indent}call void @__metal2vulkan_sac2d_e{}(ptr addrspace(3) {}, i64 {}, i64 {dw}, i64 {dh}, ptr addrspace(1) {}, i64 {}, i64 {sw}, i64 {sh}, i64 {ox}, i64 {oy})",
        call.elem, call.dst, call.dst_epr, call.src, call.src_epr
    ));
    out.push_fmt(format_args!(
        "{indent}{} = inttoptr i64 1 to ptr",
        call.result
    ));
}

fn helper_definition(elem: u32) -> String {
    let ty = match elem {
        2 => "i16",
        _ => "i32",
    };
    format!(
        r#"
define internal void @__metal2vulkan_sac2d_e{elem}(ptr addrspace(3) %dst, i64 %depr, i64 %dw, i64 %dh, ptr addrspace(1) %src, i64 %sepr, i64 %sw, i64 %sh, i64 %ox, i64 %oy) {{
entry:
  br label %rowhead
rowhead:
  %r = phi i64 [ 0, %entry ], [ %rnext, %rowlatch ]
  %rok = icmp ult i64 %r, %dh
  br i1 %rok, label %colhead, label %done
colhead:
  %c = phi i64 [ 0, %rowhead ], [ %cnext, %collatch ]
  %cok = icmp ult i64 %c, %dw
  br i1 %cok, label %body, label %rowlatch
body:
  %sx = add i64 %ox, %c
  %sy = add i64 %oy, %r
  %xin = icmp ult i64 %sx, %sw
  %yin = icmp ult i64 %sy, %sh
  %in = and i1 %xin, %yin
  %didx = mul i64 %r, %depr
  %didx2 = add i64 %didx, %c
  %dp = getelementptr {ty}, ptr addrspace(3) %dst, i64 %didx2
  br i1 %in, label %copy, label %zero
copy:
  %syr = mul i64 %sy, %sepr
  %sidx = add i64 %syr, %sx
  %sp = getelementptr {ty}, ptr addrspace(1) %src, i64 %sidx
  %v = load {ty}, ptr addrspace(1) %sp
  store {ty} %v, ptr addrspace(3) %dp
  br label %collatch
zero:
  store {ty} 0, ptr addrspace(3) %dp
  br label %collatch
collatch:
  %cnext = add i64 %c, 1
  br label %colhead
rowlatch:
  %rnext = add i64 %r, 1
  br label %rowhead
done:
  ret void
}}
"#
    )
}

fn parse_async_copy(line: &str) -> Option<AsyncCopyCall> {
    let result = line.split('=').next()?.trim().to_string();
    if !result.starts_with('%') {
        return None;
    }
    let args = split_args(super::air_text::call_arguments(line)?);
    if args.len() != 12 {
        return None;
    }
    let elem: u32 = operand_value(&args[0]).parse().ok()?;
    Some(AsyncCopyCall {
        elem,
        dst: operand_value(&args[2]),
        dst_epr: operand_value(&args[3]),
        dst_dims: operand_value(&args[5]),
        src: operand_value(&args[6]),
        src_epr: operand_value(&args[7]),
        src_dims: operand_value(&args[9]),
        offset: operand_value(&args[10]),
        result,
    })
}

fn operand_value(arg: &str) -> String {
    let a = arg.trim();
    if let Some(rest) = a.strip_prefix("<2 x i64>") {
        return rest.trim().to_string();
    }
    super::air_text::last_token(a).to_string()
}

fn call_result_id(line: &str) -> Option<String> {
    let head = line.split('=').next()?.trim();
    head.starts_with('%').then(|| head.to_string())
}

fn fresh_counter(_san_ll: &str) -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::lower_simdgroup_async_copy;
    use std::borrow::Cow;

    #[test]
    fn no_async_copy_borrows_the_original_module() {
        let ll = "define void @k() {\nentry:\n  ret void\n}\n";
        assert!(matches!(
            lower_simdgroup_async_copy(ll),
            Cow::Borrowed(value) if std::ptr::eq(value, ll)
        ));
    }

    #[test]
    fn dead_async_copy_declaration_borrows_the_already_lowered_module() {
        let ll = concat!(
            "define void @k() {\nentry:\n  ret void\n}\n",
            "declare ptr @air.simdgroup_async_copy_2d.p3i8.p1i8(i64)\n",
        );
        assert!(matches!(
            lower_simdgroup_async_copy(ll),
            Cow::Borrowed(value) if std::ptr::eq(value, ll)
        ));
    }

    #[test]
    fn explicit_null_event_lowers_with_the_async_copy_family() {
        let ll = concat!(
            "%null = call ptr @air.get_null_simdgroup_event()\n",
            "%event = call ptr @air.simdgroup_async_copy_2d.p3i8.p1i8(i64 2, i64 2, ptr addrspace(3) %dst, i64 8, i64 1, <2 x i64> <i64 4, i64 4>, ptr addrspace(1) %src, i64 8, i64 1, <2 x i64> <i64 4, i64 4>, <2 x i64> zeroinitializer, i32 0)\n",
            "%is_null = call i1 @air.is_null_simdgroup_event(ptr %event)\n",
            "call void @air.wait_simdgroup_events(i32 1, ptr %event)\n",
        );
        let lowered = lower_simdgroup_async_copy(ll);
        assert!(lowered.contains("%null = inttoptr i64 0 to ptr"));
        assert!(lowered.contains("%event = inttoptr i64 1 to ptr"));
        assert!(lowered.contains("%is_null = icmp ne i64 0, 0"));
        assert!(!lowered.contains("call ptr @air.get_null_simdgroup_event"));
    }
}
