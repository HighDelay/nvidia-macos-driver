use super::*;
use crate::native::parse::{parse_memory_alignment, split_top_level};

#[cfg(test)]
pub(in crate::native) fn parse_block_label(line: &str) -> Option<String> {
    let head = line.split_whitespace().next()?;
    let name = head.strip_suffix(':')?;
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        return None;
    }
    if name.starts_with('%') {
        Some(name.to_string())
    } else {
        Some(format!("%{name}"))
    }
}

pub(in crate::native) fn parse_terminator(line: &str) -> Option<TirTerminator> {
    let line = line.trim();
    if line == "unreachable" {
        return Some(TirTerminator::Unreachable);
    }
    if let Some(rest) = line.strip_prefix("ret ") {
        let rest = rest.split(", !").next().unwrap_or(rest).trim();
        if rest == "void" {
            return Some(TirTerminator::Ret(None));
        }
        return Some(TirTerminator::Ret(
            rest.split_whitespace().last().map(str::to_string),
        ));
    }
    if let Some(rest) = line.strip_prefix("br ") {
        let labels = collect_labels(rest);
        match labels.len() {
            1 => return Some(TirTerminator::Br(labels[0].clone())),
            2 => {
                let head = &rest[..rest.find("label ").unwrap_or(rest.len())];
                let cond = head
                    .trim()
                    .trim_end_matches(',')
                    .split_whitespace()
                    .last()?
                    .to_string();
                return Some(TirTerminator::BrCond {
                    cond,
                    t: labels[0].clone(),
                    f: labels[1].clone(),
                });
            }
            _ => return None,
        }
    }
    if let Some(rest) = line.strip_prefix("switch ") {
        return parse_switch(rest);
    }
    None
}

pub(in crate::native) fn collect_labels(s: &str) -> Vec<String> {
    s.split("label ")
        .skip(1)
        .filter_map(|chunk| chunk.split([',', ' ', '\t']).next())
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

pub(in crate::native) fn parse_switch(rest: &str) -> Option<TirTerminator> {
    let open = rest.find('[')?;
    let close = rest.rfind(']')?;
    let head = &rest[..open];
    let head_parts = split_top_level(head, ',');
    if head_parts.len() < 2 {
        return None;
    }
    let selector = head_parts[0].split_whitespace().last()?.to_string();
    let default = head_parts[1]
        .trim()
        .strip_prefix("label ")?
        .trim()
        .to_string();
    let mut cases = Vec::new();
    let mut body = rest[open + 1..close].trim();
    while !body.is_empty() {
        let (value_text, after_value) = body.split_once(',')?;
        let constant = value_text.split_whitespace().last()?.to_string();
        let after_label = after_value.trim().strip_prefix("label ")?;
        let label_end = after_label
            .find(char::is_whitespace)
            .unwrap_or(after_label.len());
        let label = after_label[..label_end].to_string();
        body = after_label[label_end..].trim();
        cases.push((constant, label));
    }
    Some(TirTerminator::Switch {
        selector,
        default,
        cases,
    })
}

pub(in crate::native) const OPERAND_FLAG_TOKENS: &[&str] = &[
    "nsw", "nuw", "exact", "fast", "nnan", "ninf", "nsz", "arcp", "contract", "afn", "reassoc",
    "disjoint", "volatile",
];

pub(in crate::native) fn resolve_cmp_predicate(line: &str) -> Option<String> {
    let rhs = rhs_of(line);
    let opcode = rhs.split_whitespace().next()?;
    if opcode != "icmp" && opcode != "fcmp" {
        return None;
    }
    let after_opcode = rhs[opcode.len()..].trim_start();
    skip_flag_tokens(after_opcode)
        .split_whitespace()
        .next()
        .map(|t| t.to_string())
}

pub(in crate::native) fn resolve_mem_align(line: &str) -> Option<u64> {
    let rhs = rhs_of(line);
    let opcode = rhs.split_whitespace().next()?;
    if opcode != "load" && opcode != "store" {
        return None;
    }
    let after_opcode = rhs[opcode.len()..].trim_start();
    let parts = split_top_level(after_opcode, ',');
    parse_memory_alignment(parts.get(2..).unwrap_or(&[]))
        .ok()
        .flatten()
}
