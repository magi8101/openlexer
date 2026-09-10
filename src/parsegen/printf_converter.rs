pub struct FormatString {
    pub parts: Vec<FormatPart>,
}

#[derive(Debug, Clone)]
pub enum FormatPart {
    Literal(String),
    Specifier(char),
}

impl FormatString {
    pub fn parse(s: &str) -> Self {
        let mut parts = Vec::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '%' && i + 1 < chars.len() {
                match chars[i + 1] {
                    '%' => {
                        parts.push(FormatPart::Literal("%".to_string()));
                        i += 2;
                    }
                    'c' | 'd' | 'i' | 'o' | 'x' | 'X' | 'u' | 'e' | 'E' | 'f' | 'F' | 'g' | 'G'
                    | 's' => {
                        parts.push(FormatPart::Specifier(chars[i + 1]));
                        i += 2;
                    }
                    _ => {
                        let mut literal = String::from("%");
                        literal.push(chars[i + 1]);
                        parts.push(FormatPart::Literal(literal));
                        i += 2;
                    }
                }
            } else if chars[i] == '\\' && i + 1 < chars.len() {
                let mut esc = String::from("\\");
                esc.push(chars[i + 1]);
                parts.push(FormatPart::Literal(esc));
                i += 2;
            } else {
                let mut literal = String::new();
                while i < chars.len() && chars[i] != '%' && chars[i] != '\\' {
                    literal.push(chars[i]);
                    i += 1;
                }
                if !literal.is_empty() {
                    parts.push(FormatPart::Literal(literal));
                }
            }
        }

        FormatString { parts }
    }

    pub fn to_python(&self) -> (String, usize) {
        let mut fmt = String::new();
        let mut spec_count = 0;

        for part in &self.parts {
            match part {
                FormatPart::Literal(s) => {
                    fmt.push_str(s);
                }
                FormatPart::Specifier(_c) => {
                    fmt.push_str("{}");
                    spec_count += 1;
                }
            }
        }

        (fmt, spec_count)
    }

    pub fn to_java(&self) -> (String, usize) {
        let mut fmt = String::new();
        let mut spec_count = 0;

        for part in &self.parts {
            match part {
                FormatPart::Literal(s) => {
                    fmt.push_str(s);
                }
                FormatPart::Specifier(c) => {
                    fmt.push('%');
                    fmt.push(*c);
                    spec_count += 1;
                }
            }
        }

        (fmt, spec_count)
    }
}

pub fn extract_printf_call(action: &str) -> Option<(PressedAction, usize, usize)> {
    let start = action.find("printf(")?;
    let mut pos = start + 7;

    if pos >= action.len() || action.chars().nth(pos) != Some('"') {
        return None;
    }

    pos += 1;
    let mut fmt_str = String::new();

    let chars: Vec<char> = action.chars().collect();
    while pos < chars.len() && chars[pos] != '"' {
        if chars[pos] == '\\' && pos + 1 < chars.len() {
            fmt_str.push(chars[pos]);
            pos += 1;
            fmt_str.push(chars[pos]);
            pos += 1;
        } else {
            fmt_str.push(chars[pos]);
            pos += 1;
        }
    }

    if pos >= chars.len() {
        return None;
    }

    pos += 1;
    while pos < chars.len() && chars[pos] != ',' {
        pos += 1;
    }

    if pos >= chars.len() {
        return None;
    }

    pos += 1;
    while pos < chars.len() && chars[pos].is_whitespace() {
        pos += 1;
    }

    let mut args = String::new();
    let mut paren_depth = 0;
    let mut in_string = false;
    let mut escape_next = false;

    while pos < chars.len() {
        let c = chars[pos];

        if escape_next {
            args.push(c);
            escape_next = false;
            pos += 1;
            continue;
        }

        if c == '\\' && in_string {
            escape_next = true;
            args.push(c);
            pos += 1;
            continue;
        }

        if c == '"' && !in_string {
            in_string = true;
            args.push(c);
            pos += 1;
            continue;
        }

        if c == '"' && in_string {
            in_string = false;
            args.push(c);
            pos += 1;
            continue;
        }

        if !in_string {
            if c == '(' {
                paren_depth += 1;
            } else if c == ')' {
                if paren_depth == 0 {
                    break;
                }
                paren_depth -= 1;
            }
        }

        // Every character that reaches here is part of the argument list
        // (including nested '(' / ')', e.g. printf("%d", foo(x, y))) - only
        // the special-cased escape/quote branches above push-and-continue
        // before this point, and only the closing ')' of printf(...) itself
        // breaks before it.
        args.push(c);
        pos += 1;
    }

    // `pos` stops AT printf(...)'s closing ')' (not past it) unless the
    // input was malformed and we ran off the end looking for it - consume
    // that ')' too so the returned range covers the whole call and callers
    // splicing text around [start, end) don't leave a stray ')' behind.
    let end_pos = if pos < chars.len() && chars[pos] == ')' {
        pos + 1
    } else {
        pos
    };

    Some((
        PressedAction {
            fmt_str,
            args: args.trim().to_string(),
        },
        start,
        end_pos,
    ))
}

