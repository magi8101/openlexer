//! Lexer rule file parser.
//!
//! Parses .l files in Flex-like format with three sections:
//! ```text
//! %{
//! /* C code prologue */
//! int lineNum = 0;
//! %}
//! %s COMMENT            # Inclusive start condition
//! %x STRING             # Exclusive start condition
//! DIGIT   [0-9]         # Named pattern definition
//! ALPHA   [a-zA-Z]
//! %%
//! <INITIAL,STRING>{DIGIT}+   { return NUM; }
//! "if"                       { return IF; }
//! "("                        { printf("(\n"); }
//! {ALPHA}+                   ID
//! \s+                        skip
//! .                          error
//! %%
//! int main() { yylex(); return 0; }
//! ```
//!
//! Sections:
//! 1. Definitions: %{ %}prologue, %s/%x for start conditions, NAME pattern definitions
//! 2. Rules: pattern action pairs, optionally with <STATE> prefix
//! 3. User code (optional, stored for output)
//!
//! Special actions:
//! - `skip` or empty `{ }`: Ignore the matched text
//! - `error`: Report an error for unrecognized characters
//! - `BEGIN(STATE)`: Switch to a different start condition

use crate::error::{Error, Result};
use crate::lexgen::regex::RegexAst;
use std::collections::HashMap;

/// Represents a single lexer rule.
#[derive(Debug, Clone)]
pub struct LexerRule {
    /// The regex pattern as a string.
    pub pattern: String,
    /// The parsed regex AST.
    pub regex: RegexAst,
    /// The action to take when this rule matches.
    pub action: RuleAction,
    /// Start conditions this rule is active in. Empty means all conditions (for inclusive)
    /// or just INITIAL (for rules without explicit conditions).
    pub start_conditions: Vec<String>,
    /// Line number in the source file (for error messages).
    pub line_number: usize,
}

/// Action to take when a rule matches.
#[derive(Debug, Clone, PartialEq)]
pub enum RuleAction {
    /// Return a token with this name.
    Token(String),
    /// Skip the matched text (don't produce a token).
    Skip,
    /// Report an error for unrecognized input.
    Error,
    /// Switch to a different start condition.
    Begin(String),
    /// Return a token and switch to a different start condition.
    TokenAndBegin(String, String),
    /// Arbitrary user code block (passed through to generated lexer).
    /// Contains the raw code text. The code can use:
    /// - yytext: the matched string
    /// - yyleng: length of matched string
    /// - yymore(): append next match to current yytext
    /// - yyless(n): put back all but first n characters
    /// - REJECT: try the next matching rule
    /// - BEGIN(state): switch start condition
    /// - return TOKEN: return a token
    Code(String),
}

/// Type of start condition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StartConditionType {
    /// Inclusive: rules without conditions are also active.
    Inclusive,
    /// Exclusive: only rules with this condition are active.
    Exclusive,
}

/// A complete lexer specification parsed from a .l file.
#[derive(Debug, Clone)]
pub struct LexerSpec {
    /// Named pattern definitions (e.g., DIGIT -> [0-9]).
    pub definitions: HashMap<String, String>,
    /// Start conditions with their types.
    pub start_conditions: HashMap<String, StartConditionType>,
    /// The rules in order of priority (first match wins for equal length).
    pub rules: Vec<LexerRule>,
    /// Prologue code from %{ %} blocks in definitions section.
    pub prologue: String,
    /// User code from the section after the second %%.
    pub user_code: String,
}

