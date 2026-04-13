pub fn translate_simple_if_else(action: &str) -> Option<String> {
    let trimmed = action.trim();

    if !trimmed.starts_with("if") {
        return None;
    }

    let parts: Vec<&str> = trimmed.split(" else ").collect();
    if parts.len() != 2 {
        return None;
    }

    let if_part = parts[0].trim();
    let else_part = parts[1].trim();

    let if_body_start = if_part.find('{')? + 1;
    let if_body_end = if_part.rfind('}')?;
    let if_body = if_part[if_body_start..if_body_end].trim();

    let else_body_start = else_part.find('{')? + 1;
    let else_body_end = else_part.rfind('}')?;
    let else_body = else_part[else_body_start..else_body_end].trim();

    let cond_start = if_part.find('(')?;
    let cond_end = if_part.find(')')?;
    let condition = &if_part[cond_start + 1..cond_end];

    let if_lines: Vec<&str> = if_body.lines().collect();
    let else_lines: Vec<&str> = else_body.lines().collect();

    let mut result = format!("if {}:\n", condition);

    for line in if_lines {
        let cleaned = line.trim().trim_end_matches(';');
        if !cleaned.is_empty() {
            if cleaned.starts_with("yyerror") {
                result.push_str("                        # ");
                result.push_str(cleaned);
                result.push('\n');
            } else {
                result.push_str("                        ");
                result.push_str(cleaned);
                result.push('\n');
            }
        }
    }

    result.push_str("                    else:\n");

    for line in else_lines {
        let cleaned = line.trim().trim_end_matches(';');
        if !cleaned.is_empty() {
            result.push_str("                        ");
            result.push_str(cleaned);
            result.push('\n');
        }
    }

    Some(result.replace("if ", "if "))
}

pub fn has_multiline_block(action: &str) -> bool {
    action.contains('{') && action.contains('}')
}

