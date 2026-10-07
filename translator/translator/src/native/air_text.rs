use std::fmt::Write as _;

pub(super) struct LineBuffer {
    pub(super) text: String,
    has_line: bool,
}

impl LineBuffer {
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            text: String::with_capacity(capacity),
            has_line: false,
        }
    }

    fn begin_line(&mut self) {
        if self.has_line {
            self.text.push('\n');
        }
        self.has_line = true;
    }

    pub(super) fn push(&mut self, line: &str) {
        self.begin_line();
        self.text.push_str(line);
    }

    pub(super) fn push_fmt(&mut self, line: std::fmt::Arguments<'_>) {
        self.begin_line();
        let _ = self.text.write_fmt(line);
    }
}

pub(super) fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for ch in s.chars() {
        match ch {
            '<' | '(' | '[' | '{' => {
                depth += 1;
                cur.push(ch);
            }
            '>' | ')' | ']' | '}' => {
                depth -= 1;
                cur.push(ch);
            }
            ',' if depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

pub(super) fn last_token(arg: &str) -> &str {
    arg.trim().rsplit(' ').next().unwrap_or("")
}

pub(super) fn call_arguments(line: &str) -> Option<&str> {
    let open = line.find('(')?;
    let mut depth = 0i32;
    for (offset, ch) in line[open..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&line[open + 1..open + offset]);
                }
            }
            _ => {}
        }
    }
    None
}