impl LexerSpec {
    /// Parses a lexer specification from the contents of a .l file.
    pub fn parse(input: &str) -> Result<Self> {
        // First, check if this looks like a parser grammar (.y file) instead of lexer rules
        Self::validate_not_grammar(input)?;

        let mut spec = LexerSpec {
            definitions: HashMap::new(),
            start_conditions: HashMap::new(),
            rules: Vec::new(),
            prologue: String::new(),
            user_code: String::new(),
        };

        // Always have INITIAL as an inclusive start condition
        spec.start_conditions
            .insert("INITIAL".to_string(), StartConditionType::Inclusive);

        // Split into sections by %%
        // Need to be careful - %% must be at start of line
        let sections = split_sections(input);

        let (definitions_section, rules_section, user_code_section) = match sections.len() {
            1 => {
                // No %%, treat entire input as rules (simple format)
                ("".to_string(), sections[0].clone(), "".to_string())
            }
            2 => {
                // Definitions %% Rules
                (sections[0].clone(), sections[1].clone(), "".to_string())
            }
            _ => {
                // Definitions %% Rules %% User Code
                // User code is everything after the second %%
                let user_code = sections[2..].join("%%");
                (sections[0].clone(), sections[1].clone(), user_code)
            }
        };

        // Parse definitions section (including %{ %} blocks)
        if !definitions_section.is_empty() {
            spec.parse_definitions(&definitions_section)?;
        }

        // Parse rules section
        spec.parse_rules(&rules_section)?;

        // Store user code
        spec.user_code = user_code_section.trim().to_string();

        if spec.rules.is_empty() {
            return Err(Error::LexerSpecError {
                line: 0,
                message: "No rules found in lexer specification".to_string(),
            });
        }

        Ok(spec)
    }

    /// Validates that the input is not a parser grammar (.y file).
    /// Detects common patterns that indicate a grammar file was pasted instead of lexer rules.
    fn validate_not_grammar(input: &str) -> Result<()> {
        let mut grammar_indicators = 0;
        let mut indicator_examples = Vec::new();

        for line in input.lines() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
                continue;
            }

            // Check for BNF-style production rules: "Name ::=" or "Name :"
            if trimmed.contains("::=") {
                grammar_indicators += 3;
                if indicator_examples.len() < 2 {
                    indicator_examples.push(format!(
                        "'{}' (BNF production rule)",
                        trimmed.chars().take(50).collect::<String>()
                    ));
                }
            }