pub struct PressedAction {
    pub fmt_str: String,
    pub args: String,
}

pub fn convert_printf_to_python(fmt: &str, args: &str) -> String {
    let format = FormatString::parse(fmt);
    let (py_fmt, spec_count) = format.to_python();

    let arg_list: Vec<&str> = args.split(',').map(|s| s.trim()).collect();

    if spec_count == 0 {
        return format!("print(\"{}\")", py_fmt);
    }

    if arg_list.len() == spec_count {
        let args_str = arg_list.join(", ");
        format!("print(\"{}\".format({}))", py_fmt, args_str)
    } else {
        format!("print(\"{}\".format({}))", py_fmt, args)
    }
}

pub fn convert_printf_to_java(fmt: &str, args: &str) -> String {
    let format = FormatString::parse(fmt);
    let (java_fmt, _spec_count) = format.to_java();

    if args.is_empty() {
        format!("System.out.printf(\"{}\");", java_fmt)
    } else {
        format!("System.out.printf(\"{}\", {});", java_fmt, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_printf_call_single_arg() {
        // Regression test: the arg-collection loop used to track paren depth
        // correctly but never actually copy ordinary characters into `args`
        // (only the escape/quote special cases did), so args always came
        // back empty for anything but a bare string literal. And `end` used
        // to point AT printf(...)'s closing ')' rather than past it, so
        // splicing text around [start, end) left the original ')' behind.
        let action = r#"result = $1; printf("= %g\n", result);"#;
        let (parsed, start, end) = extract_printf_call(action).expect("should find printf(");
        assert_eq!(parsed.fmt_str, "= %g\\n");
        assert_eq!(parsed.args, "result");
        assert_eq!(&action[start..end], r#"printf("= %g\n", result)"#);
        assert_eq!(&action[end..], ";");
    }

    #[test]
    fn test_extract_printf_call_nested_call_arg() {
        // A nested function call in the argument list has its own parens;
        // those must be copied into `args` too, not just tracked for depth.
        let action = r#"printf("%d", foo(x, y));"#;
        let (parsed, _start, end) = extract_printf_call(action).expect("should find printf(");
        assert_eq!(parsed.args, "foo(x, y)");
        assert_eq!(&action[end..], ";");
    }

    #[test]
    fn test_convert_printf_to_python_single_arg() {
        let action = r#"printf("= %g\n", result);"#;
        let (parsed, _, _) = extract_printf_call(action).unwrap();
        let py = convert_printf_to_python(&parsed.fmt_str, &parsed.args);
        assert_eq!(py, "print(\"= {}\\n\".format(result))");
    }
}
