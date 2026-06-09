pub fn format_source(source: &str) -> String {
    let mut output = String::new();
    let mut indent = 0usize;
    let mut previous_blank = false;

    for raw_line in source.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            if !previous_blank && !output.is_empty() {
                output.push('\n');
            }
            previous_blank = true;
            continue;
        }

        let leading_closes = line.chars().take_while(|ch| *ch == '}').count();
        let line_indent = indent.saturating_sub(leading_closes);

        output.push_str(&"    ".repeat(line_indent));
        output.push_str(line);
        output.push('\n');

        let (opens, closes) = brace_delta(line);
        indent = line_indent
            .saturating_add(opens)
            .saturating_sub(closes.saturating_sub(leading_closes));
        previous_blank = false;
    }

    output
}

fn brace_delta(line: &str) -> (usize, usize) {
    let mut opens = 0usize;
    let mut closes = 0usize;
    let mut chars = line.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'/') {
            break;
        }
        if ch == '"' {
            in_string = true;
        } else if ch == '{' {
            opens += 1;
        } else if ch == '}' {
            closes += 1;
        }
    }

    (opens, closes)
}

#[cfg(test)]
mod tests {
    use super::format_source;

    #[test]
    fn formats_brace_indentation_and_collapses_blank_lines() {
        let source = "function main(){\nprint(\"{\")\n\n\nif (true) {\nprint(1)\n}\n}\n";
        assert_eq!(
            format_source(source),
            "function main(){\n    print(\"{\")\n\n    if (true) {\n        print(1)\n    }\n}\n"
        );
    }

    #[test]
    fn keeps_else_body_indented_after_close_open_line() {
        let source = "function factorial(n: number) -> number {\nif (n <= 1) {\nreturn 1\n} else {\nreturn n * factorial(n - 1)\n}\n}\n";
        assert_eq!(
            format_source(source),
            "function factorial(n: number) -> number {\n    if (n <= 1) {\n        return 1\n    } else {\n        return n * factorial(n - 1)\n    }\n}\n"
        );
    }
}