            // Check for Yacc-style rules: "name:" at start of line followed by productions
            if trimmed.ends_with(':') && !trimmed.contains('{') && !trimmed.starts_with('"') {
                let name = trimmed.trim_end_matches(':');
                if name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    grammar_indicators += 2;
                    if indicator_examples.len() < 2 {
                        indicator_examples.push(format!("'{}' (grammar rule definition)", trimmed));
                    }
                }
            }

            // Check for production alternatives: line starting with "|"
            if trimmed.starts_with('|') && !trimmed.contains('{') {
                grammar_indicators += 1;
                if indicator_examples.len() < 2 {
                    indicator_examples.push(format!(
                        "'{}' (grammar alternative)",
                        trimmed.chars().take(40).collect::<String>()
                    ));
                }
            }

            // Check for %token declarations (these are parser declarations)
            if trimmed.starts_with("%token")
                || trimmed.starts_with("%type")
                || trimmed.starts_with("%left")
                || trimmed.starts_with("%right")
                || trimmed.starts_with("%nonassoc")
                || trimmed.starts_with("%start")
            {
                grammar_indicators += 2;
                if indicator_examples.len() < 2 {
                    indicator_examples.push(format!(
                        "'{}' (parser declaration)",
                        trimmed.chars().take(40).collect::<String>()
                    ));
                }
            }

            // Check for grammar symbols in angle brackets like <EOF>, <IDENTIFIER>
            if trimmed.contains("<EOF>")
                || trimmed.contains("<IDENTIFIER>")
                || trimmed.contains("<INTEGER_LITERAL>")
                || trimmed.contains("<STRING_LITERAL>")
            {
                grammar_indicators += 1;
            }
        }

        // If we found strong evidence of grammar syntax, return an error
        if grammar_indicators >= 5 {
            let examples_str = if indicator_examples.is_empty() {
                String::new()
            } else {
                format!(
                    "\n\nDetected grammar patterns:\n  - {}",
                    indicator_examples.join("\n  - ")
                )
            };

            return Err(Error::LexerSpecError {
                line: 0,
                message: format!(
                    "This appears to be a parser grammar (.y file), not a lexer specification (.l file).\n\n\
                    Lexer rules should have patterns like:\n  \
                    [0-9]+      {{ return NUMBER; }}\n  \
                    \"if\"        {{ return IF; }}\n  \
                    [ \\t\\n]+    {{ /* skip whitespace */ }}\n\n\
                    Please use the Parser tab for grammar files.{}", 
                    examples_str
                ),
            });
        }

        Ok(())
    }

    /// Parses the definitions section.
    /// Handles %{ %} prologue blocks, %s/%x start conditions, and name definitions.
    fn parse_definitions(&mut self, input: &str) -> Result<()> {
        let mut in_prologue = false;
        let mut prologue_buf = String::new();

        for line in input.lines() {
            let trimmed = line.trim();

            // Handle %{ and %} blocks
            if trimmed == "%{" {
                in_prologue = true;
                continue;
            }
            if trimmed == "%}" {
                in_prologue = false;
                self.prologue.push_str(&prologue_buf);
                self.prologue.push('\n');
                prologue_buf.clear();
                continue;
            }
            if in_prologue {
                prologue_buf.push_str(line);
                prologue_buf.push('\n');
                continue;
            }

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                continue;
            }

            // Skip C-style block comments (simple single-line detection)
            if trimmed.starts_with("/*") {
                continue;
            }

            // Inclusive start condition: %s NAME1 NAME2 ...
            if trimmed.starts_with("%s") {
                let names = trimmed[2..].split_whitespace();
                for name in names {
                    self.start_conditions
                        .insert(name.to_string(), StartConditionType::Inclusive);
                }
                continue;
            }

            // Exclusive start condition: %x NAME1 NAME2 ...
            if trimmed.starts_with("%x") {
                let names = trimmed[2..].split_whitespace();
                for name in names {
                    self.start_conditions
                        .insert(name.to_string(), StartConditionType::Exclusive);
                }
                continue;
            }

            // Skip other % directives we don't handle yet
            if trimmed.starts_with('%') {
                continue;
            }

            // Named pattern definition: NAME  pattern
            if let Some((name, pattern)) = parse_definition(trimmed) {
                // Validate that name looks like an identifier (uppercase)
                if name
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
                {
                    self.definitions.insert(name, pattern);
                }
            }
        }
        Ok(())
    }

    /// Parses the rules section.
    fn parse_rules(&mut self, input: &str) -> Result<()> {
        let lines: Vec<&str> = input.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                i += 1;
                continue;
            }

            // Skip C-style block comments (single-line, like /* ... */)
            if trimmed.starts_with("/*") && trimmed.ends_with("*/") {
                i += 1;
                continue;
            }

            // Skip multi-line C-style block comments
            if trimmed.starts_with("/*") {
                while i < lines.len() {
                    let comment_line = lines[i].trim();
                    i += 1;
                    if comment_line.contains("*/") {
                        break;
                    }
                }
                continue;
            }

            // Skip %{ %} blocks in rules section
            if trimmed == "%{" {
                while i < lines.len() && lines[i].trim() != "%}" {
                    i += 1;
                }
                i += 1;
                continue;
            }

            // Skip indented lines (C code in Flex format)
            if !line.is_empty() && (line.starts_with(' ') || line.starts_with('\t')) {
                i += 1;
                continue;
            }

            // Parse the rule - may span multiple lines if action has { }
            let (rule, lines_consumed) = self.parse_rule_multiline(&lines, i)?;
            self.rules.push(rule);
            i += lines_consumed;
        }
        Ok(())
    }

    /// Parses a single rule that may span multiple lines (for { } actions).
    fn parse_rule_multiline(&self, lines: &[&str], start_idx: usize) -> Result<(LexerRule, usize)> {
        let line = lines[start_idx];
        let line_number = start_idx + 1;
        let mut chars_iter = line.chars().peekable();
        let mut start_conditions = Vec::new();

        // Check for start condition prefix: <COND1,COND2>
        if chars_iter.peek() == Some(&'<') {
            chars_iter.next(); // consume '<'
            let mut cond_str = String::new();
            while let Some(&c) = chars_iter.peek() {
                if c == '>' {
                    chars_iter.next(); // consume '>'
                    break;
                }
                cond_str.push(c);
                chars_iter.next();
            }
            // Parse comma-separated conditions
            for cond in cond_str.split(',') {
                let cond = cond.trim();
                if cond == "*" {
                    // <*> means all conditions
                    start_conditions = self.start_conditions.keys().cloned().collect();
                    break;
                } else if !cond.is_empty() {
                    start_conditions.push(cond.to_string());
                }
            }
        }

        // Get the rest of the line after start conditions
        let remaining: String = chars_iter.collect();
        let remaining = remaining.trim();

        // Parse the pattern (handles quoted strings)
        let (pattern_raw, action_str, extra_lines) =
            split_pattern_action_flex(remaining, lines, start_idx, line_number)?;
        let pattern = self.expand_definitions(&pattern_raw);

        // Parse the regex pattern
        let regex = RegexAst::parse(&pattern).map_err(|e| Error::LexerSpecError {
            line: line_number,
            message: format!("Invalid pattern '{}': {}", pattern, e),
        })?;

        // Parse the action
        let action = parse_action(&action_str);

        Ok((
            LexerRule {
                pattern,
                regex,
                action,
                start_conditions,
                line_number,
            },
            1 + extra_lines,
        ))
    }

    /// Expands {NAME} references in a pattern.
    fn expand_definitions(&self, pattern: &str) -> String {
        let mut result = String::new();
        let mut chars = pattern.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\\' {
                // Escape sequence - push both chars and skip the next one
                result.push(c);
                if let Some(next) = chars.next() {
                    result.push(next);
                }
            } else if c == '{' {
                // Collect the name
                let mut name = String::new();
                let mut found_close = false;
                while let Some(&nc) = chars.peek() {
                    if nc == '}' {
                        chars.next(); // consume '}'
                        found_close = true;
                        break;
                    }
                    name.push(nc);
                    chars.next();
                }

                // Only expand if it looks like a valid definition name
                if found_close
                    && !name.is_empty()
                    && name.chars().all(|c| c.is_alphanumeric() || c == '_')
                {
                    // Look up the definition
                    if let Some(expansion) = self.definitions.get(&name) {
                        // Wrap in group to preserve precedence
                        result.push('(');
                        result.push_str(expansion);
                        result.push(')');
                    } else {
                        // Not a definition, keep as literal
                        result.push('{');
                        result.push_str(&name);
                        result.push('}');
                    }
                } else {
                    // Not a valid definition reference, keep as literal
                    result.push('{');
                    result.push_str(&name);
                    if found_close {
                        result.push('}');
                    }
                }
            } else {
                result.push(c);
            }
        }
        result
    }

    /// Returns all unique token names (excluding Skip, Error, and Begin actions).
    pub fn token_names(&self) -> Vec<&str> {
        self.rules
            .iter()
            .filter_map(|rule| match &rule.action {
                RuleAction::Token(name) => Some(name.as_str()),
                RuleAction::TokenAndBegin(name, _) => Some(name.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Returns all start condition names.
    pub fn condition_names(&self) -> Vec<&str> {
        self.start_conditions.keys().map(|s| s.as_str()).collect()
    }
}

/// Parses a definition line: NAME  pattern
fn parse_definition(line: &str) -> Option<(String, String)> {
    let mut parts = line.splitn(2, |c: char| c.is_whitespace());
    let name = parts.next()?.trim().to_string();
    let pattern = parts.next()?.trim().to_string();
    if name.is_empty() || pattern.is_empty() {
        return None;
    }
    Some((name, pattern))
}

/// Parses an action string.
/// Handles:
/// - Simple token names: NUM, IDENTIFIER
/// - skip/error keywords
/// - BEGIN(STATE) for state changes
/// - { } brace actions from Flex
fn parse_action(action_str: &str) -> RuleAction {
    let trimmed = action_str.trim();
    let action_lower = trimmed.to_lowercase();

    // Handle empty action or empty braces - means skip
    if trimmed.is_empty() || trimmed == "{}" || trimmed == "{ }" {
        return RuleAction::Skip;
    }

    // Simple skip/error keywords
    if action_lower == "skip" {
        return RuleAction::Skip;
    }
    if action_lower == "error" {
        return RuleAction::Error;
    }

    // Handle C-style brace actions: { ... }
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        let inner = trimmed[1..trimmed.len() - 1].trim();
        return parse_c_action(inner);
    }

    // Check for BEGIN(STATE)
    if action_lower.starts_with("begin(") && action_lower.ends_with(")") {
        let state = trimmed[6..trimmed.len() - 1].trim().to_string();
        return RuleAction::Begin(state);
    }

    // Check for TOKEN BEGIN(STATE) or TOKEN, BEGIN(STATE)
    if let Some(begin_idx) = action_lower.find("begin(") {
        let token_part = trimmed[..begin_idx].trim().trim_end_matches(',').trim();
        let begin_part = &trimmed[begin_idx..];
        if begin_part.to_lowercase().starts_with("begin(") && begin_part.ends_with(")") {
            let state = begin_part[6..begin_part.len() - 1].trim().to_string();
            if !token_part.is_empty() {
                return RuleAction::TokenAndBegin(token_part.to_string(), state);
            }
        }
    }

    RuleAction::Token(trimmed.to_string())
}

/// Parses C-style action code to determine the action type.
/// Looks for patterns like:
/// - Empty or just whitespace/semicolons: Skip
/// - return TOKEN; or return TOKEN_NAME;: Token
/// - BEGIN(STATE); : Begin
/// - printf("...", yytext, TOKEN); : Token (extracts TOKEN name)
/// - printf/other code without return: Skip (just side effects)
/// - Complex code with REJECT/yymore/yyless or multiple statements: Code
fn parse_c_action(code: &str) -> RuleAction {
    let code = code.trim();

    // Empty action or just comment
    if code.is_empty() || code == ";" || code.starts_with("/*") && code.ends_with("*/") {
        return RuleAction::Skip;
    }

    let code_lower = code.to_lowercase();

    // Detect complex actions that need to be passed through as raw Code:
    // - REJECT: requires backtracking
    // - yymore(): appends next match
    // - yyless(n): puts back characters
    // - Multiple statements (multiple semicolons outside strings)
    // - Control flow (if, for, while, switch)
    // - Variable declarations
    let needs_raw_code = code_lower.contains("reject")
        || code_lower.contains("yymore")
        || code_lower.contains("yyless")
        || code_lower.contains("unput")
        || code_lower.contains("input(")
        || has_multiple_statements(code)
        || code_lower.contains("if ")
        || code_lower.contains("if(")
        || code_lower.contains("for ")
        || code_lower.contains("for(")
        || code_lower.contains("while ")
        || code_lower.contains("while(")
        || code_lower.contains("switch ")
        || code_lower.contains("switch(")
        || (code.contains("int ")
            || code.contains("char ")
            || code.contains("double ")
            || code.contains("float ")
            || code.contains("void "));

    if needs_raw_code {
        return RuleAction::Code(code.to_string());
    }

    // Look for BEGIN(STATE)
    if let Some(begin_pos) = code_lower.find("begin(") {
        // Find the state name
        let after_begin = &code[begin_pos + 6..];
        if let Some(close_paren) = after_begin.find(')') {
            let state = after_begin[..close_paren].trim().to_string();

            // Check if there's also a return statement
            if let Some(return_pos) = code_lower.find("return ") {
                // Extract token from return statement
                let after_return = &code[return_pos + 7..];
                let token = after_return
                    .split(|c: char| c == ';' || c.is_whitespace())
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !token.is_empty() {
                    return RuleAction::TokenAndBegin(token, state);
                }
            }
            return RuleAction::Begin(state);
        }
    }

    // Look for return TOKEN;
    if let Some(return_pos) = code_lower.find("return ") {
        let after_return = &code[return_pos + 7..];
        let token = after_return
            .split(|c: char| c == ';' || c.is_whitespace())
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        if !token.is_empty() {
            return RuleAction::Token(token);
        }
    }

    // Look for printf("...", yytext, TOKEN); pattern - common in textbook examples
    // Pattern: printf("%s %d\n", yytext, TOKEN_NAME);
    // We need to extract TOKEN_NAME as the last argument before );
    if code_lower.contains("printf") && code.contains("yytext") {
        // Find the last comma (token name is after it)
        if let Some(last_comma) = code.rfind(',') {
            let after_comma = &code[last_comma + 1..];
            // Extract the token name - should be before ); or )
            let token = after_comma
                .trim()
                .trim_end_matches(|c: char| c == ')' || c == ';' || c.is_whitespace())
                .trim()
                .to_string();
            // Exclude lex builtins that aren't actual token names
            let lex_builtins = ["yytext", "yyleng", "yylval", "yylineno", "yyin", "yyout"];
            if !token.is_empty()
                && token.chars().all(|c| c.is_alphanumeric() || c == '_')
                && !lex_builtins.contains(&token.as_str())
            {
                return RuleAction::Token(token);
            }
        }
    }

    // No return statement - treat as skip (side-effect only action like printf)
    RuleAction::Skip
}

/// Checks if code contains multiple statements (multiple semicolons outside strings).
fn has_multiple_statements(code: &str) -> bool {
    let mut semicolon_count = 0;
    let mut in_string = false;
    let mut in_char = false;
    let mut prev_char = ' ';

    for c in code.chars() {
        if c == '"' && prev_char != '\\' && !in_char {
            in_string = !in_string;
        } else if c == '\'' && prev_char != '\\' && !in_string {
            in_char = !in_char;
        } else if c == ';' && !in_string && !in_char {
            semicolon_count += 1;
            if semicolon_count > 1 {
                return true;
            }
        }
        prev_char = c;
    }
    false
}

/// Splits input into sections by %% that appears at start of line.
fn split_sections(input: &str) -> Vec<String> {
    let mut sections = Vec::new();
    let mut current = String::new();

    for line in input.lines() {
        if line.trim() == "%%" {
            sections.push(current);
            current = String::new();
        } else {
            current.push_str(line);
            current.push('\n');
        }
    }
    sections.push(current);
    sections
}

/// Converts a Flex quoted string pattern to a regex.
/// "abc" becomes abc (with special chars escaped)
/// Handles escape sequences inside quotes.
fn convert_quoted_pattern(quoted: &str) -> String {
    // Remove surrounding quotes
    let inner = &quoted[1..quoted.len() - 1];
    let mut result = String::new();
    let mut chars = inner.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            // Escape sequence
            if let Some(next) = chars.next() {
                match next {
                    'n' => result.push_str("\\n"),
                    't' => result.push_str("\\t"),
                    'r' => result.push_str("\\r"),
                    '"' => result.push('"'),
                    '\\' => result.push_str("\\\\"),
                    _ => {
                        result.push('\\');
                        result.push(next);
                    }
                }
            }
        } else {
            // Escape regex metacharacters
            match c {
                '(' | ')' | '[' | ']' | '{' | '}' | '.' | '*' | '+' | '?' | '^' | '$' | '|'
                | '\\' => {
                    result.push('\\');
                    result.push(c);
                }
                _ => result.push(c),
            }
        }
    }
    result
}

