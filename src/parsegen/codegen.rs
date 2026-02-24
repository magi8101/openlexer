//! Code generation for parsers with semantic actions.

use crate::error::Result;
use crate::lexgen::codegen::TargetLanguage;
use crate::parsegen::grammar::{Grammar, Rule, Symbol};
use crate::parsegen::lalr::{Action, ParsingTable};
use std::collections::HashMap;

/// Returns true if the grammar uses advanced features requiring the full codegen path.
fn grammar_needs_advanced(grammar: &Grammar) -> bool {
    grammar.has_union()
        || grammar.locations
        || !grammar.destructors.is_empty()
        || grammar.lac_enabled
        || grammar.glr_mode
        || grammar.prologue.is_some()
        || grammar.error_verbose
}

/// Helper: build symbol-to-integer mappings for compact table encoding.
/// Returns (terminal_ids, nonterminal_ids, num_states).
fn build_symbol_ids(
    table: &ParsingTable,
    grammar: &Grammar,
) -> (HashMap<String, usize>, HashMap<String, usize>, usize) {
    // Terminals: all declared tokens + "$" for EOF
    let mut term_ids: HashMap<String, usize> = HashMap::new();
    term_ids.insert("$".to_string(), 0);
    for (i, tok) in grammar.tokens.iter().enumerate() {
        term_ids.insert(tok.clone(), i + 1);
    }

    // Nonterminals: collect all unique LHS symbols from rules
    let mut nt_ids: HashMap<String, usize> = HashMap::new();
    let mut nt_idx = 0;
    for rule in &grammar.rules {
        if !nt_ids.contains_key(&rule.lhs) {
            nt_ids.insert(rule.lhs.clone(), nt_idx);
            nt_idx += 1;
        }
    }

    // Count states
    let num_states = {
        let mut max_s = 0usize;
        for &s in table.action.keys() {
            if s > max_s { max_s = s; }
        }
        for &s in table.goto.keys() {
            if s > max_s { max_s = s; }
        }
        max_s + 1
    };

    (term_ids, nt_ids, num_states)
}

