#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FunctionConstant {
    pub index: u32,
    pub name: String,
    pub type_name: String,
    pub abi_type_encoding: String,
}

pub fn parse_function_constants(ll: &str) -> Vec<FunctionConstant> {
    declared_function_constants(ll, |_| true)
}

pub(crate) fn function_constants_without_a_supplied_value(ll: &str) -> Vec<FunctionConstant> {
    declared_function_constants(ll, declares_no_value)
}

fn declared_function_constants(ll: &str, keep: impl Fn(&str) -> bool) -> Vec<FunctionConstant> {
    let mut out: Vec<FunctionConstant> = Vec::new();
    for line in ll.lines() {
        let t = line.trim_start();
        if !t.starts_with('@') || !t.contains(".MTL_FC_INIT_") {
            continue;
        }
        let Some(eq) = t.find(" = ") else {
            continue;
        };
        let Some((base, marker)) = t[1..eq].trim().split_once(".MTL_FC_INIT_") else {
            continue;
        };
        let digits: String = marker.chars().take_while(|c| c.is_ascii_digit()).collect();
        let Ok(index) = digits.parse::<u32>() else {
            continue;
        };
        let abi_type_encoding = marker
            .strip_prefix(&digits)
            .and_then(|suffix| suffix.strip_prefix('_'))
            .unwrap_or_default()
            .to_string();
        if out.iter().any(|f| f.index == index) {
            continue;
        }
        let declared = fc_global_decl_type_and_initializer(&t[eq + 3..]);
        if !keep(declared.as_ref().map_or("", |(_, initializer)| initializer)) {
            continue;
        }
        out.push(FunctionConstant {
            index,
            name: base.to_string(),
            type_name: declared.map(|(type_name, _)| type_name).unwrap_or_default(),
            abi_type_encoding,
        });
    }
    out.sort_by_key(|f| f.index);
    out
}

fn declares_no_value(initializer: &str) -> bool {
    initializer
        .strip_prefix("undef")
        .is_some_and(|rest| !rest.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
}

fn fc_global_decl_type_and_initializer(decl: &str) -> Option<(String, &str)> {
    let after = decl
        .split(" constant ")
        .nth(1)
        .or_else(|| decl.split(" global ").nth(1))?;
    let s = after.trim_start();
    let (type_name, rest) = if let Some(rest) = s.strip_prefix('<') {
        let end = rest.find('>')?;
        (format!("<{}>", &rest[..end]), &rest[end + 1..])
    } else {
        let token = s.split_whitespace().next()?;
        (token.to_string(), &s[token.len()..])
    };
    Some((type_name, rest.trim_start()))
}