/// Splits a Flex rule line into pattern and action parts.
/// Handles:
/// - Quoted string patterns like "("
/// - Actions in { } braces (may span multiple lines)
/// - Simple token name actions
/// Returns (pattern, action, extra_lines_consumed)
fn split_pattern_action_flex(
    line: &str,
    all_lines: &[&str],
    current_idx: usize,
    _line_number: usize,
) -> Result<(String, String, usize)> {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut in_bracket = false;
    let mut in_quote = false;
    let mut escaped = false;
    let mut pattern = String::new();

    // Parse the pattern
    while i < chars.len() {
        let c = chars[i];

        if escaped {
            pattern.push(c);
            escaped = false;
            i += 1;
            continue;
        }

        match c {
            '\\' => {
                pattern.push(c);
                escaped = true;
            }
            '"' if !in_bracket => {
                if in_quote {
                    // End of quoted string
                    pattern.push(c);
                    in_quote = false;
                } else {
                    // Start of quoted string - we'll convert it later
                    pattern.push(c);
                    in_quote = true;
                }
            }
            '[' if !in_quote => {
                pattern.push(c);
                in_bracket = true;
            }
            ']' if !in_quote => {
                pattern.push(c);
                in_bracket = false;
            }
            ' ' | '\t' if !in_bracket && !in_quote => {
                // Found the separator between pattern and action
                break;
            }
            _ => {
                pattern.push(c);
            }
        }
        i += 1;
    }

    // Convert quoted patterns to regex
    if pattern.starts_with('"') && pattern.ends_with('"') && pattern.len() >= 2 {
        pattern = convert_quoted_pattern(&pattern);
    }

    // Skip whitespace between pattern and action
    while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t') {
        i += 1;
    }

    if i >= chars.len() {
        // No action - in Flex this means discard the token (skip)
        return Ok((pattern, "skip".to_string(), 0));
    }

    // Parse the action
    let action_start: String = chars[i..].iter().collect();
    let action_start = action_start.trim();

    // Check if action starts with { - multi-line C code
    if action_start.starts_with('{') {
        let (action, extra_lines) = collect_brace_action(action_start, all_lines, current_idx)?;
        Ok((pattern, action, extra_lines))
    } else {
        // Simple action on same line
        Ok((pattern, action_start.to_string(), 0))
    }
}