/// Translate $$ and $N in semantic actions for minimal C output.
/// Simple integer-only substitution (no union support).
fn substitute_vars_c_minimal(action: &str, rhs_len: usize) -> String {
    let mut out = String::new();
    let chars: Vec<char> = action.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '$' && i + 1 < chars.len() {
            if chars[i + 1] == '$' {
                out.push_str("yyval");
                i += 2;
            } else if chars[i + 1].is_ascii_digit() {
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
                let num_str: String = chars[i + 1..j].iter().collect();
                if let Ok(n) = num_str.parse::<usize>() {
                    if n > 0 && n <= rhs_len {
                        out.push_str(&format!("vs[sp-{}]", rhs_len - n));
                    } else {
                        out.push('0');
                    }
                }
                i = j;
            } else {
                out.push('$');
                i += 1;
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// Translate $$ and $N in semantic actions for minimal Java output.
fn substitute_vars_java_minimal(action: &str, rhs_len: usize) -> String {
    let mut out = String::new();
    let chars: Vec<char> = action.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '$' && i + 1 < chars.len() {
            if chars[i + 1] == '$' {
                out.push_str("yyval");
                i += 2;
            } else if chars[i + 1].is_ascii_digit() {
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
                let num_str: String = chars[i + 1..j].iter().collect();
                if let Ok(n) = num_str.parse::<usize>() {
                    if n > 0 && n <= rhs_len {
                        out.push_str(&format!("vs[sp-{}]", rhs_len - n));
                    } else {
                        out.push_str("0");
                    }
                }
                i = j;
            } else {
                out.push('$');
                i += 1;
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out = out.replace("printf(", "System.out.printf(");
    out
}

/// Translate $$ and $N in semantic actions for minimal Python output.
fn substitute_vars_python_minimal(action: &str, rhs_len: usize) -> String {
    let mut out = String::new();
    let chars: Vec<char> = action.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '$' && i + 1 < chars.len() {
            if chars[i + 1] == '$' {
                out.push_str("yyval");
                i += 2;
            } else if chars[i + 1].is_ascii_digit() {
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
                let num_str: String = chars[i + 1..j].iter().collect();
                if let Ok(n) = num_str.parse::<usize>() {
                    if n > 0 && n <= rhs_len {
                        out.push_str(&format!("vs[sp-{}]", rhs_len - n));
                    } else {
                        out.push_str("0");
                    }
                }
                i = j;
            } else {
                out.push('$');
                i += 1;
            }
        } else if chars[i] == ';' {
            i += 1; // skip C semicolons
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out = out.replace("printf(", "print(");
    out = out.replace("%d", "{}");
    out = out.replace("\\n", "");
    out
}

// =============================================================================
// Minimal parser generators (~60 lines output for simple grammars)
// =============================================================================

fn generate_c_minimal(table: &ParsingTable, grammar: &Grammar) -> Result<String> {
    let (term_ids, nt_ids, num_states) = build_symbol_ids(table, grammar);
    let num_terms = term_ids.len();
    let num_nts = nt_ids.len();
    let num_rules = grammar.rules.len();

    let mut c = String::new();
    c.push_str("/* Parser generated by OpenLexer */\n");
    c.push_str("#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n#include <ctype.h>\n\n");

    // Token IDs as defines
    c.push_str("#define T_EOF 0\n");
    for (tok, &id) in &term_ids {
        if tok != "$" {
            c.push_str(&format!("#define T_{} {}\n", tok.to_uppercase(), id));
        }
    }
    c.push_str("\n");

    // Action table: positive = shift to state, negative = reduce by rule, 0 = error, -9999 = accept
    c.push_str(&format!("static int ACT[{}][{}] = {{\n", num_states, num_terms));
    for s in 0..num_states {
        c.push_str("  {");
        for t in 0..num_terms {
            let val = if let Some(actions) = table.action.get(&s) {
                // Find the terminal name for this id
                let term_name = term_ids.iter().find(|(_, &v)| v == t).map(|(k, _)| k.as_str()).unwrap_or("");
                match actions.get(term_name) {
                    Some(Action::Shift(n)) => *n as i32 + 1, // +1 so 0 means error
                    Some(Action::Reduce(n)) => -(*n as i32) - 1, // -1 so -1 means rule 0
                    Some(Action::Accept) => -9999,
                    None => 0,
                }
            } else {
                0
            };
            if t > 0 { c.push(','); }
            c.push_str(&format!("{}", val));
        }
        c.push_str("},\n");
    }
    c.push_str("};\n\n");

    // Goto table
    c.push_str(&format!("static int GOTO[{}][{}] = {{\n", num_states, num_nts));
    for s in 0..num_states {
        c.push_str("  {");
        for nt in 0..num_nts {
            let val = if let Some(gotos) = table.goto.get(&s) {
                let nt_name = nt_ids.iter().find(|(_, &v)| v == nt).map(|(k, _)| k.as_str()).unwrap_or("");
                match gotos.get(nt_name) {
                    Some(&n) => n as i32,
                    None => -1,
                }
            } else {
                -1
            };
            if nt > 0 { c.push(','); }
            c.push_str(&format!("{}", val));
        }
        c.push_str("},\n");
    }
    c.push_str("};\n\n");

    // Rule info: LHS nonterminal ID and RHS length
    c.push_str(&format!("static int RLHS[{}] = {{", num_rules));
    for (i, rule) in grammar.rules.iter().enumerate() {
        if i > 0 { c.push(','); }
        c.push_str(&format!("{}", nt_ids.get(&rule.lhs).unwrap_or(&0)));
    }
    c.push_str("};\n");
    c.push_str(&format!("static int RLEN[{}] = {{", num_rules));
    for (i, rule) in grammar.rules.iter().enumerate() {
        if i > 0 { c.push(','); }
        c.push_str(&format!("{}", rule.rhs.len()));
    }
    c.push_str("};\n\n");

    // Stack and parser
    c.push_str("static int ss[512], vs[512], sp;\n\n");

    // Inline tokenizer
    c.push_str("static const char *input_ptr;\nstatic int yylval;\n\n");
    c.push_str("int yylex(void) {\n");
    c.push_str("    while (*input_ptr == ' ' || *input_ptr == '\\t' || *input_ptr == '\\n') input_ptr++;\n");
    c.push_str("    if (*input_ptr == 0) return T_EOF;\n");
    c.push_str("    if (isdigit(*input_ptr)) {\n");
    c.push_str("        yylval = 0;\n");
    c.push_str("        while (isdigit(*input_ptr)) { yylval = yylval * 10 + (*input_ptr - '0'); input_ptr++; }\n");
    // Find the NUMBER token
    let number_tid = term_ids.get("NUMBER").or_else(|| term_ids.get("number")).or_else(|| term_ids.get("NUM"));
    if let Some(&tid) = number_tid {
        c.push_str(&format!("        return {};\n", tid));
    } else {
        // If no NUMBER token, just return the first non-operator token or use a fallback
        c.push_str("        return -1; /* No NUMBER token defined */\n");
    }
    c.push_str("    }\n");

    // Single-character tokens
    c.push_str("    yylval = 0;\n");
    c.push_str("    char ch = *input_ptr++;\n");
    // Map single-char token names to their IDs
    for (tok, &id) in &term_ids {
        if tok.len() == 1 && tok != "$" {
            let ch = tok.chars().next().unwrap();
            c.push_str(&format!("    if (ch == '{}') return {};\n",
                if ch == '\'' { "\\'" } else { &tok },
                id
            ));
        } else if tok.len() > 1 && tok != "$" {
            // Multi-char token names like PLUS, MINUS etc.
            // Check if the grammar maps operator names to single chars
            match tok.to_uppercase().as_str() {
                "PLUS" => c.push_str(&format!("    if (ch == '+') return {};\n", id)),
                "MINUS" | "SUB" => c.push_str(&format!("    if (ch == '-') return {};\n", id)),
                "TIMES" | "MUL" | "STAR" | "MULTIPLY" => c.push_str(&format!("    if (ch == '*') return {};\n", id)),
                "DIVIDE" | "DIV" | "SLASH" => c.push_str(&format!("    if (ch == '/') return {};\n", id)),
                "LPAREN" | "LPAR" => c.push_str(&format!("    if (ch == '(') return {};\n", id)),
                "RPAREN" | "RPAR" => c.push_str(&format!("    if (ch == ')') return {};\n", id)),
                "EQUALS" | "EQ" | "ASSIGN" => c.push_str(&format!("    if (ch == '=') return {};\n", id)),
                "SEMICOLON" | "SEMI" => c.push_str(&format!("    if (ch == ';') return {};\n", id)),
                "COMMA" => c.push_str(&format!("    if (ch == ',') return {};\n", id)),
                "MOD" | "MODULO" | "PERCENT" => c.push_str(&format!("    if (ch == '%%') return {};\n", id)),
                "POW" | "POWER" | "CARET" => c.push_str(&format!("    if (ch == '^') return {};\n", id)),
                _ => {}
            }
        }
    }
    c.push_str("    return T_EOF;\n");
    c.push_str("}\n\n");

    // Parse function
    c.push_str("int yyparse(const char *src) {\n");
    c.push_str("    input_ptr = src; sp = 0; ss[0] = 0; vs[0] = 0;\n");
    c.push_str("    int tok = yylex(), yyval;\n");
    c.push_str("    while (1) {\n");
    c.push_str("        int a = ACT[ss[sp]][tok];\n");
    c.push_str("        if (a > 0) { sp++; ss[sp] = a-1; vs[sp] = yylval; tok = yylex(); }\n");
    c.push_str("        else if (a == -9999) { return vs[sp]; }\n");
    c.push_str("        else if (a < 0) {\n");
    c.push_str("            int r = -(a+1), len = RLEN[r];\n");
    c.push_str("            yyval = (len > 0) ? vs[sp-len+1] : 0;\n");

    // Semantic actions
    c.push_str("            switch(r) {\n");
    for (i, rule) in grammar.rules.iter().enumerate() {
        if let Some(action) = &rule.action {
            let sub = substitute_vars_c_minimal(action, rule.rhs.len());
            let trimmed = sub.trim();
            if !trimmed.is_empty() {
                c.push_str(&format!("                case {}: {} break;\n", i, trimmed));
            }
        }
    }
    c.push_str("            }\n");

    c.push_str("            sp -= len;\n");
    c.push_str("            int g = GOTO[ss[sp]][RLHS[r]];\n");
    c.push_str("            sp++; ss[sp] = g; vs[sp] = yyval;\n");
    c.push_str("        } else { printf(\"Syntax error at token %d\\n\", tok); return -1; }\n");
    c.push_str("    }\n}\n\n");

    // Test driver
    c.push_str("#ifndef PARSER_NO_MAIN\n");
    c.push_str("int main(int argc, char **argv) {\n");
    c.push_str("    char buffer[1024];\n");
    c.push_str("    const char *expr;\n");
    c.push_str("    if (argc > 1) {\n");
    c.push_str("        expr = argv[1];\n");
    c.push_str("    } else {\n");
    c.push_str("        if (fgets(buffer, sizeof(buffer), stdin)) {\n");
    c.push_str("            size_t len = strlen(buffer);\n");
    c.push_str("            if (len > 0 && buffer[len-1] == '\\n') buffer[len-1] = '\\0';\n");
    c.push_str("            expr = buffer;\n");
    c.push_str("        } else {\n");
    c.push_str("            expr = \"3 + 4 * 2\";\n");
    c.push_str("        }\n");
    c.push_str("    }\n");
    c.push_str("    printf(\"Input: \\\"%s\\\"\\n\", expr);\n");
    c.push_str("    int result = yyparse(expr);\n");
    c.push_str("    printf(\"Result: %d\\n\", result);\n");
    c.push_str("    return 0;\n}\n");
    c.push_str("#endif\n");

    Ok(c)
}

fn generate_java_minimal(table: &ParsingTable, grammar: &Grammar) -> Result<String> {
    let (term_ids, nt_ids, num_states) = build_symbol_ids(table, grammar);
    let num_terms = term_ids.len();
    let num_nts = nt_ids.len();
    let _num_rules = grammar.rules.len();

    let mut c = String::new();
    c.push_str("/** Parser generated by OpenLexer */\n");
    c.push_str("public class Parser {\n");

    // Token IDs
    c.push_str("    static final int T_EOF = 0");
    for (tok, &id) in &term_ids {
        if tok != "$" {
            c.push_str(&format!(", T_{} = {}", tok.to_uppercase(), id));
        }
    }
    c.push_str(";\n\n");

    // Action table
    c.push_str(&format!("    static int[][] ACT = {{\n"));
    for s in 0..num_states {
        c.push_str("        {");
        for t in 0..num_terms {
            let val = if let Some(actions) = table.action.get(&s) {
                let term_name = term_ids.iter().find(|(_, &v)| v == t).map(|(k, _)| k.as_str()).unwrap_or("");
                match actions.get(term_name) {
                    Some(Action::Shift(n)) => *n as i32 + 1,
                    Some(Action::Reduce(n)) => -(*n as i32) - 1,
                    Some(Action::Accept) => -9999,
                    None => 0,
                }
            } else { 0 };
            if t > 0 { c.push(','); }
            c.push_str(&format!("{}", val));
        }
        c.push_str("},\n");
    }
    c.push_str("    };\n\n");

    // Goto table
    c.push_str(&format!("    static int[][] GT = {{\n"));
    for s in 0..num_states {
        c.push_str("        {");
        for nt in 0..num_nts {
            let val = if let Some(gotos) = table.goto.get(&s) {
                let nt_name = nt_ids.iter().find(|(_, &v)| v == nt).map(|(k, _)| k.as_str()).unwrap_or("");
                match gotos.get(nt_name) { Some(&n) => n as i32, None => -1 }
            } else { -1 };
            if nt > 0 { c.push(','); }
            c.push_str(&format!("{}", val));
        }
        c.push_str("},\n");
    }
    c.push_str("    };\n\n");

    // Rule LHS and RHS length
    c.push_str("    static int[] RLHS = {");
    for (i, rule) in grammar.rules.iter().enumerate() {
        if i > 0 { c.push(','); }
        c.push_str(&format!("{}", nt_ids.get(&rule.lhs).unwrap_or(&0)));
    }
    c.push_str("};\n");
    c.push_str("    static int[] RLEN = {");
    for (i, rule) in grammar.rules.iter().enumerate() {
        if i > 0 { c.push(','); }
        c.push_str(&format!("{}", rule.rhs.len()));
    }
    c.push_str("};\n\n");

    // Lexer state
    c.push_str("    static String src; static int pos; static int yylval;\n\n");

    // Inline tokenizer
    let number_tid = term_ids.get("NUMBER").or_else(|| term_ids.get("number")).or_else(|| term_ids.get("NUM"));
    c.push_str("    static int yylex() {\n");
    c.push_str("        while (pos < src.length() && Character.isWhitespace(src.charAt(pos))) pos++;\n");
    c.push_str("        if (pos >= src.length()) return T_EOF;\n");
    c.push_str("        if (Character.isDigit(src.charAt(pos))) {\n");
    c.push_str("            yylval = 0;\n");
    c.push_str("            while (pos < src.length() && Character.isDigit(src.charAt(pos)))\n");
    c.push_str("                { yylval = yylval * 10 + (src.charAt(pos) - '0'); pos++; }\n");
    if let Some(&tid) = number_tid {
        c.push_str(&format!("            return {};\n", tid));
    } else {
        c.push_str("            return -1;\n");
    }
    c.push_str("        }\n");
    c.push_str("        yylval = 0; char ch = src.charAt(pos++);\n");
    for (tok, &id) in &term_ids {
        if tok.len() == 1 && tok != "$" {
            c.push_str(&format!("        if (ch == '{}') return {};\n", tok, id));
        } else if tok.len() > 1 && tok != "$" {
            match tok.to_uppercase().as_str() {
                "PLUS" => c.push_str(&format!("        if (ch == '+') return {};\n", id)),
                "MINUS" | "SUB" => c.push_str(&format!("        if (ch == '-') return {};\n", id)),
                "TIMES" | "MUL" | "STAR" | "MULTIPLY" => c.push_str(&format!("        if (ch == '*') return {};\n", id)),
                "DIVIDE" | "DIV" | "SLASH" => c.push_str(&format!("        if (ch == '/') return {};\n", id)),
                "LPAREN" | "LPAR" => c.push_str(&format!("        if (ch == '(') return {};\n", id)),
                "RPAREN" | "RPAR" => c.push_str(&format!("        if (ch == ')') return {};\n", id)),
                "EQUALS" | "EQ" | "ASSIGN" => c.push_str(&format!("        if (ch == '=') return {};\n", id)),
                "SEMICOLON" | "SEMI" => c.push_str(&format!("        if (ch == ';') return {};\n", id)),
                "COMMA" => c.push_str(&format!("        if (ch == ',') return {};\n", id)),
                "MOD" | "MODULO" | "PERCENT" => c.push_str(&format!("        if (ch == '%%') return {};\n", id)),
                "POW" | "POWER" | "CARET" => c.push_str(&format!("        if (ch == '^') return {};\n", id)),
                _ => {}
            }
        }
    }
    c.push_str("        return T_EOF;\n    }\n\n");

    // Parse function
    c.push_str("    static int parse(String input) {\n");
    c.push_str("        src = input; pos = 0;\n");
    c.push_str("        int[] ss = new int[512], vs = new int[512]; int sp = 0;\n");
    c.push_str("        ss[0] = 0; vs[0] = 0;\n");
    c.push_str("        int tok = yylex(), yyval;\n");
    c.push_str("        while (true) {\n");
    c.push_str("            int a = ACT[ss[sp]][tok];\n");
    c.push_str("            if (a > 0) { sp++; ss[sp] = a-1; vs[sp] = yylval; tok = yylex(); }\n");
    c.push_str("            else if (a == -9999) { return vs[sp]; }\n");
    c.push_str("            else if (a < 0) {\n");
    c.push_str("                int r = -(a+1), len = RLEN[r];\n");
    c.push_str("                yyval = (len > 0) ? vs[sp-len+1] : 0;\n");

    // Semantic actions
    c.push_str("                switch(r) {\n");
    for (i, rule) in grammar.rules.iter().enumerate() {
        if let Some(action) = &rule.action {
            let sub = substitute_vars_java_minimal(action, rule.rhs.len());
            let trimmed = sub.trim();
            if !trimmed.is_empty() {
                c.push_str(&format!("                    case {}: {} break;\n", i, trimmed));
            }
        }
    }
    c.push_str("                }\n");

    c.push_str("                sp -= len;\n");
    c.push_str("                int g = GT[ss[sp]][RLHS[r]];\n");
    c.push_str("                sp++; ss[sp] = g; vs[sp] = yyval;\n");
    c.push_str("            } else {\n");
    c.push_str("                System.out.println(\"Syntax error at token \" + tok);\n");
    c.push_str("                return -1;\n");
    c.push_str("            }\n");
    c.push_str("        }\n    }\n\n");

    // Main
    c.push_str("    public static void main(String[] args) {\n");
    c.push_str("        String expr = \"3 + 4 * 2\";\n");
    c.push_str("        if (args.length > 0) {\n");
    c.push_str("            expr = args[0];\n");
    c.push_str("        } else {\n");
    c.push_str("            try (java.util.Scanner sc = new java.util.Scanner(System.in)) {\n");
    c.push_str("                if (sc.hasNextLine()) expr = sc.nextLine();\n");
    c.push_str("            } catch (Exception e) {}\n");
    c.push_str("        }\n");
    c.push_str("        System.out.println(\"Input: \\\"\" + expr + \"\\\"\");\n");
    c.push_str("        System.out.println(\"Result: \" + parse(expr));\n");
    c.push_str("    }\n}\n");

    Ok(c)
}

fn generate_python_minimal(table: &ParsingTable, grammar: &Grammar) -> Result<String> {
    let (term_ids, nt_ids, num_states) = build_symbol_ids(table, grammar);
    let num_terms = term_ids.len();
    let num_nts = nt_ids.len();
    let _num_rules = grammar.rules.len();

    let mut c = String::new();
    c.push_str("# Parser generated by OpenLexer\n\n");

    // Token IDs
    c.push_str("T_EOF = 0\n");
    for (tok, &id) in &term_ids {
        if tok != "$" {
            c.push_str(&format!("T_{} = {}\n", tok.to_uppercase(), id));
        }
    }
    c.push_str("\n");

    // Action table
    c.push_str("ACT = [\n");
    for s in 0..num_states {
        c.push_str("    [");
        for t in 0..num_terms {
            let val = if let Some(actions) = table.action.get(&s) {
                let term_name = term_ids.iter().find(|(_, &v)| v == t).map(|(k, _)| k.as_str()).unwrap_or("");
                match actions.get(term_name) {
                    Some(Action::Shift(n)) => *n as i32 + 1,
                    Some(Action::Reduce(n)) => -(*n as i32) - 1,
                    Some(Action::Accept) => -9999,
                    None => 0,
                }
            } else { 0 };
            if t > 0 { c.push(','); }
            c.push_str(&format!("{}", val));
        }
        c.push_str("],\n");
    }
    c.push_str("]\n\n");

    // Goto table
    c.push_str("GT = [\n");
    for s in 0..num_states {
        c.push_str("    [");
        for nt in 0..num_nts {
            let val = if let Some(gotos) = table.goto.get(&s) {
                let nt_name = nt_ids.iter().find(|(_, &v)| v == nt).map(|(k, _)| k.as_str()).unwrap_or("");
                match gotos.get(nt_name) { Some(&n) => n as i32, None => -1 }
            } else { -1 };
            if nt > 0 { c.push(','); }
            c.push_str(&format!("{}", val));
        }
        c.push_str("],\n");
    }
    c.push_str("]\n\n");

    // Rule LHS and length
    c.push_str("RLHS = [");
    for (i, rule) in grammar.rules.iter().enumerate() {
        if i > 0 { c.push(','); }
        c.push_str(&format!("{}", nt_ids.get(&rule.lhs).unwrap_or(&0)));
    }
    c.push_str("]\n");
    c.push_str("RLEN = [");
    for (i, rule) in grammar.rules.iter().enumerate() {
        if i > 0 { c.push(','); }
        c.push_str(&format!("{}", rule.rhs.len()));
    }
    c.push_str("]\n\n");

    // Tokenizer
    let number_tid = term_ids.get("NUMBER").or_else(|| term_ids.get("number")).or_else(|| term_ids.get("NUM"));
    c.push_str("class Lexer:\n");
    c.push_str("    def __init__(self, src): self.src, self.pos, self.yylval = src, 0, 0\n");
    c.push_str("    def lex(self):\n");
    c.push_str("        while self.pos < len(self.src) and self.src[self.pos] in ' \\t\\n': self.pos += 1\n");
    c.push_str("        if self.pos >= len(self.src): return T_EOF\n");
    c.push_str("        if self.src[self.pos].isdigit():\n");
    c.push_str("            self.yylval = 0\n");
    c.push_str("            while self.pos < len(self.src) and self.src[self.pos].isdigit():\n");
    c.push_str("                self.yylval = self.yylval * 10 + int(self.src[self.pos]); self.pos += 1\n");
    if let Some(&tid) = number_tid {
        c.push_str(&format!("            return {}\n", tid));
    } else {
        c.push_str("            return -1\n");
    }
    c.push_str("        self.yylval = 0; ch = self.src[self.pos]; self.pos += 1\n");
    for (tok, &id) in &term_ids {
        if tok.len() == 1 && tok != "$" {
            c.push_str(&format!("        if ch == '{}': return {}\n", tok, id));
        } else if tok.len() > 1 && tok != "$" {
            match tok.to_uppercase().as_str() {
                "PLUS" => c.push_str(&format!("        if ch == '+': return {}\n", id)),
                "MINUS" | "SUB" => c.push_str(&format!("        if ch == '-': return {}\n", id)),
                "TIMES" | "MUL" | "STAR" | "MULTIPLY" => c.push_str(&format!("        if ch == '*': return {}\n", id)),
                "DIVIDE" | "DIV" | "SLASH" => c.push_str(&format!("        if ch == '/': return {}\n", id)),
                "LPAREN" | "LPAR" => c.push_str(&format!("        if ch == '(': return {}\n", id)),
                "RPAREN" | "RPAR" => c.push_str(&format!("        if ch == ')': return {}\n", id)),
                "EQUALS" | "EQ" | "ASSIGN" => c.push_str(&format!("        if ch == '=': return {}\n", id)),
                "SEMICOLON" | "SEMI" => c.push_str(&format!("        if ch == ';': return {}\n", id)),
                "COMMA" => c.push_str(&format!("        if ch == ',': return {}\n", id)),
                "MOD" | "MODULO" | "PERCENT" => c.push_str(&format!("        if ch == '%%': return {}\n", id)),
                "POW" | "POWER" | "CARET" => c.push_str(&format!("        if ch == '^': return {}\n", id)),
                _ => {}
            }
        }
    }
    c.push_str("        return T_EOF\n\n");

    // Parse function
    c.push_str("def parse(src):\n");
    c.push_str("    lex = Lexer(src); ss, vs, sp = [0]*512, [0]*512, 0\n");
    c.push_str("    tok = lex.lex()\n");
    c.push_str("    while True:\n");
    c.push_str("        a = ACT[ss[sp]][tok]\n");
    c.push_str("        if a > 0: sp += 1; ss[sp] = a-1; vs[sp] = lex.yylval; tok = lex.lex()\n");
    c.push_str("        elif a == -9999: return vs[sp]\n");
    c.push_str("        elif a < 0:\n");
    c.push_str("            r = -(a+1); ln = RLEN[r]\n");
    c.push_str("            yyval = vs[sp-ln+1] if ln > 0 else 0\n");

    // Semantic actions
    let has_actions = grammar.rules.iter().any(|r| r.action.is_some());
    if has_actions {
        let mut first = true;
        for (i, rule) in grammar.rules.iter().enumerate() {
            if let Some(action) = &rule.action {
                let sub = substitute_vars_python_minimal(action, rule.rhs.len());
                let trimmed = sub.trim();
                if !trimmed.is_empty() {
                    if first {
                        c.push_str(&format!("            if r == {}: {}\n", i, trimmed));
                        first = false;
                    } else {
                        c.push_str(&format!("            elif r == {}: {}\n", i, trimmed));
                    }
                }
            }
        }
    }

    c.push_str("            sp -= ln; g = GT[ss[sp]][RLHS[r]]\n");
    c.push_str("            sp += 1; ss[sp] = g; vs[sp] = yyval\n");
    c.push_str("        else: print(f'Syntax error at token {tok}'); return -1\n\n");

    // Test
    c.push_str("if __name__ == '__main__':\n");
    c.push_str("    import sys\n");
    c.push_str("    if len(sys.argv) > 1:\n");
    c.push_str("        expr = sys.argv[1]\n");
    c.push_str("    else:\n");
    c.push_str("        stdin_input = sys.stdin.read().strip()\n");
    c.push_str("        expr = stdin_input if stdin_input else '3 + 4 * 2'\n");
    c.push_str("    print(f'Input: \"{expr}\"')\n");
    c.push_str("    print(f'Result: {parse(expr)}')\n");

    Ok(c)
}

pub fn generate_parser(
    table: &ParsingTable,
    grammar: &Grammar,
    lang: TargetLanguage,
) -> Result<String> {
    if !grammar_needs_advanced(grammar) {
        return match lang {
            TargetLanguage::C => generate_c_minimal(table, grammar),
            TargetLanguage::Java => generate_java_minimal(table, grammar),
            TargetLanguage::Python => generate_python_minimal(table, grammar),
        };
    }
    match lang {
        TargetLanguage::C => generate_c(table, grammar),
        TargetLanguage::Java => generate_java(table, grammar),
        TargetLanguage::Python => generate_python(table, grammar),
    }
}

fn generate_c(table: &ParsingTable, grammar: &Grammar) -> Result<String> {
    let mut code = String::new();
    code.push_str("/* Generated by OpenLexer */\n");
    code.push_str("#include <stdio.h>\n");
    code.push_str("#include <stdlib.h>\n");
    code.push_str("#include <string.h>\n\n");

    // Emit user prologue from %{ ... %}
    if let Some(prologue) = &grammar.prologue {
        code.push_str("/* User prologue from %{ ... %} */\n");
        code.push_str(prologue);
        code.push_str("\n\n");
    }

    // Generate YYLTYPE if locations enabled
    if grammar.locations {
        code.push_str("/* Location tracking */\n");
        code.push_str("typedef struct YYLTYPE {\n");
        code.push_str("    int first_line;\n");
        code.push_str("    int first_column;\n");
        code.push_str("    int last_line;\n");
        code.push_str("    int last_column;\n");
        code.push_str("} YYLTYPE;\n\n");
        code.push_str("YYLTYPE yylloc;\n\n");
    }

    // Generate YYSTYPE: use raw_union_body if available (Bison-compatible verbatim copy)
    if let Some(raw_body) = &grammar.raw_union_body {
        code.push_str("/* Semantic value type from %union */\n");
        code.push_str("typedef union YYSTYPE {\n");
        code.push_str(raw_body);
        code.push_str("} YYSTYPE;\n\n");
    } else if grammar.has_union() {
        code.push_str("/* Semantic value type from %union */\n");
        code.push_str("typedef union YYSTYPE {\n");
        for field in &grammar.union_fields {
            code.push_str(&format!("    {} {};\n", field.c_type, field.name));
        }
        code.push_str("} YYSTYPE;\n\n");
    } else {
        code.push_str("/* Default semantic value type */\n");
        code.push_str("typedef int YYSTYPE;\n\n");
    }

    // Dynamic stack with realloc
    code.push_str("/* Dynamic parser stack */\n");
    code.push_str("#define YYINITDEPTH 200\n");
    code.push_str("#define YYMAXDEPTH 10000\n\n");

    code.push_str("typedef struct {\n");
    code.push_str("    int state;\n");
    code.push_str("    YYSTYPE value;\n");
    if grammar.locations {
        code.push_str("    YYLTYPE loc;\n");
    }
    code.push_str("} yystack_item;\n\n");

    code.push_str("static yystack_item *yystack = NULL;\n");
    code.push_str("static int yystack_capacity = 0;\n");
    code.push_str("static int yytop = 0;\n\n");

    // Stack growth function
    code.push_str("static int yygrow_stack(int needed) {\n");
    code.push_str("    if (needed > YYMAXDEPTH) {\n");
    code.push_str("        return -1; /* Stack overflow */\n");
    code.push_str("    }\n");
    code.push_str("    if (needed > yystack_capacity) {\n");
    code.push_str("        int new_cap = yystack_capacity ? yystack_capacity * 2 : YYINITDEPTH;\n");
    code.push_str("        while (new_cap < needed) new_cap *= 2;\n");
    code.push_str("        if (new_cap > YYMAXDEPTH) new_cap = YYMAXDEPTH;\n");
    code.push_str("        yystack_item *new_stack = (yystack_item *)realloc(yystack, new_cap * sizeof(yystack_item));\n");
    code.push_str("        if (!new_stack) return -1;\n");
    code.push_str("        yystack = new_stack;\n");
    code.push_str("        yystack_capacity = new_cap;\n");
    code.push_str("    }\n");
    code.push_str("    return 0;\n");
    code.push_str("}\n\n");

    // Push function
    code.push_str("static int yypush(int state, YYSTYPE value");
    if grammar.locations {
        code.push_str(", YYLTYPE loc");
    }
    code.push_str(") {\n");
    code.push_str("    if (yygrow_stack(yytop + 2) != 0) {\n");
    code.push_str("        fprintf(stderr, \"Parser stack overflow\\n\");\n");
    code.push_str("        return -1;\n");
    code.push_str("    }\n");
    code.push_str("    yytop++;\n");
    code.push_str("    yystack[yytop].state = state;\n");
    code.push_str("    yystack[yytop].value = value;\n");
    if grammar.locations {
        code.push_str("    yystack[yytop].loc = loc;\n");
    }
    code.push_str("    return 0;\n");
    code.push_str("}\n\n");

    // Pop function
    code.push_str("static int yypop(int n) {\n");
    code.push_str("    yytop -= n;\n");
    code.push_str("    return (yytop >= 0) ? yystack[yytop].state : 0;\n");
    code.push_str("}\n\n");

    // Stack cleanup function
    code.push_str("static void yyfree_stack(void) {\n");
    code.push_str("    free(yystack);\n");
    code.push_str("    yystack = NULL;\n");
    code.push_str("    yystack_capacity = 0;\n");
    code.push_str("    yytop = 0;\n");
    code.push_str("}\n\n");

    // Action Lookup Function
    code.push_str("// Action: 0=Error, 1=Shift, 2=Reduce, 3=Accept\n");
    code.push_str("void get_action(int state, const char* token, int* type, int* param) {\n");
    code.push_str("    *type = 0; *param = 0;\n");

    for (state, actions) in &table.action {
        code.push_str(&format!("    if (state == {}) {{\n", state));
        for (term, act) in actions {
            let (t, p) = match act {
                Action::Shift(n) => (1, *n),
                Action::Reduce(n) => (2, *n),
                Action::Accept => (3, 0),
            };
            code.push_str(&format!(
                "        if (strcmp(token, \"{}\") == 0) {{ *type = {}; *param = {}; return; }}\n",
                term, t, p
            ));
        }
        code.push_str("    }\n");
    }
    code.push_str("}\n\n");

    // Goto Lookup Function
    code.push_str("int get_goto(int state, const char* lhs) {\n");
    for (state, gotos) in &table.goto {
        code.push_str(&format!("    if (state == {}) {{\n", state));
        for (nt, next) in gotos {
            code.push_str(&format!(
                "        if (strcmp(lhs, \"{}\") == 0) return {};\n",
                nt, next
            ));
        }
        code.push_str("    }\n");
    }
    code.push_str("    return -1;\n");
    code.push_str("}\n\n");

    // Rule Info
    code.push_str("void get_rule(int id, char** lhs, int* len) {\n");
    code.push_str("    switch(id) {\n");
    for (i, rule) in grammar.rules.iter().enumerate() {
        code.push_str(&format!(
            "        case {}: *lhs = \"{}\"; *len = {}; break;\n",
            i,
            rule.lhs,
            rule.rhs.len()
        ));
    }
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // Global yylval with proper type
    code.push_str("YYSTYPE yylval;\n");
    if grammar.locations {
        code.push_str("YYLTYPE yylloc = {1, 1, 1, 1};\n");
    }
    code.push_str("\n");

    // Error handling macros
    code.push_str("/* Error recovery support */\n");
    code.push_str("#define yyerrok  (yyerrstatus = 0)\n");
    code.push_str("#define yyclearin (yychar = -1)\n");
    code.push_str("static int yynerrs = 0;\n");
    code.push_str("static int yyerrstatus = 0;\n");
    code.push_str("static int yychar = -1;\n\n");

    // External lexer declaration
    code.push_str("/* External lexer function - user must provide */\n");
    code.push_str("extern int yylex(void);\n");
    code.push_str("extern char *yytext;\n");
    code.push_str("extern int yyleng;\n\n");

    // Token name lookup for error messages
    code.push_str("static const char* yytokenname(int token) {\n");
    code.push_str("    switch(token) {\n");
    for (i, tok) in grammar.tokens.iter().enumerate() {
        code.push_str(&format!("        case {}: return \"{}\";\n", i + 256, tok));
    }
    code.push_str("        case 0: return \"$\";\n");
    code.push_str("        default: return \"unknown\";\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // Token count for LAC
    code.push_str(&format!("#define YYNTOKENS {}\n", grammar.tokens.len() + 1));
    code.push_str("static const int yytokens[] = {0");
    for i in 0..grammar.tokens.len() {
        code.push_str(&format!(", {}", i + 256));
    }
    code.push_str("};\n\n");

    // Destructor invocation function if any %destructor declared
    if !grammar.destructors.is_empty() {
        code.push_str("/* Symbol destructors for error recovery cleanup */\n");
        code.push_str("static void yydestruct(const char *msg, int yytype, YYSTYPE *yyvaluep");
        if grammar.locations {
            code.push_str(", YYLTYPE *yylocationp");
        }
        code.push_str(") {\n");
        code.push_str("    (void)msg; (void)yyvaluep;\n");
        if grammar.locations {
            code.push_str("    (void)yylocationp;\n");
        }
        code.push_str("    switch (yytype) {\n");

        // For each destructor, generate cases for matching symbol types
        for destructor in &grammar.destructors {
            for target in &destructor.targets {
                // Find token number for terminal, or symbol type for nonterminal
                if let Some(idx) = grammar.tokens.iter().position(|t| t == target) {
                    code.push_str(&format!("        case {}: /* {} */\n", idx + 256, target));
                    // Substitute $$ with *yyvaluep
                    let cleaned = destructor.code.replace("$$", "(*yyvaluep)");
                    code.push_str(&format!("            {{ {} }}\n", cleaned.trim()));
                    code.push_str("            break;\n");
                }
            }
        }
        code.push_str("        default:\n");
        code.push_str("            break;\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");
    }

    // LAC (Lookahead Correction) exploratory parsing
    if grammar.lac_enabled {
        code.push_str("/* LAC: Lookahead Correction - exploratory parsing */\n");
        code.push_str("static int yy_lac_stack_states[YYMAXDEPTH];\n");
        code.push_str("static int yy_lac_stack_top;\n\n");

        code.push_str("/* Copy current stack for LAC exploration */\n");
        code.push_str("static void yy_lac_setup(void) {\n");
        code.push_str("    yy_lac_stack_top = yytop;\n");
        code.push_str("    for (int i = 0; i <= yytop; i++) {\n");
        code.push_str("        yy_lac_stack_states[i] = yystack[i].state;\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");

        code.push_str("/* Check if a token is acceptable from current LAC stack state */\n");
        code.push_str("static int yy_lac_check(int token) {\n");
        code.push_str("    int lac_top = yy_lac_stack_top;\n");
        code.push_str("    int lac_states[YYMAXDEPTH];\n");
        code.push_str("    for (int i = 0; i <= lac_top; i++) {\n");
        code.push_str("        lac_states[i] = yy_lac_stack_states[i];\n");
        code.push_str("    }\n");
        code.push_str("    int action_type, action_param;\n");
        code.push_str("    char *lhs_name;\n");
        code.push_str("    int rhs_len;\n");
        code.push_str("    /* Simulate parsing: reduce until we can shift or hit error */\n");
        code.push_str("    for (int iter = 0; iter < 1000; iter++) {\n");
        code.push_str("        int state = lac_states[lac_top];\n");
        code.push_str(
            "        get_action(state, yytokenname(token), &action_type, &action_param);\n",
        );
        code.push_str("        if (action_type == 1 || action_type == 3) {\n");
        code.push_str("            /* Shift or Accept: token is valid */\n");
        code.push_str("            return 1;\n");
        code.push_str("        }\n");
        code.push_str("        if (action_type == 2) {\n");
        code.push_str("            /* Reduce: simulate stack operations */\n");
        code.push_str("            get_rule(action_param, &lhs_name, &rhs_len);\n");
        code.push_str("            lac_top -= rhs_len;\n");
        code.push_str("            if (lac_top < 0) return 0;\n");
        code.push_str("            int goto_state = get_goto(lac_states[lac_top], lhs_name);\n");
        code.push_str("            if (goto_state == -1) return 0;\n");
        code.push_str("            lac_top++;\n");
        code.push_str("            lac_states[lac_top] = goto_state;\n");
        code.push_str("        } else {\n");
        code.push_str("            /* Error: token not valid here */\n");
        code.push_str("            return 0;\n");
        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("    return 0; /* Iteration limit hit */\n");
        code.push_str("}\n\n");

        code.push_str("/* Report syntax error with expected token list */\n");
        code.push_str("static void yy_lac_error(int unexpected_token) {\n");
        code.push_str("    yy_lac_setup();\n");
        code.push_str("    fprintf(stderr, \"syntax error: unexpected %s\", yytokenname(unexpected_token));\n");
        code.push_str("    int expected_count = 0;\n");
        code.push_str("    int expected[YYNTOKENS];\n");
        code.push_str("    for (int i = 0; i < YYNTOKENS; i++) {\n");
        code.push_str("        if (yy_lac_check(yytokens[i])) {\n");
        code.push_str("            expected[expected_count++] = yytokens[i];\n");
        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("    if (expected_count > 0 && expected_count <= 5) {\n");
        code.push_str("        fprintf(stderr, \", expected\");\n");
        code.push_str("        for (int i = 0; i < expected_count; i++) {\n");
        code.push_str("            fprintf(stderr, \"%s%s\", (i == 0) ? \" \" : \" or \", yytokenname(expected[i]));\n");
        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("    fprintf(stderr, \"\\n\");\n");
        code.push_str("    yynerrs++;\n");
        code.push_str("}\n\n");
    }

    // Error reporting function (always generate yyerror for user semantic actions)
    code.push_str("void yyerror(const char *msg) {\n");
    code.push_str("    fprintf(stderr, \"%s\\n\", msg);\n");
    code.push_str("    yynerrs++;\n");
    code.push_str("}\n\n");

    if grammar.error_verbose && !grammar.lac_enabled {
        code.push_str("static void yyerror_detailed(int state, int token) {\n");
        code.push_str(
            "    fprintf(stderr, \"syntax error: unexpected %s\", yytokenname(token));\n",
        );
        code.push_str("    fprintf(stderr, \" in state %d\\n\", state);\n");
        code.push_str("    yynerrs++;\n");
        code.push_str("}\n\n");
    }

    // Main parse function
    code.push_str("int yyparse(void) {\n");
    code.push_str("    /* Initialize stack */\n");
    code.push_str("    if (yygrow_stack(YYINITDEPTH) != 0) {\n");
    code.push_str("        fprintf(stderr, \"Out of memory\\n\");\n");
    code.push_str("        return 2;\n");
    code.push_str("    }\n");
    code.push_str("    yytop = 0;\n");
    code.push_str("    yystack[0].state = 0;\n");
    code.push_str("    memset(&yystack[0].value, 0, sizeof(YYSTYPE));\n");
    if grammar.locations {
        code.push_str("    yystack[0].loc = yylloc;\n");
    }
    code.push_str("\n");
    code.push_str("    int yytoken = yylex();\n");
    code.push_str("    int action_type, action_param;\n");
    code.push_str("    char *lhs_name;\n");
    code.push_str("    int rhs_len;\n");
    code.push_str("    YYSTYPE yyval;\n");
    if grammar.locations {
        code.push_str("    YYLTYPE yyloc;\n");
    }
    code.push_str("\n");

    code.push_str("    while (1) {\n");
    code.push_str("        int state = yystack[yytop].state;\n");
    code.push_str(
        "        get_action(state, yytokenname(yytoken), &action_type, &action_param);\n\n",
    );

    code.push_str("        if (action_type == 0) { /* Error */\n");
    if grammar.lac_enabled {
        code.push_str("            yy_lac_error(yytoken);\n");
    } else if grammar.error_verbose {
        code.push_str("            yyerror_detailed(state, yytoken);\n");
    } else {
        code.push_str("            yyerror(\"syntax error\");\n");
    }
    // Call destructors for stack cleanup if any defined
    if !grammar.destructors.is_empty() {
        code.push_str("            /* Cleanup stack using destructors */\n");
        code.push_str("            while (yytop > 0) {\n");
        code.push_str("                yydestruct(\"Error: discarding\", yystack[yytop].state, &yystack[yytop].value");
        if grammar.locations {
            code.push_str(", &yystack[yytop].loc");
        }
        code.push_str(");\n");
        code.push_str("                yytop--;\n");
        code.push_str("            }\n");
    }
    code.push_str("            yyfree_stack();\n");
    code.push_str("            return 1;\n");
    code.push_str("        }\n\n");

    code.push_str("        if (action_type == 1) { /* Shift */\n");
    if grammar.locations {
        code.push_str("            if (yypush(action_param, yylval, yylloc) != 0) {\n");
    } else {
        code.push_str("            if (yypush(action_param, yylval) != 0) {\n");
    }
    code.push_str("                yyfree_stack();\n");
    code.push_str("                return 2;\n");
    code.push_str("            }\n");
    code.push_str("            yytoken = yylex();\n");
    code.push_str("        }\n\n");

    code.push_str("        else if (action_type == 2) { /* Reduce */\n");
    code.push_str("            get_rule(action_param, &lhs_name, &rhs_len);\n");
    code.push_str("            memset(&yyval, 0, sizeof(YYSTYPE));\n");
    code.push_str("            if (rhs_len > 0) {\n");
    code.push_str(
        "                yyval = yystack[yytop - (rhs_len - 1)].value; /* Default: $$ = $1 */\n",
    );
    code.push_str("            }\n");
    if grammar.locations {
        code.push_str("            /* Compute location: from first to last symbol */\n");
        code.push_str("            if (rhs_len > 0) {\n");
        code.push_str(
            "                yyloc.first_line = yystack[yytop - (rhs_len - 1)].loc.first_line;\n",
        );
        code.push_str("                yyloc.first_column = yystack[yytop - (rhs_len - 1)].loc.first_column;\n");
        code.push_str("                yyloc.last_line = yystack[yytop].loc.last_line;\n");
        code.push_str("                yyloc.last_column = yystack[yytop].loc.last_column;\n");
        code.push_str("            } else {\n");
        code.push_str("                yyloc = yylloc;\n");
        code.push_str("            }\n");
    }
    code.push_str("\n");
    code.push_str("            /* Semantic Actions */\n");
    code.push_str("            switch(action_param) {\n");
    for (i, rule) in grammar.rules.iter().enumerate() {
        if let Some(action) = &rule.action {
            let substituted = substitute_vars_c_union(action, rule.rhs.len(), grammar, rule);
            code.push_str(&format!("                case {}: {{\n", i));
            for line in substituted.lines() {
                code.push_str(&format!("                    {}\n", line));
            }
            code.push_str("                } break;\n");
        }
    }
    code.push_str("            }\n\n");

    code.push_str("            yypop(rhs_len);\n");
    code.push_str("            int goto_state = get_goto(yystack[yytop].state, lhs_name);\n");
    code.push_str("            if (goto_state == -1) {\n");
    code.push_str("                yyerror(\"internal goto error\");\n");
    code.push_str("                yyfree_stack();\n");
    code.push_str("                return 1;\n");
    code.push_str("            }\n");
    if grammar.locations {
        code.push_str("            if (yypush(goto_state, yyval, yyloc) != 0) {\n");
    } else {
        code.push_str("            if (yypush(goto_state, yyval) != 0) {\n");
    }
    code.push_str("                yyfree_stack();\n");
    code.push_str("                return 2;\n");
    code.push_str("            }\n");
    code.push_str("        }\n\n");

    code.push_str("        else if (action_type == 3) { /* Accept */\n");
    code.push_str("            yyfree_stack();\n");
    code.push_str("            return 0;\n");
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // Add combined test driver
    code.push_str(&generate_c_parser_test_driver());

    Ok(code)
}

/// Generates a C test driver that shows lexer+parser integration.
fn generate_c_parser_test_driver() -> String {
    let mut code = String::new();
    code.push_str(
        "/* =============================================================================\n",
    );
    code.push_str(" * Combined Lexer + Parser Test Driver\n");
    code.push_str(" * \n");
    code.push_str(" * To use with generated lexer:\n");
    code.push_str(" *   1. Generate lexer: openlexer gen-lexer -l grammar.l -L c -o output/\n");
    code.push_str(
        " *   2. Generate parser: openlexer gen-parser --parser grammar.y -L c -o output/\n",
    );
    code.push_str(" *   3. Compile both: gcc -o parser lexer.c parser.c -DPARSER_NO_MAIN\n");
    code.push_str(" *   4. Or use the built-in test below\n");
    code.push_str(
        " * ============================================================================= */\n\n",
    );
    code.push_str("#ifndef PARSER_NO_MAIN\n");
    code.push_str("/* Extern declarations for lexer integration */\n");
    code.push_str("extern int yylex(void);  /* Returns token type, sets yylval */\n");
    code.push_str("extern YYSTYPE yylval;   /* Semantic value from lexer */\n\n");
    code.push_str("int main(int argc, char **argv) {\n");
    code.push_str("    printf(\"=== OpenLexer Parser Test ===\\n\");\n");
    code.push_str("    int result = yyparse();\n");
    code.push_str("    if (result == 0) {\n");
    code.push_str("        printf(\"Parse successful!\\n\");\n");
    code.push_str("    } else {\n");
    code.push_str("        printf(\"Parse failed with code %d\\n\", result);\n");
    code.push_str("    }\n");
    code.push_str("    return result;\n");
    code.push_str("}\n");
    code.push_str("#endif /* PARSER_NO_MAIN */\n");
    code
}

/// Substitutes $$ and $N references in semantic actions for C output.
/// This version is union-aware: it uses the declared type tags to access
/// the correct union member (e.g., yyval.ival, yystack[idx].value.sval).
fn substitute_vars_c_union(action: &str, rhs_len: usize, grammar: &Grammar, rule: &Rule) -> String {
    let mut out = String::new();
    let chars: Vec<char> = action.chars().collect();
    let mut i = 0;

    // Determine the type tag for $$ (the LHS)
    let lhs_type = grammar.nterm_types.get(&rule.lhs);

    while i < chars.len() {
        // Handle @$ and @N for location access
        if chars[i] == '@' && i + 1 < chars.len() {
            if chars[i + 1] == '$' {
                out.push_str("yyloc");
                i += 2;
                continue;
            } else if chars[i + 1].is_ascii_digit() {
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
                let num_str: String = chars[i + 1..j].iter().collect();
                if let Ok(n) = num_str.parse::<usize>() {
                    if n > 0 && n <= rhs_len {
                        let offset = rhs_len - n;
                        out.push_str(&format!("yystack[yytop - {}].loc", offset));
                    }
                }
                i = j;
                continue;
            }
        }

        // Handle $$ and $N for value access (including $<type>$ and $<type>N)
        if chars[i] == '$' && i + 1 < chars.len() {
            // Check for explicit type cast: $<type>$ or $<type>N
            if chars[i + 1] == '<' {
                // Find closing '>'
                let mut j = i + 2;
                while j < chars.len() && chars[j] != '>' {
                    j += 1;
                }
                if j < chars.len() && chars[j] == '>' {
                    let explicit_type: String = chars[i + 2..j].iter().collect();
                    j += 1; // Move past '>'

                    if j < chars.len() {
                        if chars[j] == '$' {
                            // $<type>$ - result with explicit type
                            out.push_str(&format!("yyval.{}", explicit_type));
                            i = j + 1;
                            continue;
                        } else if chars[j].is_ascii_digit() {
                            // $<type>N - numbered reference with explicit type
                            let mut k = j;
                            while k < chars.len() && chars[k].is_ascii_digit() {
                                k += 1;
                            }
                            let num_str: String = chars[j..k].iter().collect();
                            if let Ok(n) = num_str.parse::<usize>() {
                                if n > 0 && n <= rhs_len {
                                    let offset = rhs_len - n;
                                    out.push_str(&format!(
                                        "yystack[yytop - {}].value.{}",
                                        offset, explicit_type
                                    ));
                                } else {
                                    out.push_str(&format!(
                                        "yystack[yytop].value.{} /* invalid $<{}>{}*/",
                                        explicit_type, explicit_type, n
                                    ));
                                }
                            }
                            i = k;
                            continue;
                        }
                    }
                }
                // Invalid syntax, just emit $
                out.push('$');
                i += 1;
                continue;
            }

            if chars[i + 1] == '$' {
                // $$ - the result value
                if grammar.has_union() {
                    if let Some(tag) = lhs_type {
                        out.push_str(&format!("yyval.{}", tag));
                    } else {
                        out.push_str("yyval"); // No type tag, use whole union
                    }
                } else {
                    out.push_str("yyval");
                }
                i += 2;
            } else if chars[i + 1].is_ascii_digit() {
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
                let num_str: String = chars[i + 1..j].iter().collect();
                if let Ok(n) = num_str.parse::<usize>() {
                    if n > 0 && n <= rhs_len {
                        let offset = rhs_len - n;
                        // Determine the type of $N by looking at the RHS symbol
                        let rhs_sym = &rule.rhs[n - 1];
                        let sym_type = match rhs_sym {
                            Symbol::Terminal(name) => grammar.token_types.get(name),
                            Symbol::NonTerminal(name) => grammar.nterm_types.get(name),
                        };

                        if grammar.has_union() {
                            if let Some(tag) = sym_type {
                                out.push_str(&format!("yystack[yytop - {}].value.{}", offset, tag));
                            } else {
                                out.push_str(&format!("yystack[yytop - {}].value", offset));
                            }
                        } else {
                            out.push_str(&format!("yystack[yytop - {}].value", offset));
                        }
                    } else {
                        // Invalid reference, emit a fallback
                        out.push_str(&format!("yystack[yytop].value /* invalid ${}*/", n));
                    }
                }
                i = j;
            } else {
                out.push('$');
                i += 1;
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// Converts a C type to the equivalent Java type for %union fields.
fn c_type_to_java(c_type: &str) -> String {
    let trimmed = c_type.trim();
    // Handle pointer types
    if trimmed.ends_with('*') {
        // Common pointer types
        if trimmed.contains("char") {
            return "String".to_string();
        }
        // Generic pointer becomes Object
        return "Object".to_string();
    }
    // Handle basic types
    match trimmed {
        "int" => "int".to_string(),
        "long" | "long int" => "long".to_string(),
        "short" | "short int" => "short".to_string(),
        "unsigned" | "unsigned int" => "int".to_string(),
        "unsigned long" => "long".to_string(),
        "float" => "float".to_string(),
        "double" => "double".to_string(),
        "char" => "char".to_string(),
        _ => {
            // Check for struct types
            if trimmed.starts_with("struct ") {
                // struct foo -> Foo (capitalized)
                let name = trimmed.strip_prefix("struct ").unwrap_or(trimmed).trim();
                let mut chars = name.chars();
                match chars.next() {
                    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                    None => "Object".to_string(),
                }
            } else {
                // Unknown type, use Object
                "Object".to_string()
            }
        }
    }
}

fn generate_java(table: &ParsingTable, grammar: &Grammar) -> Result<String> {
    let mut code = String::new();
    code.push_str("import java.util.Stack;\n");
    code.push_str("import java.util.HashMap;\n");
    code.push_str("import java.util.ArrayList;\n\n");
    code.push_str("public class Parser {\n");

    // Generate YYSTYPE class if union is declared
    if grammar.has_union() {
        code.push_str("    /* Semantic value type from %union */\n");
        code.push_str("    public static class YYSTYPE {\n");
        for field in &grammar.union_fields {
            let java_type = c_type_to_java(&field.c_type);
            code.push_str(&format!("        public {} {};\n", java_type, field.name));
        }
        code.push_str("    }\n\n");
    }

    // Generate YYLTYPE if locations enabled
    if grammar.locations {
        code.push_str("    /* Location tracking */\n");
        code.push_str("    public static class YYLTYPE {\n");
        code.push_str("        public int firstLine = 1;\n");
        code.push_str("        public int firstColumn = 1;\n");
        code.push_str("        public int lastLine = 1;\n");
        code.push_str("        public int lastColumn = 1;\n");
        code.push_str("    }\n\n");
    }

    code.push_str("    private static class Action {\n");
    code.push_str("        char type; // 'S', 'R', 'A', 'E'\n");
    code.push_str("        int param;\n");
    code.push_str("        Action(char t, int p) { type = t; param = p; }\n");
    code.push_str("    }\n\n");

    // Stack entry with state, value, and optional location
    code.push_str("    private static class StackEntry {\n");
    code.push_str("        int state;\n");
    if grammar.has_union() {
        code.push_str("        YYSTYPE value;\n");
    } else {
        code.push_str("        Object value;\n");
    }
    if grammar.locations {
        code.push_str("        YYLTYPE loc;\n");
    }
    code.push_str("        StackEntry(int s, ");
    if grammar.has_union() {
        code.push_str("YYSTYPE v");
    } else {
        code.push_str("Object v");
    }
    if grammar.locations {
        code.push_str(", YYLTYPE l");
    }
    code.push_str(") {\n");
    code.push_str("            state = s;\n");
    code.push_str("            value = v;\n");
    if grammar.locations {
        code.push_str("            loc = l;\n");
    }
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    code.push_str(
        "    private HashMap<Integer, HashMap<String, Action>> actionTable = new HashMap<>();\n",
    );
    code.push_str(
        "    private HashMap<Integer, HashMap<String, Integer>> gotoTable = new HashMap<>();\n",
    );
    code.push_str("    private int[][] rules; // [lhs_id, rhs_len]\n");
    code.push_str("    private int yynerrs = 0;\n\n");

    code.push_str("    public Parser() {\n");
    code.push_str("        initTables();\n");
    code.push_str("    }\n\n");

    code.push_str("    private void initTables() {\n");

    // Serialize Action Table
    for (state, actions) in &table.action {
        code.push_str(&format!(
            "        actionTable.put({}, new HashMap<>());\n",
            state
        ));
        for (term, act) in actions {
            let (t, p) = match act {
                Action::Shift(n) => ('S', *n),
                Action::Reduce(n) => ('R', *n),
                Action::Accept => ('A', 0),
            };
            code.push_str(&format!(
                "        actionTable.get({}).put(\"{}\", new Action('{}', {}));\n",
                state, term, t, p
            ));
        }
    }

    // Serialize Goto Table
    for (state, gotos) in &table.goto {
        code.push_str(&format!(
            "        gotoTable.put({}, new HashMap<>());\n",
            state
        ));
        for (nt, next) in gotos {
            code.push_str(&format!(
                "        gotoTable.get({}).put(\"{}\", {});\n",
                state, nt, next
            ));
        }
    }

    // Serialize Rules info
    code.push_str("        rules = new int[][] {\n");
    for rule in &grammar.rules {
        // We need an ID for LHS string? For now just storing length.
        // Real impl needs string map. For MVP using dummy 0 for LHS ID since we use string goto.
        code.push_str(&format!(
            "            {{ 0, {} }}, // {} -> ...\n",
            rule.rhs.len(),
            rule.lhs
        ));
    }
    code.push_str("        };\n");
    code.push_str("    }\n\n");

    code.push_str("    // Helper to get LHS string by rule index\n");
    code.push_str("    private String getLhs(int ruleId) {\n");
    code.push_str("        switch(ruleId) {\n");
    for (i, rule) in grammar.rules.iter().enumerate() {
        code.push_str(&format!(
            "            case {}: return \"{}\";\n",
            i, rule.lhs
        ));
    }
    code.push_str("            default: return \"\";\n");
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    code.push_str("    public int parse(Lexer lexer) {\n");
    code.push_str("        Stack<Integer> stack = new Stack<>();\n");
    code.push_str("        Stack<Integer> valueStack = new Stack<>();\n");
    code.push_str("        stack.push(0);\n");
    code.push_str("        valueStack.push(0);\n");
    code.push_str("        Token token = lexer.nextToken();\n");
    code.push_str("        String sym = token.type;\n");
    code.push_str("        \n");
    code.push_str("        while (true) {\n");
    code.push_str("            int state = stack.peek();\n");
    code.push_str("            if (!actionTable.containsKey(state) || !actionTable.get(state).containsKey(sym)) {\n");
    code.push_str("                throw new RuntimeException(\"Syntax Error\");\n");
    code.push_str("            }\n");
    code.push_str("            \n");
    code.push_str("            Action act = actionTable.get(state).get(sym);\n");
    code.push_str("            if (act.type == 'S') { // Shift\n");
    code.push_str("                stack.push(act.param);\n");
    code.push_str("                valueStack.push(token.value);\n");
    code.push_str("                token = lexer.nextToken();\n");
    code.push_str("                sym = token.type;\n");
    code.push_str("            } else if (act.type == 'R') { // Reduce\n");
    code.push_str("                int ruleId = act.param;\n");
    code.push_str("                int len = rules[ruleId][1];\n");
    code.push_str("                String lhs = getLhs(ruleId);\n");
    code.push_str("                \n");
    code.push_str("                // Get values from stack for semantic action\n");
    code.push_str("                int[] vals = new int[len];\n");
    code.push_str("                for (int i = len - 1; i >= 0; i--) {\n");
    code.push_str("                    vals[i] = valueStack.peek();\n");
    code.push_str("                    valueStack.pop();\n");
    code.push_str("                    stack.pop();\n");
    code.push_str("                }\n");
    code.push_str("                int result = (len > 0) ? vals[0] : 0;\n");
    code.push_str("                \n");
    code.push_str("                // Semantic Actions\n");
    code.push_str("                switch (ruleId) {\n");
    for (i, rule) in grammar.rules.iter().enumerate() {
        if let Some(action) = &rule.action {
            let translated = translate_action_java(action, rule.rhs.len());
            code.push_str(&format!("                    case {}:\n", i));
            for line in translated.lines() {
                if !line.trim().is_empty() {
                    code.push_str(&format!("                        {}\n", line.trim()));
                }
            }
            code.push_str("                        break;\n");
        }
    }
    code.push_str("                }\n");
    code.push_str("                \n");
    code.push_str("                int top = stack.peek();\n");
    code.push_str("                if (gotoTable.containsKey(top) && gotoTable.get(top).containsKey(lhs)) {\n");
    code.push_str("                    stack.push(gotoTable.get(top).get(lhs));\n");
    code.push_str("                    valueStack.push(result);\n");
    code.push_str("                }\n");
    code.push_str("            } else if (act.type == 'A') {\n");
    code.push_str("                return valueStack.peek();\n");
    code.push_str("            }\n");
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("    \n");
    code.push_str("    // Token and Lexer interfaces (implement or use generated lexer)\n");
    code.push_str("    public interface Lexer {\n");
    code.push_str("        Token nextToken();\n");
    code.push_str("    }\n");
    code.push_str("    \n");
    code.push_str("    public static class Token {\n");
    code.push_str("        public String type;\n");
    code.push_str("        public int value;\n");
    code.push_str("        public String text;\n");
    code.push_str("        public Token(String type, int value) {\n");
    code.push_str("            this.type = type;\n");
    code.push_str("            this.value = value;\n");
    code.push_str("            this.text = String.valueOf(value);\n");
    code.push_str("        }\n");
    code.push_str("        public Token(String type, String text) {\n");
    code.push_str("            this.type = type;\n");
    code.push_str("            this.text = text;\n");
    code.push_str("            try { this.value = Integer.parseInt(text); } catch (Exception e) { this.value = 0; }\n");
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("    \n");

    // Add test driver
    code.push_str(&generate_java_parser_test_driver());

    code.push_str("}\n");
    Ok(code)
}

/// Generates a Java test driver for lexer+parser integration.
fn generate_java_parser_test_driver() -> String {
    let mut code = String::new();
    code.push_str(
        "    /* =========================================================================\n",
    );
    code.push_str("     * Combined Lexer + Parser Test Driver\n");
    code.push_str("     * \n");
    code.push_str("     * To use with generated Lexer.java:\n");
    code.push_str(
        "     *   1. Generate lexer: openlexer gen-lexer -l grammar.l -L java -o output/\n",
    );
    code.push_str(
        "     *   2. Generate parser: openlexer gen-parser --parser grammar.y -L java -o output/\n",
    );
    code.push_str("     *   3. Compile: javac Lexer.java Parser.java\n");
    code.push_str("     *   4. Run: java Parser \"3 + 4 * 2\"\n");
    code.push_str(
        "     * ========================================================================= */\n",
    );
    code.push_str("    \n");
    code.push_str("    /** Adapter to use generated Lexer with Parser. */\n");
    code.push_str("    public static class LexerAdapter implements Lexer {\n");
    code.push_str("        private Object lexer;\n");
    code.push_str("        private java.lang.reflect.Method nextTokenMethod;\n");
    code.push_str("        \n");
    code.push_str("        public LexerAdapter(Object lexer) {\n");
    code.push_str("            this.lexer = lexer;\n");
    code.push_str("            try {\n");
    code.push_str(
        "                this.nextTokenMethod = lexer.getClass().getMethod(\"nextToken\");\n",
    );
    code.push_str("            } catch (Exception e) {\n");
    code.push_str(
        "                throw new RuntimeException(\"Lexer must have nextToken() method\", e);\n",
    );
    code.push_str("            }\n");
    code.push_str("        }\n");
    code.push_str("        \n");
    code.push_str("        @Override\n");
    code.push_str("        public Token nextToken() {\n");
    code.push_str("            try {\n");
    code.push_str("                Object tok = nextTokenMethod.invoke(lexer);\n");
    code.push_str("                // Get type and text via reflection\n");
    code.push_str("                Object typeObj = tok.getClass().getField(\"type\").get(tok);\n");
    code.push_str("                String type = typeObj.toString();\n");
    code.push_str("                if (type.equals(\"EOF\")) type = \"$\";\n");
    code.push_str(
        "                String text = (String) tok.getClass().getField(\"text\").get(tok);\n",
    );
    code.push_str("                return new Token(type, text);\n");
    code.push_str("            } catch (Exception e) {\n");
    code.push_str("                throw new RuntimeException(\"Failed to get next token\", e);\n");
    code.push_str("            }\n");
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("    \n");
    code.push_str("    /** Test parsing an expression. */\n");
    code.push_str("    public static void testParse(String expr) {\n");
    code.push_str("        System.out.println(\"Parsing: \\\"\" + expr + \"\\\"\");\n");
    code.push_str("        try {\n");
    code.push_str("            // Try to load and use generated Lexer\n");
    code.push_str("            Class<?> lexerClass = Class.forName(\"Lexer\");\n");
    code.push_str(
        "            Object lexer = lexerClass.getConstructor(String.class).newInstance(expr);\n",
    );
    code.push_str("            LexerAdapter adapter = new LexerAdapter(lexer);\n");
    code.push_str("            Parser parser = new Parser(adapter);\n");
    code.push_str("            int result = parser.parse();\n");
    code.push_str("            System.out.println(\"  Result: \" + result);\n");
    code.push_str("        } catch (ClassNotFoundException e) {\n");
    code.push_str("            System.err.println(\"  Error: Lexer.class not found. Generate and compile Lexer.java first.\");\n");
    code.push_str("        } catch (Exception e) {\n");
    code.push_str("            System.err.println(\"  Error: \" + e.getMessage());\n");
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("    \n");
    code.push_str("    public static void main(String[] args) {\n");
    code.push_str("        System.out.println(\"=== OpenLexer Parser Test ===\");\n");
    code.push_str("        System.out.println();\n");
    code.push_str("        \n");
    code.push_str("        if (args.length > 0) {\n");
    code.push_str("            for (String arg : args) {\n");
    code.push_str("                testParse(arg);\n");
    code.push_str("            }\n");
    code.push_str("        } else {\n");
    code.push_str("            try {\n");
    code.push_str("                java.util.Scanner sc = new java.util.Scanner(System.in);\n");
    code.push_str("                boolean hasInput = false;\n");
    code.push_str("                while (sc.hasNextLine()) {\n");
    code.push_str("                    String line = sc.nextLine().trim();\n");
    code.push_str("                    if (!line.isEmpty()) { testParse(line); hasInput = true; }\n");
    code.push_str("                }\n");
    code.push_str("                if (!hasInput) {\n");
    code.push_str("                    testParse(\"3 + 4\");\n");
    code.push_str("                    testParse(\"3 + 4 * 2\");\n");
    code.push_str("                }\n");
    code.push_str("            } catch (Exception e) {\n");
    code.push_str("                testParse(\"3 + 4 * 2\");\n");
    code.push_str("            }\n");
    code.push_str("        }\n");
    code.push_str("    }\n");
    code
}

/// Translate Bison-style semantic action to Java.
/// Converts $$ to result, $1/$2/etc to vals[0]/vals[1]/etc.
fn translate_action_java(action: &str, rhs_len: usize) -> String {
    let mut out = String::new();
    let chars: Vec<char> = action.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '$' && i + 1 < chars.len() {
            if chars[i + 1] == '$' {
                out.push_str("result");
                i += 2;
            } else if chars[i + 1].is_ascii_digit() {
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
                let num_str: String = chars[i + 1..j].iter().collect();
                if let Ok(n) = num_str.parse::<usize>() {
                    if n > 0 && n <= rhs_len {
                        out.push_str(&format!("vals[{}]", n - 1));
                    } else {
                        out.push_str("0"); // Invalid reference
                    }
                }
                i = j;
            } else {
                out.push('$');
                i += 1;
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }

    // Convert C patterns to Java
    out = out.replace("printf(", "System.out.printf(");
    out = out.replace("pow(", "(int)Math.pow(");

    // Remove C-specific tokens that have no Java equivalent
    out = out.replace("yyerrok", "/* yyerrok */");

    out
}

fn generate_python(table: &ParsingTable, grammar: &Grammar) -> Result<String> {
    let mut code = String::new();
    code.push_str("# Generated by OpenLexer\n\n");
    code.push_str("class Parser:\n");
    code.push_str("    def __init__(self, lexer):\n");
    code.push_str("        self.lexer = lexer\n");
    code.push_str("        self.action = {\n");

    // Serialize Action Table
    for (state, actions) in &table.action {
        code.push_str(&format!("            {}: {{", state));
        for (term, act) in actions {
            match act {
                Action::Shift(next) => code.push_str(&format!("'{}': ('S', {}), ", term, next)),
                Action::Reduce(rule) => code.push_str(&format!("'{}': ('R', {}), ", term, rule)),
                Action::Accept => code.push_str(&format!("'{}': ('A', 0), ", term)),
            }
        }
        code.push_str("},\n");
    }
    code.push_str("        }\n");

    code.push_str("        self.goto = {\n");
    for (state, gotos) in &table.goto {
        code.push_str(&format!("            {}: {{", state));
        for (nt, next) in gotos {
            code.push_str(&format!("'{}': {}, ", nt, next));
        }
        code.push_str("},\n");
    }
    code.push_str("        }\n");

    // LHS and RHS_LEN lookup for Reductions
    code.push_str("        self.rules = [\n");
    for rule in &grammar.rules {
        code.push_str(&format!(
            "            ('{}', {}),\n",
            rule.lhs,
            rule.rhs.len()
        ));
    }
    code.push_str("        ]\n\n");

    code.push_str("    def parse(self):\n");
    code.push_str("        state_stack = [0]\n");
    code.push_str("        value_stack = [None]  # Semantic values\n");
    code.push_str("        sym = self.lexer.next_token()\n");
    code.push_str("        \n");
    code.push_str("        while True:\n");
    code.push_str("            state = state_stack[-1]\n");
    code.push_str("            token_type = sym.type if hasattr(sym, 'type') else str(sym)\n");
    code.push_str("            \n");
    code.push_str(
        "            if state not in self.action or token_type not in self.action[state]:\n",
    );
    code.push_str(
        "                raise SyntaxError(f'Unexpected token {sym} in state {state}')\n",
    );
    code.push_str("            \n");
    code.push_str("            act, param = self.action[state][token_type]\n");
    code.push_str("            \n");
    code.push_str("            if act == 'S':  # Shift\n");
    code.push_str("                state_stack.append(param)\n");
    code.push_str("                # Push token value onto value stack\n");
    code.push_str("                value_stack.append(getattr(sym, 'value', sym.text if hasattr(sym, 'text') else sym))\n");
    code.push_str("                sym = self.lexer.next_token()\n");
    code.push_str("                \n");
    code.push_str("            elif act == 'R':  # Reduce\n");
    code.push_str("                lhs, length = self.rules[param]\n");
    code.push_str("                \n");
    code.push_str("                # Get values from stack for semantic action\n");
    code.push_str("                if length > 0:\n");
    code.push_str("                    vals = value_stack[-length:]\n");
    code.push_str("                else:\n");
    code.push_str("                    vals = []\n");
    code.push_str("                \n");
    code.push_str("                # Default: result = first value or None\n");
    code.push_str("                result = vals[0] if vals else None\n");
    code.push_str("                \n");
    code.push_str("                # Semantic action by rule\n");
    code.push_str("                rule_id = param\n");

    // Generate semantic actions
    for (i, rule) in grammar.rules.iter().enumerate() {
        if let Some(action) = &rule.action {
            let translated = translate_action_python(action, rule.rhs.len());
            code.push_str(&format!("                if rule_id == {}:\n", i));
            for line in translated.lines() {
                if !line.trim().is_empty() {
                    code.push_str(&format!("                    {}\n", line.trim()));
                }
            }
        }
    }

    code.push_str("                \n");
    code.push_str("                # Pop stacks\n");
    code.push_str("                for _ in range(length):\n");
    code.push_str("                    state_stack.pop()\n");
    code.push_str("                    value_stack.pop()\n");
    code.push_str("                \n");
    code.push_str("                # Push result and goto state\n");
    code.push_str("                top = state_stack[-1]\n");
    code.push_str("                if top in self.goto and lhs in self.goto[top]:\n");
    code.push_str("                    state_stack.append(self.goto[top][lhs])\n");
    code.push_str("                    value_stack.append(result)\n");
    code.push_str("                else:\n");
    code.push_str("                    raise SyntaxError(f'No goto for {lhs} from state {top}')\n");
    code.push_str("                    \n");
    code.push_str("            elif act == 'A':  # Accept\n");
    code.push_str("                return value_stack[-1] if len(value_stack) > 1 else None\n");

    // Add combined lexer+parser test driver
    code.push_str("\n\n");
    code.push_str(&generate_python_parser_test_driver());

    Ok(code)
}

/// Generates a Python test driver that integrates lexer and parser.
pub fn generate_python_parser_test_driver() -> String {
    let mut code = String::new();
    code.push_str(
        "# =============================================================================\n",
    );
    code.push_str("# Combined Lexer + Parser Test Driver\n");
    code.push_str(
        "# =============================================================================\n\n",
    );
    code.push_str("class LexerAdapter:\n");
    code.push_str("    \"\"\"Adapts a lexer to work with the parser.\n");
    code.push_str("    \n");
    code.push_str("    Usage:\n");
    code.push_str("        from lexer import Lexer, TokenType\n");
    code.push_str("        from parser import Parser, LexerAdapter\n");
    code.push_str("        \n");
    code.push_str("        adapter = LexerAdapter(Lexer('3 + 4 * 2'))\n");
    code.push_str("        parser = Parser(adapter)\n");
    code.push_str("        result = parser.parse()\n");
    code.push_str("    \"\"\"\n");
    code.push_str("    def __init__(self, lexer):\n");
    code.push_str("        self.lexer = lexer\n");
    code.push_str("    \n");
    code.push_str("    def next_token(self):\n");
    code.push_str("        tok = self.lexer.next_token()\n");
    code.push_str("        # Return adapted token with string type name\n");
    code.push_str("        return AdaptedToken(tok)\n\n\n");
    code.push_str("class AdaptedToken:\n");
    code.push_str("    \"\"\"Token adapter for parser compatibility.\"\"\"\n");
    code.push_str("    def __init__(self, tok):\n");
    code.push_str("        # Convert enum to string ('$' for EOF)\n");
    code.push_str("        self.type = '$' if tok.type.name == 'EOF' else tok.type.name\n");
    code.push_str("        self.text = tok.text\n");
    code.push_str("        self.pos = tok.pos\n");
    code.push_str("        # Extract numeric value for NUMBER tokens\n");
    code.push_str("        try:\n");
    code.push_str("            self.value = int(tok.text) if tok.text.isdigit() else tok.text\n");
    code.push_str("        except:\n");
    code.push_str("            self.value = tok.text\n\n\n");
    code.push_str("def parse_expression(expr: str, lexer_class=None):\n");
    code.push_str("    \"\"\"Parse an expression string and return the result.\n");
    code.push_str("    \n");
    code.push_str("    Args:\n");
    code.push_str("        expr: The expression to parse\n");
    code.push_str(
        "        lexer_class: Optional custom Lexer class (defaults to imported Lexer)\n",
    );
    code.push_str("    \n");
    code.push_str("    Returns:\n");
    code.push_str("        The parse result (semantic value from reduction)\n");
    code.push_str("    \n");
    code.push_str("    Example:\n");
    code.push_str("        >>> parse_expression('3 + 4')\n");
    code.push_str("        7\n");
    code.push_str("    \"\"\"\n");
    code.push_str("    try:\n");
    code.push_str("        from lexer import Lexer as DefaultLexer\n");
    code.push_str("        LexerClass = lexer_class or DefaultLexer\n");
    code.push_str("    except ImportError:\n");
    code.push_str(
        "        raise ImportError('Could not import Lexer. Generate lexer.py first.')\n",
    );
    code.push_str("    \n");
    code.push_str("    adapter = LexerAdapter(LexerClass(expr))\n");
    code.push_str("    parser = Parser(adapter)\n");
    code.push_str("    return parser.parse()\n\n\n");
    code.push_str("def test_parse(expr: str):\n");
    code.push_str("    \"\"\"Test parsing an expression and print the result.\"\"\"\n");
    code.push_str("    print(f'Parsing: {expr!r}')\n");
    code.push_str("    try:\n");
    code.push_str("        result = parse_expression(expr)\n");
    code.push_str("        print(f'  Result: {result}')\n");
    code.push_str("        return result\n");
    code.push_str("    except Exception as e:\n");
    code.push_str("        print(f'  Error: {e}')\n");
    code.push_str("        return None\n\n\n");
    code.push_str("if __name__ == '__main__':\n");
    code.push_str("    import sys\n");
    code.push_str("    print('=== OpenLexer Parser Test Driver ===')\n");
    code.push_str("    print()\n");
    code.push_str("    \n");
    code.push_str("    if len(sys.argv) > 1:\n");
    code.push_str("        for arg in sys.argv[1:]:\n");
    code.push_str("            test_parse(arg)\n");
    code.push_str("    else:\n");
    code.push_str("        _input = sys.stdin.read().strip()\n");
    code.push_str("        if _input:\n");
    code.push_str("            for line in _input.splitlines():\n");
    code.push_str("                test_parse(line)\n");
    code.push_str("        else:\n");
    code.push_str("            test_parse('3 + 4')\n");
    code.push_str("            test_parse('3 + 4 * 2')\n");
    code
}

/// Translate Bison-style semantic action to Python.
/// Converts $$ to result, $1/$2/etc to vals[0]/vals[1]/etc.
fn translate_action_python(action: &str, rhs_len: usize) -> String {
    let mut out = String::new();
    let chars: Vec<char> = action.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '$' && i + 1 < chars.len() {
            if chars[i + 1] == '$' {
                out.push_str("result");
                i += 2;
            } else if chars[i + 1].is_ascii_digit() {
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
                let num_str: String = chars[i + 1..j].iter().collect();
                if let Ok(n) = num_str.parse::<usize>() {
                    if n > 0 && n <= rhs_len {
                        out.push_str(&format!("vals[{}]", n - 1));
                    } else {
                        out.push_str("None"); // Invalid reference
                    }
                }
                i = j;
            } else {
                out.push('$');
                i += 1;
            }
        } else if chars[i] == ';' {
            // Skip semicolons (C statement terminator)
            i += 1;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }

    // Convert some common C patterns to Python
    out = out.replace("printf(", "print(");
    out = out.replace("%d", "{}");
    out = out.replace("\\n", "");

    out
}