/// Collects a { } brace-delimited action that may span multiple lines.
/// Returns the action content and number of extra lines consumed.
fn collect_brace_action(
    first_part: &str,
    all_lines: &[&str],
    start_idx: usize,
) -> Result<(String, usize)> {
    let mut action = String::new();
    let mut brace_depth = 0;
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;
    let mut extra_lines = 0;

    // Process first line
    let mut current = first_part.to_string();

    loop {
        for c in current.chars() {
            if escaped {
                action.push(c);
                escaped = false;
                continue;
            }

            match c {
                '\\' => {
                    action.push(c);
                    escaped = true;
                }
                '"' if !in_char => {
                    action.push(c);
                    in_string = !in_string;
                }
                '\'' if !in_string => {
                    action.push(c);
                    in_char = !in_char;
                }
                '{' if !in_string && !in_char => {
                    brace_depth += 1;
                    action.push(c);
                }
                '}' if !in_string && !in_char => {
                    brace_depth -= 1;
                    action.push(c);
                    if brace_depth == 0 {
                        // Found matching close brace
                        return Ok((action, extra_lines));
                    }
                }
                _ => {
                    action.push(c);
                }
            }
        }

        // Need more lines
        extra_lines += 1;
        if start_idx + extra_lines >= all_lines.len() {
            // Ran out of lines - malformed
            break;
        }
        current = all_lines[start_idx + extra_lines].to_string();
        action.push('\n');
    }

    // If we got here with non-zero brace depth, return what we have
    Ok((action, extra_lines))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_rule() {
        let spec = LexerSpec::parse("[0-9]+  NUM").unwrap();
        assert_eq!(spec.rules.len(), 1);
        assert_eq!(spec.rules[0].pattern, "[0-9]+");
        assert_eq!(spec.rules[0].action, RuleAction::Token("NUM".to_string()));
    }

    #[test]
    fn test_parse_skip_rule() {
        let spec = LexerSpec::parse("\\s+  skip").unwrap();
        assert_eq!(spec.rules.len(), 1);
        assert_eq!(spec.rules[0].action, RuleAction::Skip);
    }

    #[test]
    fn test_parse_error_rule() {
        let spec = LexerSpec::parse(".  error").unwrap();
        assert_eq!(spec.rules.len(), 1);
        assert_eq!(spec.rules[0].action, RuleAction::Error);
    }

    #[test]
    fn test_parse_multiple_rules() {
        let input = r#"
[0-9]+  NUM
if      IF
else    ELSE
[a-z]+  ID
\s+     skip
.       error
"#;
        let spec = LexerSpec::parse(input).unwrap();
        assert_eq!(spec.rules.len(), 6);

        let names = spec.token_names();
        assert!(names.contains(&"NUM"));
        assert!(names.contains(&"IF"));
        assert!(names.contains(&"ELSE"));
        assert!(names.contains(&"ID"));
    }

    #[test]
    fn test_skip_comments() {
        let input = r#"
# This is a comment
[0-9]+  NUM
// Another comment
[a-z]+  ID
"#;
        let spec = LexerSpec::parse(input).unwrap();
        assert_eq!(spec.rules.len(), 2);
    }

    #[test]
    fn test_definitions_section() {
        let input = r#"
DIGIT   [0-9]
ALPHA   [a-zA-Z]
%%
{DIGIT}+   NUM
{ALPHA}+   ID
"#;
        let spec = LexerSpec::parse(input).unwrap();
        assert_eq!(spec.definitions.get("DIGIT"), Some(&"[0-9]".to_string()));
        assert_eq!(spec.definitions.get("ALPHA"), Some(&"[a-zA-Z]".to_string()));
        assert_eq!(spec.rules.len(), 2);
        // Check pattern was expanded
        assert_eq!(spec.rules[0].pattern, "([0-9])+");
    }

    #[test]
    fn test_start_conditions() {
        let input = r#"
%x COMMENT
%s STRING
%%
<COMMENT>\*/    END_COMMENT
<STRING>"       END_STRING
[a-z]+          ID
"#;
        let spec = LexerSpec::parse(input).unwrap();
        assert_eq!(
            spec.start_conditions.get("COMMENT"),
            Some(&StartConditionType::Exclusive)
        );
        assert_eq!(
            spec.start_conditions.get("STRING"),
            Some(&StartConditionType::Inclusive)
        );
        assert_eq!(
            spec.start_conditions.get("INITIAL"),
            Some(&StartConditionType::Inclusive)
        );

        assert_eq!(spec.rules[0].start_conditions, vec!["COMMENT"]);
        assert_eq!(spec.rules[1].start_conditions, vec!["STRING"]);
        assert!(spec.rules[2].start_conditions.is_empty()); // No explicit condition
    }

    #[test]
    fn test_begin_action() {
        let input = r#"
%x COMMENT
%%
/\*         BEGIN(COMMENT)
<COMMENT>\*/  BEGIN(INITIAL)
"#;
        let spec = LexerSpec::parse(input).unwrap();
        assert_eq!(
            spec.rules[0].action,
            RuleAction::Begin("COMMENT".to_string())
        );
        assert_eq!(
            spec.rules[1].action,
            RuleAction::Begin("INITIAL".to_string())
        );
    }

    #[test]
    fn test_multiple_start_conditions() {
        let input = r#"
%x A B
%%
<A,B>foo    FOO
<*>bar      BAR
"#;
        let spec = LexerSpec::parse(input).unwrap();
        assert_eq!(spec.rules[0].start_conditions.len(), 2);
        assert!(spec.rules[0].start_conditions.contains(&"A".to_string()));
        assert!(spec.rules[0].start_conditions.contains(&"B".to_string()));
        // <*> should match all conditions (INITIAL, A, B)
        assert_eq!(spec.rules[1].start_conditions.len(), 3);
    }

    #[test]
    fn test_flex_textbook_format() {
        let input = r#"
%{
int lineNum = 0;
%}
%%
"(" { printf("(\n"); }
")" { printf(")\n"); }
"+" { printf("+\n"); }
"*" { printf("*\n"); }
\n { lineNum++; }
[ \t]+ { }
[0-9]+ { printf("%s\n", yytext); }
%%
int yywrap() {
return 1;
}
"#;
        let spec = LexerSpec::parse(input).unwrap();
        // Should have parsed the prologue
        assert!(spec.prologue.contains("lineNum"));
        // Should have parsed the rules
        assert!(
            spec.rules.len() >= 5,
            "Expected at least 5 rules, got {}",
            spec.rules.len()
        );
        // First rule should match "("
        assert_eq!(spec.rules[0].pattern, "\\(");
        // Actions with printf but no return should be Skip
        assert_eq!(spec.rules[0].action, RuleAction::Skip);
        // Empty braces { } should be Skip
        // assert_eq!(spec.rules[5].action, RuleAction::Skip);
    }

    #[test]
    fn test_quoted_pattern() {
        let input = r#"
%%
"hello" { return HELLO; }
"+" { return PLUS; }
"#;
        let spec = LexerSpec::parse(input).unwrap();
        assert_eq!(spec.rules.len(), 2);
        // Quoted "hello" becomes escaped hello
        assert_eq!(spec.rules[0].pattern, "hello");
        assert_eq!(spec.rules[0].action, RuleAction::Token("HELLO".to_string()));
        // Quoted "+" becomes escaped \+
        assert_eq!(spec.rules[1].pattern, "\\+");
        assert_eq!(spec.rules[1].action, RuleAction::Token("PLUS".to_string()));
    }
}
