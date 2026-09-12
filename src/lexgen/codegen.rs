//! Code generation for table-driven lexers.

use crate::error::Result;
use crate::lexgen::dfa::Dfa;
use crate::lexgen::nfa::Nfa;
use crate::lexgen::rules::{LexerSpec, RuleAction};
use std::collections::HashMap;

pub enum TargetLanguage {
    C,
    Java,
    Python,
}


/// Generates a lexer from a DFA (legacy single-pattern mode).
pub fn generate_lexer(dfa: &Dfa, lang: TargetLanguage) -> Result<String> {
    match lang {
        TargetLanguage::C => generate_c_simple(dfa),
        TargetLanguage::Java => generate_java_simple(dfa),
        TargetLanguage::Python => generate_python_simple(dfa),
    }
}

/// Generates a full lexer from a lexer specification with start conditions.
/// Builds separate DFAs for each start condition for optimal performance.
pub fn generate_lexer_from_spec_with_conditions(
    spec: &LexerSpec,
    lang: TargetLanguage,
) -> Result<String> {
    // Build DFAs for each start condition. `dfas` is used whenever we're at the
    // beginning of a line (bol) and also serves as the fallback table otherwise.
    // `non_bol_dfas` only gets an entry for a condition when it actually has a
    // `^`-anchored rule - most lexers never use `^`, so they pay nothing extra.
    let mut dfas: HashMap<String, Dfa> = HashMap::new();
    let mut non_bol_dfas: HashMap<String, Dfa> = HashMap::new();

    for condition in spec.start_conditions.keys() {
        let bol_nfa = Nfa::from_lexer_spec_for_condition(spec, condition, true)?;
        dfas.insert(condition.clone(), Dfa::from_nfa(&bol_nfa)?);

        if Nfa::condition_has_bol_rules(spec, condition) {
            let non_bol_nfa = Nfa::from_lexer_spec_for_condition(spec, condition, false)?;
            non_bol_dfas.insert(condition.clone(), Dfa::from_nfa(&non_bol_nfa)?);
        }
    }

    match lang {
        TargetLanguage::C => generate_c_with_conditions(&dfas, &non_bol_dfas, spec),
        TargetLanguage::Java => generate_java_with_conditions(&dfas, &non_bol_dfas, spec),
        TargetLanguage::Python => generate_python_with_conditions(&dfas, &non_bol_dfas, spec),
    }
}

/// Generates a full lexer from a DFA and lexer specification (multi-token mode).
/// Legacy API - does not support start conditions properly.
pub fn generate_lexer_from_spec(
    dfa: &Dfa,
    spec: &LexerSpec,
    lang: TargetLanguage,
) -> Result<String> {
    let trailing_dfas = build_trailing_context_dfas(spec)?;

    // If spec has only INITIAL condition and no `^` (beginning-of-line) anchors,
    // use the simple path - the condition-aware path below carries extra
    // machinery (a StartCondition enum, condition-keyed tables) that only pays
    // for itself when start conditions or `^` anchors are actually in use.
    // (Trailing context isn't yet wired into that condition-aware path, so a
    // rule combining `/` with explicit %s/%x, or with a `^` anchor that itself
    // routes the whole spec through this path, falls back to matching the
    // full r1r2 text unsplit rather than erroring.)
    if spec.start_conditions.len() <= 1 && !spec.rules.iter().any(|r| r.anchored_start) {
        match lang {
            TargetLanguage::C => generate_c_full(dfa, spec, &trailing_dfas),
            TargetLanguage::Java => generate_java_full(dfa, spec, &trailing_dfas),
            TargetLanguage::Python => generate_python_full(dfa, spec, &trailing_dfas),
        }
    } else {
        // Use the new condition-aware generator
        generate_lexer_from_spec_with_conditions(spec, lang)
    }
}

/// Small standalone DFAs for a rule's `r1` and `r2` (from `r1/r2` trailing
/// context), keyed by rule index. The generated lexer uses both to find
/// where r1 ends: r1 alone can accept more than one prefix of the matched
/// text (e.g. `a*` on "aaaa" accepts at every position), so the split point
/// is the longest r1-accepted prefix whose remaining suffix is an *exact*
/// match for r2 - not just the longest r1 prefix on its own, which would
/// give the wrong boundary for a pattern like `a*/a` (see build_trailing_context_dfas).
struct TrailingContextDfas {
    r1: HashMap<usize, Dfa>,
    r2: HashMap<usize, Dfa>,
}

impl TrailingContextDfas {
    fn is_empty(&self) -> bool {
        self.r1.is_empty()
    }
}

/// Builds `r1`/`r2` DFAs for every rule with trailing context (`r1/r2`). The
/// generated lexer re-scans a rule's matched text against these to find
/// where r1 ends, so only r1's text becomes the token and r2 is left
/// unconsumed for the next match.
fn build_trailing_context_dfas(spec: &LexerSpec) -> Result<TrailingContextDfas> {
    let mut r1_dfas = HashMap::new();
    let mut r2_dfas = HashMap::new();
    for (idx, rule) in spec.rules.iter().enumerate() {
        if let Some(r1) = &rule.trailing_context {
            let nfa = Nfa::from_regex(&r1.root)?;
            r1_dfas.insert(idx, Dfa::from_nfa(&nfa)?);
        }
        if let Some(r2) = &rule.trailing_context_r2 {
            let nfa = Nfa::from_regex(&r2.root)?;
            r2_dfas.insert(idx, Dfa::from_nfa(&nfa)?);
        }
    }
    Ok(TrailingContextDfas {
        r1: r1_dfas,
        r2: r2_dfas,
    })
}

// =============================================================================
// Full multi-token lexer generation with start conditions
// =============================================================================

fn generate_python_with_conditions(
    dfas: &HashMap<String, Dfa>,
    non_bol_dfas: &HashMap<String, Dfa>,
    spec: &LexerSpec,
) -> Result<String> {
    let mut code = String::new();

    code.push_str("\"\"\"Lexer generated by OpenLexer with start condition support.\"\"\"\n\n");
    code.push_str("from enum import Enum, auto\n");
    code.push_str("from dataclasses import dataclass\n");
    code.push_str("from typing import Optional, Dict, Tuple, List\n\n");

    // Token type enum
    code.push_str("class TokenType(Enum):\n");
    code.push_str("    EOF = auto()\n");
    code.push_str("    ERROR = auto()\n");
    let mut seen_tokens = std::collections::HashSet::new();
    seen_tokens.insert("EOF".to_string());
    seen_tokens.insert("ERROR".to_string());
    for rule in &spec.rules {
        match &rule.action {
            RuleAction::Token(name) | RuleAction::TokenAndBegin(name, _) => {
                let upper = name.to_uppercase();
                if seen_tokens.insert(upper.clone()) {
                    code.push_str(&format!("    {} = auto()\n", upper));
                }
            }
            _ => {}
        }
    }
    code.push_str("\n\n");

    // Start condition enum
    code.push_str("class StartCondition(Enum):\n");
    for (idx, cond) in spec.start_conditions.keys().enumerate() {
        code.push_str(&format!("    {} = {}\n", cond, idx));
    }
    code.push_str("\n\n");

    // Token dataclass
    code.push_str("@dataclass\n");
    code.push_str("class Token:\n");
    code.push_str("    type: TokenType\n");
    code.push_str("    text: str\n");
    code.push_str("    pos: int\n");
    code.push_str("    line: int = 1\n");
    code.push_str("    column: int = 1\n\n");

    // Rule to action mapping (token type and optional state change)
    code.push_str("# Maps rule_index -> (token_type or None, new_state or None)\n");
    code.push_str(
        "RULE_ACTIONS: Dict[int, Tuple[Optional[TokenType], Optional[StartCondition]]] = {\n",
    );
    for (idx, rule) in spec.rules.iter().enumerate() {
        let (token_str, state_str) = match &rule.action {
            RuleAction::Token(name) => (
                format!("TokenType.{}", name.to_uppercase()),
                "None".to_string(),
            ),
            RuleAction::TokenAndBegin(name, state) => (
                format!("TokenType.{}", name.to_uppercase()),
                format!("StartCondition.{}", state),
            ),
            RuleAction::Skip => ("None".to_string(), "None".to_string()),
            RuleAction::Begin(state) => ("None".to_string(), format!("StartCondition.{}", state)),
            RuleAction::Error => ("TokenType.ERROR".to_string(), "None".to_string()),
            RuleAction::Code(_) => {
                // Code actions are handled separately in the action switch
                ("None".to_string(), "None".to_string())
            }
        };
        code.push_str(&format!("    {}: ({}, {}),\n", idx, token_str, state_str));
    }
    code.push_str("}\n\n");

    // Generate range-based transition tables for each start condition
    code.push_str("# DFA range-based transition tables per start condition\n");
    code.push_str("# Format: {state: [(start_codepoint, end_codepoint, target_state), ...]}\n");
    code.push_str(
        "RANGE_TRANSITIONS: Dict[StartCondition, Dict[int, List[Tuple[int, int, int]]]] = {\n",
    );
    for (cond, dfa) in dfas {
        code.push_str(&format!("    StartCondition.{}: {{\n", cond));
        for state in &dfa.states {
            if !state.range_transitions.is_empty() {
                code.push_str(&format!("        {}: [", state.id));
                for &(start_cp, end_cp, target) in &state.range_transitions {
                    code.push_str(&format!("(0x{:X}, 0x{:X}, {}), ", start_cp, end_cp, target));
                }
                code.push_str("],\n");
            } else if !state.transitions.is_empty() {
                // Fallback to char-based as ranges
                code.push_str(&format!("        {}: [", state.id));
                for (ch, target) in &state.transitions {
                    let cp = *ch as u32;
                    code.push_str(&format!("(0x{:X}, 0x{:X}, {}), ", cp, cp, target));
                }
                code.push_str("],\n");
            }
        }
        code.push_str("    },\n");
    }
    code.push_str("}\n\n");

    // Accepting states per condition
    code.push_str("# Accepting states: condition -> {state_id: rule_index}\n");
    code.push_str("ACCEPTING: Dict[StartCondition, Dict[int, int]] = {\n");
    for (cond, dfa) in dfas {
        code.push_str(&format!("    StartCondition.{}: {{\n", cond));
        for state in &dfa.states {
            if state.is_accepting {
                if let Some(rule_idx) = state.rule_index {
                    code.push_str(&format!("        {}: {},\n", state.id, rule_idx));
                }
            }
        }
        code.push_str("    },\n");
    }
    code.push_str("}\n\n");

    // Not-beginning-of-line tables: only present for a condition that actually
    // has a `^`-anchored rule (which can't match unless at bol). When a
    // condition has no entry here, the tables above serve both cases.
    code.push_str("# '^' (beginning-of-line) anchor support: tables to use when NOT at\n");
    code.push_str("# the start of a line. A condition missing here has no '^' rules, so\n");
    code.push_str("# RANGE_TRANSITIONS/ACCEPTING above are used regardless of bol state.\n");
    code.push_str(
        "NON_BOL_RANGE_TRANSITIONS: Dict[StartCondition, Dict[int, List[Tuple[int, int, int]]]] = {\n",
    );
    for (cond, dfa) in non_bol_dfas {
        code.push_str(&format!("    StartCondition.{}: {{\n", cond));
        for state in &dfa.states {
            if !state.range_transitions.is_empty() {
                code.push_str(&format!("        {}: [", state.id));
                for &(start_cp, end_cp, target) in &state.range_transitions {
                    code.push_str(&format!("(0x{:X}, 0x{:X}, {}), ", start_cp, end_cp, target));
                }
                code.push_str("],\n");
            } else if !state.transitions.is_empty() {
                code.push_str(&format!("        {}: [", state.id));
                for (ch, target) in &state.transitions {
                    let cp = *ch as u32;
                    code.push_str(&format!("(0x{:X}, 0x{:X}, {}), ", cp, cp, target));
                }
                code.push_str("],\n");
            }
        }
        code.push_str("    },\n");
    }
    code.push_str("}\n\n");

    code.push_str("NON_BOL_ACCEPTING: Dict[StartCondition, Dict[int, int]] = {\n");
    for (cond, dfa) in non_bol_dfas {
        code.push_str(&format!("    StartCondition.{}: {{\n", cond));
        for state in &dfa.states {
            if state.is_accepting {
                if let Some(rule_idx) = state.rule_index {
                    code.push_str(&format!("        {}: {},\n", state.id, rule_idx));
                }
            }
        }
        code.push_str("    },\n");
    }
    code.push_str("}\n\n");

    // Lexer class with start condition support
    code.push_str("class Lexer:\n");
    code.push_str("    def __init__(self, input_str: str):\n");
    code.push_str("        self.input = input_str\n");
    code.push_str("        self.pos = 0\n");
    code.push_str("        self.line = 1\n");
    code.push_str("        self.column = 1\n");
    code.push_str("        self.condition = StartCondition.INITIAL\n");
    code.push_str("        self.at_bol = True  # start of input counts as beginning-of-line\n");
    code.push_str("        self._condition_stack: list = []\n\n");

    code.push_str("    def begin(self, condition: StartCondition):\n");
    code.push_str("        \"\"\"Switch to a new start condition.\"\"\"\n");
    code.push_str("        self.condition = condition\n\n");

    code.push_str("    def push_state(self, condition: StartCondition):\n");
    code.push_str("        \"\"\"Push current condition and switch to new one.\"\"\"\n");
    code.push_str("        self._condition_stack.append(self.condition)\n");
    code.push_str("        self.condition = condition\n\n");

    code.push_str("    def pop_state(self):\n");
    code.push_str("        \"\"\"Pop and restore previous condition.\"\"\"\n");
    code.push_str("        if self._condition_stack:\n");
    code.push_str("            self.condition = self._condition_stack.pop()\n\n");

    code.push_str("    def _get_next_state(self, state: int, codepoint: int, trans_table) -> int:\n");
    code.push_str("        \"\"\"Find next state using range-based transitions.\"\"\"\n");
    code.push_str("        ranges = trans_table.get(state, [])\n");
    code.push_str("        for start_cp, end_cp, target in ranges:\n");
    code.push_str("            if start_cp <= codepoint <= end_cp:\n");
    code.push_str("                return target\n");
    code.push_str("        return -1\n\n");

    code.push_str("    def next_token(self) -> Token:\n");
    code.push_str("        while self.pos < len(self.input):\n");
    code.push_str("            start = self.pos\n");
    code.push_str("            start_line = self.line\n");
    code.push_str("            start_column = self.column\n");
    code.push_str("            state = 0\n");
    code.push_str("            last_accepting_rule = -1\n");
    code.push_str("            last_accepting_pos = start\n\n");

    code.push_str("            # '^' anchor: use the not-at-bol tables only when this condition\n");
    code.push_str("            # actually has one; otherwise the normal tables cover both cases.\n");
    code.push_str("            if not self.at_bol and self.condition in NON_BOL_ACCEPTING:\n");
    code.push_str("                trans_table = NON_BOL_RANGE_TRANSITIONS.get(self.condition, {})\n");
    code.push_str("                accept_table = NON_BOL_ACCEPTING.get(self.condition, {})\n");
    code.push_str("            else:\n");
    code.push_str("                trans_table = RANGE_TRANSITIONS.get(self.condition, {})\n");
    code.push_str("                accept_table = ACCEPTING.get(self.condition, {})\n\n");

    code.push_str("            while self.pos < len(self.input):\n");
    code.push_str("                c = self.input[self.pos]\n");
    code.push_str("                codepoint = ord(c)\n");
    code.push_str("                next_state = self._get_next_state(state, codepoint, trans_table)\n");
    code.push_str("                if next_state == -1:\n");
    code.push_str("                    break\n");
    code.push_str("                state = next_state\n");
    code.push_str("                self.pos += 1\n");
    code.push_str("                # Track line/column\n");
    code.push_str("                if c == '\\n':\n");
    code.push_str("                    self.line += 1\n");
    code.push_str("                    self.column = 1\n");
    code.push_str("                else:\n");
    code.push_str("                    self.column += 1\n\n");

    code.push_str("                if state in accept_table:\n");
    code.push_str("                    last_accepting_rule = accept_table[state]\n");
    code.push_str("                    last_accepting_pos = self.pos\n\n");

    code.push_str("            if last_accepting_rule >= 0:\n");
    code.push_str("                self.pos = last_accepting_pos\n");
    code.push_str(
        "                action = RULE_ACTIONS.get(last_accepting_rule, (TokenType.ERROR, None))\n",
    );
    code.push_str("                token_type, new_condition = action\n");
    code.push_str("                text = self.input[start:last_accepting_pos]\n");
    code.push_str("                self.at_bol = text.endswith('\\n')\n\n");
    code.push_str("                # Handle state change\n");
    code.push_str("                if new_condition is not None:\n");
    code.push_str("                    self.condition = new_condition\n\n");
    code.push_str("                if token_type is None:  # Skip\n");
    code.push_str("                    continue\n");
    code.push_str(
        "                return Token(token_type, text, start, start_line, start_column)\n",
    );
    code.push_str("            else:\n");
    code.push_str("                # No match - return error token for single char\n");
    code.push_str("                self.pos = start + 1\n");
    code.push_str("                self.at_bol = self.input[start] == '\\n'\n");
    code.push_str("                if self.input[start] == '\\n':\n");
    code.push_str("                    self.line += 1\n");
    code.push_str("                    self.column = 1\n");
    code.push_str("                else:\n");
    code.push_str("                    self.column += 1\n");
    code.push_str("                return Token(TokenType.ERROR, self.input[start:self.pos], start, start_line, start_column)\n\n");

    code.push_str("        return Token(TokenType.EOF, '', self.pos, self.line, self.column)\n\n");

    code.push_str("    def tokenize(self):\n");
    code.push_str("        \"\"\"Generator that yields all tokens.\"\"\"\n");
    code.push_str("        while True:\n");
    code.push_str("            token = self.next_token()\n");
    code.push_str("            yield token\n");
    code.push_str("            if token.type == TokenType.EOF:\n");
    code.push_str("                break\n");

    // Built-in test driver
    code.push_str("\n\n");
    code.push_str(&generate_python_test_driver());

    Ok(code)
}

fn generate_c_with_conditions(
    dfas: &HashMap<String, Dfa>,
    non_bol_dfas: &HashMap<String, Dfa>,
    spec: &LexerSpec,
) -> Result<String> {
    let mut code = String::new();

    code.push_str("/**\n");
    code.push_str(" * Lexer generated by OpenLexer with start condition support.\n");
    code.push_str(" */\n\n");
    code.push_str("#include <stdio.h>\n");
    code.push_str("#include <stdlib.h>\n");
    code.push_str("#include <string.h>\n");
    code.push_str("#include <stdint.h>\n\n");

    // UTF-8 decode function for Unicode support
    code.push_str("/* UTF-8 decoding: read a codepoint from UTF-8 string */\n");
    code.push_str("static uint32_t utf8_decode(const char** ptr) {\n");
    code.push_str("    const unsigned char* s = (const unsigned char*)*ptr;\n");
    code.push_str("    uint32_t cp;\n");
    code.push_str("    int len;\n");
    code.push_str("    if (*s == 0) return 0;\n");
    code.push_str("    if (*s < 0x80) { cp = *s; len = 1; }\n");
    code.push_str("    else if ((*s & 0xE0) == 0xC0) { cp = *s & 0x1F; len = 2; }\n");
    code.push_str("    else if ((*s & 0xF0) == 0xE0) { cp = *s & 0x0F; len = 3; }\n");
    code.push_str("    else if ((*s & 0xF8) == 0xF0) { cp = *s & 0x07; len = 4; }\n");
    code.push_str("    else { *ptr = (const char*)(s + 1); return 0xFFFD; } /* invalid */\n");
    code.push_str("    for (int i = 1; i < len; i++) {\n");
    code.push_str(
        "        if ((s[i] & 0xC0) != 0x80) { *ptr = (const char*)(s + 1); return 0xFFFD; }\n",
    );
    code.push_str("        cp = (cp << 6) | (s[i] & 0x3F);\n");
    code.push_str("    }\n");
    code.push_str("    *ptr = (const char*)(s + len);\n");
    code.push_str("    return cp;\n");
    code.push_str("}\n\n");

    code.push_str("/* Get byte length of UTF-8 character */\n");
    code.push_str("static int utf8_char_len(const char* s) {\n");
    code.push_str("    unsigned char c = (unsigned char)*s;\n");
    code.push_str("    if (c == 0) return 0;\n");
    code.push_str("    if (c < 0x80) return 1;\n");
    code.push_str("    if ((c & 0xE0) == 0xC0) return 2;\n");
    code.push_str("    if ((c & 0xF0) == 0xE0) return 3;\n");
    code.push_str("    if ((c & 0xF8) == 0xF0) return 4;\n");
    code.push_str("    return 1; /* invalid, skip one byte */\n");
    code.push_str("}\n\n");

    // Token type enum
    code.push_str("typedef enum {\n");
    code.push_str("    TOKEN_EOF,\n");
    code.push_str("    TOKEN_ERROR,\n");
    let mut seen_tokens = std::collections::HashSet::new();
    seen_tokens.insert("EOF".to_string());
    seen_tokens.insert("ERROR".to_string());
    for rule in &spec.rules {
        match &rule.action {
            RuleAction::Token(name) | RuleAction::TokenAndBegin(name, _) => {
                let upper = name.to_uppercase();
                if seen_tokens.insert(upper.clone()) {
                    code.push_str(&format!("    TOKEN_{},\n", upper));
                }
            }
            _ => {}
        }
    }
    code.push_str("} TokenType;\n\n");

    // Start condition enum
    code.push_str("typedef enum {\n");
    for (idx, cond) in spec.start_conditions.keys().enumerate() {
        code.push_str(&format!("    CONDITION_{} = {},\n", cond, idx));
    }
    code.push_str("} StartCondition;\n\n");

    // Token struct
    code.push_str("typedef struct {\n");
    code.push_str("    TokenType type;\n");
    code.push_str("    const char* start;\n");
    code.push_str("    int length;\n");
    code.push_str("    int line;\n");
    code.push_str("    int column;\n");
    code.push_str("} Token;\n\n");

    // Lexer struct
    code.push_str("typedef struct {\n");
    code.push_str("    const char* input;\n");
    code.push_str("    const char* current;\n");
    code.push_str("    int line;\n");
    code.push_str("    int column;\n");
    code.push_str("    StartCondition condition;\n");
    code.push_str("    StartCondition condition_stack[32];\n");
    code.push_str("    int condition_stack_size;\n");
    code.push_str("    /* '^' anchor: true at start of input or right after a newline */\n");
    code.push_str("    int at_bol;\n");
    code.push_str("    /* yymore support */\n");
    code.push_str("    int yymore_flag;\n");
    code.push_str("    const char* yymore_start;\n");
    code.push_str("    /* REJECT support - stores backup of matches */\n");
    code.push_str("    int reject_active;\n");
    code.push_str("    int reject_rule_index;\n");
    code.push_str("    struct {\n");
    code.push_str("        int rule;\n");
    code.push_str("        const char* pos;\n");
    code.push_str("        int line;\n");
    code.push_str("        int column;\n");
    code.push_str("    } reject_stack[64];\n");
    code.push_str("    int reject_stack_size;\n");
    code.push_str("} Lexer;\n\n");

    // Global yytext and yyleng variables (set during lexing)
    code.push_str("/* Global lex variables */\n");
    code.push_str("static const char* yytext = NULL;\n");
    code.push_str("static int yyleng = 0;\n");
    code.push_str("static int yylineno = 1;\n");
    code.push_str("static Lexer* yylex_lexer = NULL;\n\n");

    // yymore macro
    code.push_str("/* yymore() - append next match to current yytext */\n");
    code.push_str(
        "#define yymore() do { if (yylex_lexer) yylex_lexer->yymore_flag = 1; } while(0)\n\n",
    );

    // yyless macro
    code.push_str("/* yyless(n) - return all but first n characters to input */\n");
    code.push_str("#define yyless(n) do { \\\n");
    code.push_str("    if (yylex_lexer && (n) >= 0 && (n) <= yyleng) { \\\n");
    code.push_str("        int put_back = yyleng - (n); \\\n");
    code.push_str("        yylex_lexer->current -= put_back; \\\n");
    code.push_str("        yyleng = (n); \\\n");
    code.push_str("    } \\\n");
    code.push_str("} while(0)\n\n");

    // REJECT macro
    code.push_str("/* REJECT - try the next matching rule */\n");
    code.push_str("#define REJECT do { \\\n");
    code.push_str("    if (yylex_lexer && yylex_lexer->reject_stack_size > 0) { \\\n");
    code.push_str("        yylex_lexer->reject_active = 1; \\\n");
    code.push_str("    } \\\n");
    code.push_str("} while(0)\n\n");

    // input() function
    code.push_str("/* input() - read next character from input */\n");
    code.push_str("static int input(void) {\n");
    code.push_str("    if (yylex_lexer && *yylex_lexer->current != '\\0') {\n");
    code.push_str("        char c = *yylex_lexer->current++;\n");
    code.push_str("        if (c == '\\n') {\n");
    code.push_str("            yylex_lexer->line++;\n");
    code.push_str("            yylex_lexer->column = 1;\n");
    code.push_str("        } else {\n");
    code.push_str("            yylex_lexer->column++;\n");
    code.push_str("        }\n");
    code.push_str("        return (unsigned char)c;\n");
    code.push_str("    }\n");
    code.push_str("    return EOF;\n");
    code.push_str("}\n\n");

    // unput() function
    code.push_str("/* unput(c) - push character back to input */\n");
    code.push_str("static void unput(int c) {\n");
    code.push_str("    if (yylex_lexer && yylex_lexer->current > yylex_lexer->input) {\n");
    code.push_str("        yylex_lexer->current--;\n");
    code.push_str("        if (c == '\\n') {\n");
    code.push_str("            yylex_lexer->line--;\n");
    code.push_str("            /* column tracking for unput is approximate */\n");
    code.push_str("        } else {\n");
    code.push_str("            yylex_lexer->column--;\n");
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // Init function
    code.push_str("void lexer_init(Lexer* lexer, const char* input) {\n");
    code.push_str("    lexer->input = input;\n");
    code.push_str("    lexer->current = input;\n");
    code.push_str("    lexer->line = 1;\n");
    code.push_str("    lexer->column = 1;\n");
    code.push_str("    lexer->condition = CONDITION_INITIAL;\n");
    code.push_str("    lexer->condition_stack_size = 0;\n");
    code.push_str("    lexer->at_bol = 1;\n");
    code.push_str("    lexer->yymore_flag = 0;\n");
    code.push_str("    lexer->yymore_start = input;\n");
    code.push_str("    lexer->reject_active = 0;\n");
    code.push_str("    lexer->reject_stack_size = 0;\n");
    code.push_str("    yylex_lexer = lexer;\n");
    code.push_str("}\n\n");

    // Heap-allocating constructor/destructor: lets other translation units
    // (e.g. a generated parser's yylex() adapter) create and drive a Lexer
    // through an opaque `Lexer*` without knowing this struct's field layout.
    code.push_str("Lexer* lexer_create(const char* input) {\n");
    code.push_str("    Lexer* lexer = (Lexer*)malloc(sizeof(Lexer));\n");
    code.push_str("    if (lexer) lexer_init(lexer, input);\n");
    code.push_str("    return lexer;\n");
    code.push_str("}\n\n");
    code.push_str("void lexer_destroy(Lexer* lexer) {\n");
    code.push_str("    free(lexer);\n");
    code.push_str("}\n\n");

    // Begin function
    code.push_str("void lexer_begin(Lexer* lexer, StartCondition condition) {\n");
    code.push_str("    lexer->condition = condition;\n");
    code.push_str("}\n\n");

    // Push/pop state functions
    code.push_str("void lexer_push_state(Lexer* lexer, StartCondition condition) {\n");
    code.push_str("    if (lexer->condition_stack_size < 32) {\n");
    code.push_str(
        "        lexer->condition_stack[lexer->condition_stack_size++] = lexer->condition;\n",
    );
    code.push_str("    }\n");
    code.push_str("    lexer->condition = condition;\n");
    code.push_str("}\n\n");

    code.push_str("void lexer_pop_state(Lexer* lexer) {\n");
    code.push_str("    if (lexer->condition_stack_size > 0) {\n");
    code.push_str(
        "        lexer->condition = lexer->condition_stack[--lexer->condition_stack_size];\n",
    );
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // Emits transition_<cond><suffix>/accepting_<cond><suffix> functions for each
    // (condition, dfa) pair. Used once for the normal tables and once more (with
    // suffix "_nobol") for conditions that have a `^`-anchored rule.
    fn emit_condition_fns(code: &mut String, dfas: &HashMap<String, Dfa>, suffix: &str) {
        for (cond, dfa) in dfas {
            code.push_str(&format!(
                "static int transition_{cond}{suffix}(int state, uint32_t cp) {{\n"
            ));
            code.push_str("    switch (state) {\n");
            for state in &dfa.states {
                code.push_str(&format!("        case {}:\n", state.id));

                // Use range_transitions if available, fall back to char transitions
                if !state.range_transitions.is_empty() {
                    for &(start_cp, end_cp, target) in &state.range_transitions {
                        if start_cp == end_cp {
                            code.push_str(&format!(
                                "            if (cp == 0x{:X}) return {};\n",
                                start_cp, target
                            ));
                        } else {
                            code.push_str(&format!(
                                "            if (cp >= 0x{:X} && cp <= 0x{:X}) return {};\n",
                                start_cp, end_cp, target
                            ));
                        }
                    }
                    code.push_str("            return -1;\n");
                } else if !state.transitions.is_empty() {
                    // Fallback to legacy char-based switch
                    code.push_str("            switch (cp) {\n");
                    for (ch, target) in &state.transitions {
                        let ch_repr = escape_c_char(*ch);
                        code.push_str(&format!(
                            "                case {}: return {};\n",
                            ch_repr, target
                        ));
                    }
                    code.push_str("                default: return -1;\n");
                    code.push_str("            }\n");
                } else {
                    code.push_str("            return -1;\n");
                }
            }
            code.push_str("        default: return -1;\n");
            code.push_str("    }\n");
            code.push_str("}\n\n");
        }

        for (cond, dfa) in dfas {
            code.push_str(&format!(
                "static int accepting_{cond}{suffix}(int state) {{\n"
            ));
            code.push_str("    switch (state) {\n");
            for state in &dfa.states {
                if state.is_accepting {
                    if let Some(rule_idx) = state.rule_index {
                        code.push_str(&format!(
                            "        case {}: return {};\n",
                            state.id, rule_idx
                        ));
                    }
                }
            }
            code.push_str("        default: return -1;\n");
            code.push_str("    }\n");
            code.push_str("}\n\n");
        }
    }

    emit_condition_fns(&mut code, dfas, "");
    emit_condition_fns(&mut code, non_bol_dfas, "_nobol");

    // Generate dispatch transition function. `at_bol` selects the not-at-bol
    // table for a condition that has one (i.e. has a `^`-anchored rule);
    // conditions without any `^` rule ignore at_bol and always use the same table.
    code.push_str("static int transition(StartCondition cond, int state, uint32_t cp, int at_bol) {\n");
    code.push_str("    switch (cond) {\n");
    for cond in dfas.keys() {
        if non_bol_dfas.contains_key(cond) {
            code.push_str(&format!(
                "        case CONDITION_{cond}: return at_bol ? transition_{cond}(state, cp) : transition_{cond}_nobol(state, cp);\n"
            ));
        } else {
            code.push_str(&format!(
                "        case CONDITION_{cond}: return transition_{cond}(state, cp);\n"
            ));
        }
    }
    code.push_str("        default: return -1;\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // Generate dispatch accepting function
    code.push_str("static int accepting(StartCondition cond, int state, int at_bol) {\n");
    code.push_str("    switch (cond) {\n");
    for cond in dfas.keys() {
        if non_bol_dfas.contains_key(cond) {
            code.push_str(&format!(
                "        case CONDITION_{cond}: return at_bol ? accepting_{cond}(state) : accepting_{cond}_nobol(state);\n"
            ));
        } else {
            code.push_str(&format!(
                "        case CONDITION_{cond}: return accepting_{cond}(state);\n"
            ));
        }
    }
    code.push_str("        default: return -1;\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // Rule action function - returns (token_type, new_condition or -1)
    code.push_str("typedef struct { TokenType token; int new_condition; } RuleAction;\n\n");
        code.push_str("static RuleAction get_rule_action(int rule_index) {\n");
    code.push_str("    switch (rule_index) {\n");
    for (idx, rule) in spec.rules.iter().enumerate() {
        let (token_str, cond_str) = match &rule.action {
            RuleAction::Token(name) => (format!("TOKEN_{}", name.to_uppercase()), "-1".to_string()),
            RuleAction::TokenAndBegin(name, state) => (
                format!("TOKEN_{}", name.to_uppercase()),
                format!("CONDITION_{}", state),
            ),
            RuleAction::Skip => {
                ("TOKEN_EOF".to_string(), "-1".to_string()) // TOKEN_EOF with -1 means skip
            }
            RuleAction::Begin(state) => ("TOKEN_EOF".to_string(), format!("CONDITION_{}", state)),
            RuleAction::Error => ("TOKEN_ERROR".to_string(), "-1".to_string()),
            RuleAction::Code(_) => {
                // Code rules return special marker - handled by inline action code
                ("TOKEN_EOF".to_string(), "-2".to_string())
            }
        };
        code.push_str(&format!(
            "        case {}: return (RuleAction){{ {}, {} }};\n",
            idx, token_str, cond_str
        ));
    }
    code.push_str("        default: return (RuleAction){ TOKEN_ERROR, -1 };\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // Token name helper
    code.push_str("const char* lexer_token_name(TokenType type) {\n");
    code.push_str("    switch (type) {\n");
    code.push_str("        case TOKEN_EOF: return \"EOF\";\n");
    code.push_str("        case TOKEN_ERROR: return \"ERROR\";\n");
    let mut displayed_tokens = std::collections::HashSet::new();
    displayed_tokens.insert("EOF".to_string());
    displayed_tokens.insert("ERROR".to_string());
    for rule in &spec.rules {
        if let RuleAction::Token(name) = &rule.action {
            let upper = name.to_uppercase();
            if displayed_tokens.insert(upper.clone()) {
                code.push_str(&format!("        case TOKEN_{}: return \"{}\";\n", upper, upper));
            }
        }
    }
    code.push_str("        default: return \"UNKNOWN\";\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // Is-skip helper
    code.push_str("static int is_skip_rule(int rule_index) {\n");
    code.push_str("    switch (rule_index) {\n");
    for (idx, rule) in spec.rules.iter().enumerate() {
        if matches!(&rule.action, RuleAction::Skip | RuleAction::Begin(_)) {
            code.push_str(&format!("        case {}: return 1;\n", idx));
        }
    }
    code.push_str("        default: return 0;\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // Main next_token function
    code.push_str("Token lexer_next(Lexer* lexer) {\n");
    code.push_str("    Token token;\n");
    code.push_str("    token.type = TOKEN_EOF;\n");
    code.push_str("    token.start = lexer->current;\n");
    code.push_str("    token.length = 0;\n");
    code.push_str("    token.line = lexer->line;\n");
    code.push_str("    token.column = lexer->column;\n\n");

    code.push_str(
        "    /* Handle yymore: if flag is set, start from yymore_start instead of current */\n",
    );
    code.push_str("    const char* yymore_preserved_start = lexer->yymore_start;\n\n");

    code.push_str("    while (*lexer->current != '\\0') {\n");
    code.push_str("        const char* start;\n");
    code.push_str("        if (lexer->yymore_flag) {\n");
    code.push_str("            start = yymore_preserved_start;\n");
    code.push_str("            lexer->yymore_flag = 0;\n");
    code.push_str("        } else {\n");
    code.push_str("            start = lexer->current;\n");
    code.push_str("            lexer->yymore_start = start;\n");
    code.push_str("        }\n");
    code.push_str("        int start_line = lexer->line;\n");
    code.push_str("        int start_column = lexer->column;\n");
    code.push_str("        int state = 0;\n");
    code.push_str("        int last_accepting_rule = -1;\n");
    code.push_str("        const char* last_accepting_pos = lexer->current;\n");
    code.push_str("        int last_accepting_line = lexer->line;\n");
    code.push_str("        int last_accepting_column = lexer->column;\n\n");

    code.push_str("        /* Clear reject stack for this match attempt */\n");
    code.push_str("        lexer->reject_stack_size = 0;\n");
    code.push_str("        lexer->reject_active = 0;\n\n");

    code.push_str("        while (*lexer->current != '\\0') {\n");
    code.push_str("            const char* before = lexer->current;\n");
    code.push_str("            uint32_t cp = utf8_decode(&lexer->current);\n");
    code.push_str("            if (cp == 0) break;\n");
    code.push_str("            int next_state = transition(lexer->condition, state, cp, lexer->at_bol);\n");
    code.push_str("            if (next_state < 0) { lexer->current = before; break; }\n");
    code.push_str("            state = next_state;\n");
    code.push_str("            if (cp == '\\n') {\n");
    code.push_str("                lexer->line++;\n");
    code.push_str("                lexer->column = 1;\n");
    code.push_str("            } else {\n");
    code.push_str("                lexer->column++;\n");
    code.push_str("            }\n");
    code.push_str("            int rule = accepting(lexer->condition, state, lexer->at_bol);\n");
    code.push_str("            if (rule >= 0) {\n");
    code.push_str("                /* Store in reject stack for potential REJECT */\n");
    code.push_str("                if (lexer->reject_stack_size < 64) {\n");
    code.push_str(
        "                    lexer->reject_stack[lexer->reject_stack_size].rule = rule;\n",
    );
    code.push_str(
        "                    lexer->reject_stack[lexer->reject_stack_size].pos = lexer->current;\n",
    );
    code.push_str(
        "                    lexer->reject_stack[lexer->reject_stack_size].line = lexer->line;\n",
    );
    code.push_str("                    lexer->reject_stack[lexer->reject_stack_size].column = lexer->column;\n");
    code.push_str("                    lexer->reject_stack_size++;\n");
    code.push_str("                }\n");
    code.push_str("                last_accepting_rule = rule;\n");
    code.push_str("                last_accepting_pos = lexer->current;\n");
    code.push_str("                last_accepting_line = lexer->line;\n");
    code.push_str("                last_accepting_column = lexer->column;\n");
    code.push_str("            }\n");
    code.push_str("        }\n\n");

    code.push_str("try_rule:\n");
    code.push_str("        if (last_accepting_rule >= 0) {\n");
    code.push_str("            lexer->current = last_accepting_pos;\n");
    code.push_str("            lexer->line = last_accepting_line;\n");
    code.push_str("            lexer->column = last_accepting_column;\n\n");
    code.push_str("            /* Set global yytext and yyleng */\n");
    code.push_str("            yytext = (char*)start;\n");
    code.push_str("            yyleng = (int)(last_accepting_pos - start);\n");
    code.push_str("            yylineno = start_line;\n");
    code.push_str("            lexer->at_bol = (yyleng > 0) && (start[yyleng - 1] == '\\n');\n\n");
    code.push_str("            RuleAction action = get_rule_action(last_accepting_rule);\n");
    code.push_str("            if (action.new_condition >= 0) {\n");
    code.push_str("                lexer->condition = (StartCondition)action.new_condition;\n");
    code.push_str("            }\n\n");
    code.push_str("            /* Check for REJECT after potential user code execution */\n");
    code.push_str("            if (lexer->reject_active) {\n");
    code.push_str("                lexer->reject_active = 0;\n");
    code.push_str("                /* Try next match from reject stack */\n");
    code.push_str("                if (lexer->reject_stack_size > 1) {\n");
    code.push_str("                    lexer->reject_stack_size--;\n");
    code.push_str("                    int idx = lexer->reject_stack_size - 1;\n");
    code.push_str("                    last_accepting_rule = lexer->reject_stack[idx].rule;\n");
    code.push_str("                    last_accepting_pos = lexer->reject_stack[idx].pos;\n");
    code.push_str("                    last_accepting_line = lexer->reject_stack[idx].line;\n");
    code.push_str("                    last_accepting_column = lexer->reject_stack[idx].column;\n");
    code.push_str("                    goto try_rule;\n");
    code.push_str("                }\n");
    code.push_str("            }\n\n");
    code.push_str("            if (is_skip_rule(last_accepting_rule)) {\n");
    code.push_str("                continue;\n");
    code.push_str("            }\n");
    code.push_str("            token.type = action.token;\n");
    code.push_str("            token.start = start;\n");
    code.push_str("            token.length = (int)(last_accepting_pos - start);\n");
    code.push_str("            token.line = start_line;\n");
    code.push_str("            token.column = start_column;\n");
    code.push_str("            return token;\n");
    code.push_str("        } else {\n");
    code.push_str("            /* No match - error token for one UTF-8 character */\n");
    code.push_str("            int char_len = utf8_char_len(start);\n");
    code.push_str("            if (char_len == 0) char_len = 1;\n");
    code.push_str("            lexer->current = start + char_len;\n");
    code.push_str("            lexer->at_bol = (*start == '\\n');\n");
    code.push_str("            if (*start == '\\n') {\n");
    code.push_str("                lexer->line++;\n");
    code.push_str("                lexer->column = 1;\n");
    code.push_str("            } else {\n");
    code.push_str("                lexer->column++;\n");
    code.push_str("            }\n");
    code.push_str("            yytext = (char*)start;\n");
    code.push_str("            yyleng = char_len;\n");
    code.push_str("            token.type = TOKEN_ERROR;\n");
    code.push_str("            token.start = start;\n");
    code.push_str("            token.length = char_len;\n");
    code.push_str("            token.line = start_line;\n");
    code.push_str("            token.column = start_column;\n");
    code.push_str("            return token;\n");
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    code.push_str("    yytext = (char*)lexer->current;\n");
    code.push_str("    yyleng = 0;\n");
    code.push_str("    token.type = TOKEN_EOF;\n");
    code.push_str("    token.start = lexer->current;\n");
    code.push_str("    token.length = 0;\n");
    code.push_str("    token.line = lexer->line;\n");
    code.push_str("    token.column = lexer->column;\n");
    code.push_str("    return token;\n");
    code.push_str("}\n\n");

    // Output-param variant of lexer_next(): avoids handing the Token struct
    // itself across a translation-unit boundary (e.g. to a parser's yylex()
    // adapter), since Token's exact layout can differ between this codegen
    // path and generate_c_full's.
    code.push_str("int lexer_next_token(Lexer* lexer, const char** out_text, int* out_len) {\n");
    code.push_str("    Token tok = lexer_next(lexer);\n");
    code.push_str("    *out_text = tok.start;\n");
    code.push_str("    *out_len = tok.length;\n");
    code.push_str("    return (int)tok.type;\n");
    code.push_str("}\n");

    // Built-in test driver
    code.push_str("\n");
    code.push_str(&generate_c_test_driver());

    Ok(code)
}

fn generate_java_with_conditions(
    dfas: &HashMap<String, Dfa>,
    non_bol_dfas: &HashMap<String, Dfa>,
    spec: &LexerSpec,
) -> Result<String> {
    let mut code = String::new();

    code.push_str("/**\n");
    code.push_str(" * Lexer generated by OpenLexer with start condition support.\n");
    code.push_str(" */\n\n");
    code.push_str("import java.util.ArrayList;\n");
    code.push_str("import java.util.List;\n\n");

    // Token type enum
    code.push_str("enum TokenType {\n");
    code.push_str("    TOKEN_EOF,\n");
    code.push_str("    TOKEN_ERROR,\n");
    let mut seen_tokens = std::collections::HashSet::new();
    seen_tokens.insert("EOF".to_string());
    seen_tokens.insert("ERROR".to_string());
    for rule in &spec.rules {
        match &rule.action {
            RuleAction::Token(name) | RuleAction::TokenAndBegin(name, _) => {
                let upper = name.to_uppercase();
                if seen_tokens.insert(upper.clone()) {
                    code.push_str(&format!("    TOKEN_{},\n", upper));
                }
            }
            _ => {}
        }
    }
    code.push_str("}\n\n");

    // Start condition enum
    code.push_str("enum StartCondition {\n");
    for (idx, cond) in spec.start_conditions.keys().enumerate() {
        if idx > 0 {
            code.push_str(",\n");
        }
        code.push_str(&format!("    {}", cond));
    }
    code.push_str("\n}\n\n");

    // Token class
    code.push_str("class Token {\n");
    code.push_str("    public final TokenType type;\n");
    code.push_str("    public final String text;\n");
    code.push_str("    public final int pos;\n");
    code.push_str("    public final int line;\n");
    code.push_str("    public final int column;\n\n");
    code.push_str(
        "    public Token(TokenType type, String text, int pos, int line, int column) {\n",
    );
    code.push_str("        this.type = type;\n");
    code.push_str("        this.text = text;\n");
    code.push_str("        this.pos = pos;\n");
    code.push_str("        this.line = line;\n");
    code.push_str("        this.column = column;\n");
    code.push_str("    }\n\n");
    code.push_str("    @Override\n");
    code.push_str("    public String toString() {\n");
    code.push_str(
        "        return String.format(\"Token(%s, \\\"%s\\\", pos=%d, line=%d, col=%d)\",\n",
    );
    code.push_str("                type, text.replace(\"\\n\", \"\\\\n\"), pos, line, column);\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // Lexer class
    code.push_str("class Lexer {\n");
    code.push_str("    private final String input;\n");
    code.push_str("    private int pos = 0;\n");
    code.push_str("    private int line = 1;\n");
    code.push_str("    private int column = 1;\n");
    code.push_str("    private StartCondition condition = StartCondition.INITIAL;\n");
    code.push_str("    private final List<StartCondition> conditionStack = new ArrayList<>();\n");
    code.push_str(
        "    private boolean atBol = true;  // '^' anchor: start of input or after a newline\n\n",
    );

    code.push_str("    public Lexer(String input) {\n");
    code.push_str("        this.input = input;\n");
    code.push_str("    }\n\n");

    code.push_str("    public void begin(StartCondition condition) {\n");
    code.push_str("        this.condition = condition;\n");
    code.push_str("    }\n\n");

    code.push_str("    public void pushState(StartCondition condition) {\n");
    code.push_str("        conditionStack.add(this.condition);\n");
    code.push_str("        this.condition = condition;\n");
    code.push_str("    }\n\n");

    code.push_str("    public void popState() {\n");
    code.push_str("        if (!conditionStack.isEmpty()) {\n");
    code.push_str(
        "            this.condition = conditionStack.remove(conditionStack.size() - 1);\n",
    );
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    // Generate transition method. `atBol` selects the not-at-bol per-condition
    // method for a condition that has one (has a `^`-anchored rule); a
    // condition without any `^` rule ignores atBol and always uses the same method.
    code.push_str("    private int transition(int state, char c) {\n");
    code.push_str("        switch (condition) {\n");
    for cond in dfas.keys() {
        if non_bol_dfas.contains_key(cond) {
            code.push_str(&format!(
                "            case {cond}: return atBol ? transition_{cond}(state, c) : transition_{cond}_nobol(state, c);\n"
            ));
        } else {
            code.push_str(&format!(
                "            case {cond}: return transition_{cond}(state, c);\n"
            ));
        }
    }
    code.push_str("            default: return -1;\n");
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    // Generate accepting method
    code.push_str("    private int accepting(int state) {\n");
    code.push_str("        switch (condition) {\n");
    for cond in dfas.keys() {
        if non_bol_dfas.contains_key(cond) {
            code.push_str(&format!(
                "            case {cond}: return atBol ? accepting_{cond}(state) : accepting_{cond}_nobol(state);\n"
            ));
        } else {
            code.push_str(&format!(
                "            case {cond}: return accepting_{cond}(state);\n"
            ));
        }
    }
    code.push_str("            default: return -1;\n");
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    // Emits transition_<cond><suffix>/accepting_<cond><suffix> methods for each
    // (condition, dfa) pair - once for the normal tables, once more (suffix
    // "_nobol") for conditions that have a `^`-anchored rule.
    fn emit_condition_methods(code: &mut String, dfas: &HashMap<String, Dfa>, suffix: &str) {
        for (cond, dfa) in dfas {
            code.push_str(&format!(
                "    private int transition_{cond}{suffix}(int state, char c) {{\n"
            ));
            code.push_str("        switch (state) {\n");
            for state in &dfa.states {
                code.push_str(&format!("            case {}:\n", state.id));

                // Use range_transitions if available (same DFA data C/Python read),
                // falling back to the plain char list. Without this, states built
                // from a char class like [a-zA-Z] - which only populates
                // range_transitions - would emit no transitions at all here.
                if !state.range_transitions.is_empty() {
                    for &(start_cp, end_cp, target) in &state.range_transitions {
                        let start_ch = char::from_u32(start_cp).unwrap_or('\0');
                        if start_cp == end_cp {
                            code.push_str(&format!(
                                "                if (c == {}) return {};\n",
                                escape_java_char(start_ch),
                                target
                            ));
                        } else {
                            let end_ch = char::from_u32(end_cp).unwrap_or('\0');
                            code.push_str(&format!(
                                "                if (c >= {} && c <= {}) return {};\n",
                                escape_java_char(start_ch),
                                escape_java_char(end_ch),
                                target
                            ));
                        }
                    }
                    code.push_str("                return -1;\n");
                } else if !state.transitions.is_empty() {
                    code.push_str("                switch (c) {\n");
                    for (ch, target) in &state.transitions {
                        let ch_repr = escape_java_char(*ch);
                        code.push_str(&format!(
                            "                    case {}: return {};\n",
                            ch_repr, target
                        ));
                    }
                    code.push_str("                    default: return -1;\n");
                    code.push_str("                }\n");
                } else {
                    code.push_str("                return -1;\n");
                }
            }
            code.push_str("            default: return -1;\n");
            code.push_str("        }\n");
            code.push_str("    }\n\n");
        }

        for (cond, dfa) in dfas {
            code.push_str(&format!(
                "    private int accepting_{cond}{suffix}(int state) {{\n"
            ));
            code.push_str("        switch (state) {\n");
            for state in &dfa.states {
                if state.is_accepting {
                    if let Some(rule_idx) = state.rule_index {
                        code.push_str(&format!(
                            "            case {}: return {};\n",
                            state.id, rule_idx
                        ));
                    }
                }
            }
            code.push_str("            default: return -1;\n");
            code.push_str("        }\n");
            code.push_str("    }\n\n");
        }
    }

    emit_condition_methods(&mut code, dfas, "");
    emit_condition_methods(&mut code, non_bol_dfas, "_nobol");

    // Get token type from rule index
    code.push_str("    private TokenType getTokenType(int ruleIndex) {\n");
    code.push_str("        switch (ruleIndex) {\n");
    for (idx, rule) in spec.rules.iter().enumerate() {
        let token_str = match &rule.action {
            RuleAction::Token(name) | RuleAction::TokenAndBegin(name, _) => {
                format!("TokenType.TOKEN_{}", name.to_uppercase())
            }
            RuleAction::Skip | RuleAction::Begin(_) => "null".to_string(),
            RuleAction::Error => "TokenType.TOKEN_ERROR".to_string(),
            RuleAction::Code(_) => "null".to_string(), // Code actions handled separately
        };
        code.push_str(&format!(
            "            case {}: return {};\n",
            idx, token_str
        ));
    }
    code.push_str("            default: return TokenType.TOKEN_ERROR;\n");
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    // Handle state change from rule
    code.push_str("    private void handleStateChange(int ruleIndex) {\n");
    code.push_str("        switch (ruleIndex) {\n");
    for (idx, rule) in spec.rules.iter().enumerate() {
        match &rule.action {
            RuleAction::TokenAndBegin(_, state) | RuleAction::Begin(state) => {
                code.push_str(&format!(
                    "            case {}: condition = StartCondition.{}; break;\n",
                    idx, state
                ));
            }
            _ => {}
        }
    }
    code.push_str("            default: break;\n");
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    // Check if rule is skip
    code.push_str("    private boolean isSkipRule(int ruleIndex) {\n");
    code.push_str("        switch (ruleIndex) {\n");
    for (idx, rule) in spec.rules.iter().enumerate() {
        if matches!(&rule.action, RuleAction::Skip | RuleAction::Begin(_)) {
            code.push_str(&format!("            case {}: return true;\n", idx));
        }
    }
    code.push_str("            default: return false;\n");
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    // nextToken method
    code.push_str("    public Token nextToken() {\n");
    code.push_str("        while (pos < input.length()) {\n");
    code.push_str("            int start = pos;\n");
    code.push_str("            int startLine = line;\n");
    code.push_str("            int startColumn = column;\n");
    code.push_str("            int state = 0;\n");
    code.push_str("            int lastAcceptingRule = -1;\n");
    code.push_str("            int lastAcceptingPos = start;\n\n");

    code.push_str("            while (pos < input.length()) {\n");
    code.push_str("                char c = input.charAt(pos);\n");
    code.push_str("                int nextState = transition(state, c);\n");
    code.push_str("                if (nextState < 0) break;\n");
    code.push_str("                state = nextState;\n");
    code.push_str("                pos++;\n");
    code.push_str("                if (c == '\\n') {\n");
    code.push_str("                    line++;\n");
    code.push_str("                    column = 1;\n");
    code.push_str("                } else {\n");
    code.push_str("                    column++;\n");
    code.push_str("                }\n");
    code.push_str("                int rule = accepting(state);\n");
    code.push_str("                if (rule >= 0) {\n");
    code.push_str("                    lastAcceptingRule = rule;\n");
    code.push_str("                    lastAcceptingPos = pos;\n");
    code.push_str("                }\n");
    code.push_str("            }\n\n");

    code.push_str("            if (lastAcceptingRule >= 0) {\n");
    code.push_str("                pos = lastAcceptingPos;\n");
    code.push_str("                handleStateChange(lastAcceptingRule);\n");
    code.push_str("                String text = input.substring(start, lastAcceptingPos);\n");
    code.push_str("                atBol = !text.isEmpty() && text.charAt(text.length() - 1) == '\\n';\n");
    code.push_str("                if (isSkipRule(lastAcceptingRule)) {\n");
    code.push_str("                    continue;\n");
    code.push_str("                }\n");
    code.push_str("                TokenType type = getTokenType(lastAcceptingRule);\n");
    code.push_str("                return new Token(type, text, start, startLine, startColumn);\n");
    code.push_str("            } else {\n");
    code.push_str("                pos = start + 1;\n");
    code.push_str("                atBol = input.charAt(start) == '\\n';\n");
    code.push_str("                if (input.charAt(start) == '\\n') {\n");
    code.push_str("                    line++;\n");
    code.push_str("                    column = 1;\n");
    code.push_str("                } else {\n");
    code.push_str("                    column++;\n");
    code.push_str("                }\n");
    code.push_str("                return new Token(TokenType.TOKEN_ERROR, input.substring(start, pos), start, startLine, startColumn);\n");
    code.push_str("            }\n");
    code.push_str("        }\n");
    code.push_str("        return new Token(TokenType.TOKEN_EOF, \"\", pos, line, column);\n");
    code.push_str("    }\n\n");

    // Tokenize method - returns list
    code.push_str("    public List<Token> tokenize() {\n");
    code.push_str("        List<Token> tokens = new ArrayList<>();\n");
    code.push_str("        while (true) {\n");
    code.push_str("            Token token = nextToken();\n");
    code.push_str("            tokens.add(token);\n");
    code.push_str("            if (token.type == TokenType.TOKEN_EOF) break;\n");
    code.push_str("        }\n");
    code.push_str("        return tokens;\n");
    code.push_str("    }\n");

    // Built-in test driver
    code.push_str("\n");
    code.push_str(&generate_java_test_driver());

    code.push_str("}\n");

    Ok(code)
}

// =============================================================================
// Full multi-token lexer generation (legacy - no start conditions)
// =============================================================================

fn generate_c_full(
    dfa: &Dfa,
    spec: &LexerSpec,
    trailing_dfas: &TrailingContextDfas,
) -> Result<String> {
    let mut code = String::new();

    code.push_str("#include <stdio.h>\n");
    code.push_str("#include <stdlib.h>\n");
    code.push_str("#include <string.h>\n");
    code.push_str("#include <stdint.h>\n\n");

    // UTF-8 decoding functions for Unicode support
    code.push_str("/* UTF-8 utility functions */\n");
    code.push_str("static int utf8_char_len(unsigned char c) {\n");
    code.push_str("    if ((c & 0x80) == 0) return 1;\n");
    code.push_str("    if ((c & 0xE0) == 0xC0) return 2;\n");
    code.push_str("    if ((c & 0xF0) == 0xE0) return 3;\n");
    code.push_str("    if ((c & 0xF8) == 0xF0) return 4;\n");
    code.push_str("    return 1;\n");
    code.push_str("}\n\n");
    code.push_str("static uint32_t utf8_decode(const char** s) {\n");
    code.push_str("    const unsigned char* p = (const unsigned char*)*s;\n");
    code.push_str("    uint32_t cp;\n");
    code.push_str("    int len = utf8_char_len(*p);\n");
    code.push_str("    switch (len) {\n");
    code.push_str("        case 1: cp = p[0]; break;\n");
    code.push_str("        case 2: cp = ((p[0] & 0x1F) << 6) | (p[1] & 0x3F); break;\n");
    code.push_str("        case 3: cp = ((p[0] & 0x0F) << 12) | ((p[1] & 0x3F) << 6) | (p[2] & 0x3F); break;\n");
    code.push_str("        case 4: cp = ((p[0] & 0x07) << 18) | ((p[1] & 0x3F) << 12) | ((p[2] & 0x3F) << 6) | (p[3] & 0x3F); break;\n");
    code.push_str("        default: cp = 0;\n");
    code.push_str("    }\n");
    code.push_str("    *s += len;\n");
    code.push_str("    return cp;\n");
    code.push_str("}\n\n");

    // Generate token type enum
    code.push_str("typedef enum {\n");
    code.push_str("    TOKEN_EOF,\n");
    code.push_str("    TOKEN_ERROR,\n");

    // Track seen tokens to avoid duplicates (EOF and ERROR are already defined)
    let mut seen_tokens = std::collections::HashSet::new();
    seen_tokens.insert("EOF".to_string());
    seen_tokens.insert("ERROR".to_string());

    for rule in &spec.rules {
        if let RuleAction::Token(name) = &rule.action {
            let upper_name = name.to_uppercase();
            if seen_tokens.insert(upper_name.clone()) {
                code.push_str(&format!("    TOKEN_{},\n", upper_name));
            }
        }
    }
    code.push_str("} TokenType;\n\n");

    // Generate token struct
    code.push_str("typedef struct {\n");
    code.push_str("    TokenType type;\n");
    code.push_str("    const char* start;\n");
    code.push_str("    int length;\n");
    code.push_str("} Token;\n\n");

    // Generate lexer state
    code.push_str("typedef struct {\n");
    code.push_str("    const char* input;\n");
    code.push_str("    const char* current;\n");
    code.push_str("} Lexer;\n\n");

    // Generate init function
    code.push_str("void lexer_init(Lexer* lexer, const char* input) {\n");
    code.push_str("    lexer->input = input;\n");
    code.push_str("    lexer->current = input;\n");
    code.push_str("}\n\n");

    // Heap-allocating constructor/destructor: lets other translation units
    // (e.g. a generated parser's yylex() adapter) create and drive a Lexer
    // through an opaque `Lexer*` without knowing this struct's field layout.
    code.push_str("Lexer* lexer_create(const char* input) {\n");
    code.push_str("    Lexer* lexer = (Lexer*)malloc(sizeof(Lexer));\n");
    code.push_str("    if (lexer) lexer_init(lexer, input);\n");
    code.push_str("    return lexer;\n");
    code.push_str("}\n\n");
    code.push_str("void lexer_destroy(Lexer* lexer) {\n");
    code.push_str("    free(lexer);\n");
    code.push_str("}\n\n");

    // Generate rule index to token mapping
    code.push_str("static TokenType rule_to_token(int rule_index) {\n");
    code.push_str("    switch (rule_index) {\n");
    for (idx, rule) in spec.rules.iter().enumerate() {
        match &rule.action {
            RuleAction::Token(name) => {
                code.push_str(&format!(
                    "        case {}: return TOKEN_{};\n",
                    idx,
                    name.to_uppercase()
                ));
            }
            RuleAction::TokenAndBegin(name, _) => {
                code.push_str(&format!(
                    "        case {}: return TOKEN_{};\n",
                    idx,
                    name.to_uppercase()
                ));
            }
            RuleAction::Skip | RuleAction::Begin(_) => {
                code.push_str(&format!(
                    "        case {}: return TOKEN_EOF; // Skip or state change\n",
                    idx
                ));
            }
            RuleAction::Error => {
                code.push_str(&format!("        case {}: return TOKEN_ERROR;\n", idx));
            }
            RuleAction::Code(_) => {
                code.push_str(&format!(
                    "        case {}: return TOKEN_EOF; // Code action handled separately\n",
                    idx
                ));
            }
        }
    }
    code.push_str("        default: return TOKEN_ERROR;\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // Token name helper
    code.push_str("const char* lexer_token_name(TokenType type) {\n");
    code.push_str("    switch (type) {\n");
    code.push_str("        case TOKEN_EOF: return \"EOF\";\n");
    code.push_str("        case TOKEN_ERROR: return \"ERROR\";\n");
    let mut displayed_tokens = std::collections::HashSet::new();
    displayed_tokens.insert("EOF".to_string());
    displayed_tokens.insert("ERROR".to_string());
    for rule in &spec.rules {
        if let RuleAction::Token(name) = &rule.action {
            let upper = name.to_uppercase();
            if displayed_tokens.insert(upper.clone()) {
                code.push_str(&format!("        case TOKEN_{}: return \"{}\";\n", upper, upper));
            }
        }
    }
    code.push_str("        default: return \"UNKNOWN\";\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // Generate next_token function
    // Trailing context (r1/r2): a small standalone DFA per rule that has it,
    // used to re-scan a match and find where r1 ends. Rules without trailing
    // context have no functions/dispatch entry here.
    fn emit_tc_dfa_fns(code: &mut String, dfas: &HashMap<usize, Dfa>, prefix: &str) {
        for (rule_idx, tdfa) in dfas {
            code.push_str(&format!(
                "static int {prefix}_transition_{rule_idx}(int state, uint32_t cp) {{\n"
            ));
            code.push_str("    switch (state) {\n");
            for state in &tdfa.states {
                code.push_str(&format!("        case {}:\n", state.id));
                if !state.range_transitions.is_empty() {
                    for &(start_cp, end_cp, target) in &state.range_transitions {
                        if start_cp == end_cp {
                            code.push_str(&format!(
                                "            if (cp == 0x{:X}) return {};\n",
                                start_cp, target
                            ));
                        } else {
                            code.push_str(&format!(
                                "            if (cp >= 0x{:X} && cp <= 0x{:X}) return {};\n",
                                start_cp, end_cp, target
                            ));
                        }
                    }
                } else {
                    for (ch, target) in &state.transitions {
                        code.push_str(&format!(
                            "            if (cp == 0x{:X}) return {};\n",
                            *ch as u32, target
                        ));
                    }
                }
                code.push_str("            return -1;\n");
            }
            code.push_str("        default: return -1;\n");
            code.push_str("    }\n}\n\n");

            code.push_str(&format!(
                "static int {prefix}_accepting_{rule_idx}(int state) {{\n"
            ));
            code.push_str("    switch (state) {\n");
            for state in &tdfa.states {
                if state.is_accepting {
                    code.push_str(&format!("        case {}: return 1;\n", state.id));
                }
            }
            code.push_str("        default: return 0;\n");
            code.push_str("    }\n}\n\n");
        }
    }

    emit_tc_dfa_fns(&mut code, &trailing_dfas.r1, "tc");
    emit_tc_dfa_fns(&mut code, &trailing_dfas.r2, "tc2");

    if !trailing_dfas.is_empty() {
        code.push_str("/* True if rule_index's r2 exactly matches text[0..len) (consumes all of it and ends accepting). */\n");
        code.push_str("static int tc_r2_matches(int rule_index, const char* text, int len) {\n");
        code.push_str("    int state = 0;\n");
        code.push_str("    const char* p = text;\n");
        code.push_str("    const char* end = text + len;\n");
        code.push_str("    switch (rule_index) {\n");
        for rule_idx in trailing_dfas.r1.keys() {
            code.push_str(&format!("        case {}:\n", rule_idx));
            code.push_str("            while (p < end) {\n");
            code.push_str("                uint32_t cp = utf8_decode(&p);\n");
            code.push_str(&format!(
                "                int next = tc2_transition_{}(state, cp);\n",
                rule_idx
            ));
            code.push_str("                if (next < 0) return 0;\n");
            code.push_str("                state = next;\n");
            code.push_str("            }\n");
            code.push_str(&format!(
                "            return tc2_accepting_{}(state);\n",
                rule_idx
            ));
        }
        code.push_str("        default: return 0;\n");
        code.push_str("    }\n");
        code.push_str("}\n\n");

        // ponytail: caps at 256 candidate r1 boundaries per match; a single
        // trailing-context match with more accepting prefixes than that
        // (e.g. a huge repetitive token) silently only considers the first
        // 256 - raise MAX_TC_CANDIDATES if a real spec needs more.
        code.push_str("#define MAX_TC_CANDIDATES 256\n\n");
        code.push_str("/* Longest prefix of `start`[0..len) accepted by rule_index's r1 whose\n");
        code.push_str(" * remaining suffix is an exact match for r2 (not just the longest r1\n");
        code.push_str(" * accepts on its own, which picks the wrong boundary for e.g. r1=a-star r2=a).\n");
        code.push_str(" * Falls back to the full match (returns len) if no candidate validates -\n");
        code.push_str(" * Flex's own \"dangerous trailing context\" case. */\n");
        code.push_str("static int trailing_context_split(int rule_index, const char* start, int len) {\n");
        code.push_str("    int candidates[MAX_TC_CANDIDATES];\n");
        code.push_str("    int num_candidates = 0;\n");
        code.push_str("    int state = 0, pos = 0;\n");
        code.push_str("    const char* p = start;\n");
        code.push_str("    const char* end = start + len;\n");
        code.push_str("    switch (rule_index) {\n");
        for rule_idx in trailing_dfas.r1.keys() {
            code.push_str(&format!("        case {}:\n", rule_idx));
            code.push_str(&format!(
                "            if (tc_accepting_{}(state) && num_candidates < MAX_TC_CANDIDATES) candidates[num_candidates++] = 0;\n",
                rule_idx
            ));
            code.push_str("            while (p < end) {\n");
            code.push_str("                uint32_t cp = utf8_decode(&p);\n");
            code.push_str(&format!(
                "                int next = tc_transition_{}(state, cp);\n",
                rule_idx
            ));
            code.push_str("                if (next < 0) break;\n");
            code.push_str("                state = next;\n");
            code.push_str("                pos = (int)(p - start);\n");
            code.push_str(&format!(
                "                if (tc_accepting_{}(state) && num_candidates < MAX_TC_CANDIDATES) candidates[num_candidates++] = pos;\n",
                rule_idx
            ));
            code.push_str("            }\n");
            code.push_str("            break;\n");
        }
        code.push_str("        default: break;\n");
        code.push_str("    }\n");
        code.push_str("    /* candidates[i] == 0 is excluded even if r1/r2 both validate it: it\n");
        code.push_str("     * would produce a zero-length token and never advance lexer->current. */\n");
        code.push_str("    for (int i = num_candidates - 1; i >= 0; i--) {\n");
        code.push_str("        if (candidates[i] > 0 && tc_r2_matches(rule_index, start + candidates[i], len - candidates[i])) {\n");
        code.push_str("            return candidates[i];\n");
        code.push_str("        }\n");
        code.push_str("    }\n");
        code.push_str("    return len;\n");
        code.push_str("}\n\n");
    }

    code.push_str("Token lexer_next(Lexer* lexer) {\n");
    code.push_str("    Token token;\n");
    code.push_str("    token.type = TOKEN_EOF;\n");
    code.push_str("    token.start = lexer->current;\n");
    code.push_str("    token.length = 0;\n\n");

    code.push_str("    while (*lexer->current != '\\0') {\n");
    code.push_str("        const char* start = lexer->current;\n");
    code.push_str("        int state = 0;\n");
    code.push_str("        int last_accepting_state = -1;\n");
    code.push_str("        int last_accepting_rule = -1;\n");
    code.push_str("        const char* last_accepting_pos = start;\n\n");

    // Main DFA loop - now uses range-based transitions for Unicode support
    code.push_str("        while (*lexer->current != '\\0') {\n");
    code.push_str("            int next_state = -1;\n");
    code.push_str("            const char* before = lexer->current;\n");
    code.push_str("            uint32_t cp = utf8_decode(&lexer->current);\n");
    code.push_str("            if (cp == 0) break;\n");
    code.push_str("            switch (state) {\n");

    for state in &dfa.states {
        code.push_str(&format!("                case {}:\n", state.id));
        // Use range-based transitions if available
        if !state.range_transitions.is_empty() {
            let mut first = true;
            for &(start_cp, end_cp, target) in &state.range_transitions {
                let prefix = if first { "if" } else { "else if" };
                first = false;
                if start_cp == end_cp {
                    code.push_str(&format!(
                        "                    {} (cp == 0x{:X}) next_state = {};\n",
                        prefix, start_cp, target
                    ));
                } else {
                    code.push_str(&format!(
                        "                    {} (cp >= 0x{:X} && cp <= 0x{:X}) next_state = {};\n",
                        prefix, start_cp, end_cp, target
                    ));
                }
            }
            code.push_str("                    /* else next_state stays -1 */\n");
        } else if !state.transitions.is_empty() {
            // Fallback to char-based for legacy DFAs
            let mut first = true;
            for (ch, target) in &state.transitions {
                let prefix = if first { "if" } else { "else if" };
                first = false;
                let cp_val = *ch as u32;
                code.push_str(&format!(
                    "                    {} (cp == 0x{:X}) next_state = {};\n",
                    prefix, cp_val, target
                ));
            }
            code.push_str("                    /* else next_state stays -1 */\n");
        } else {
            code.push_str("                    next_state = -1;\n");
        }
        code.push_str("                    break;\n");
    }

    code.push_str("                default: next_state = -1; break;\n");
    code.push_str("            }\n\n");

    code.push_str("            if (next_state == -1) { lexer->current = before; break; }\n");
    code.push_str("            state = next_state;\n\n");

    // Check if current state is accepting
    code.push_str("            switch (state) {\n");
    for state in &dfa.states {
        if state.is_accepting {
            if let Some(rule_idx) = state.rule_index {
                code.push_str(&format!("                case {}: last_accepting_state = {}; last_accepting_rule = {}; last_accepting_pos = lexer->current; break;\n", 
                    state.id, state.id, rule_idx));
            }
        }
    }
    code.push_str("                default: break;\n");
    code.push_str("            }\n");
    code.push_str("        }\n\n");

    // Handle match result
    code.push_str("        if (last_accepting_rule >= 0) {\n");
    code.push_str("            int match_len = (int)(last_accepting_pos - start);\n");
    if !trailing_dfas.is_empty() {
        code.push_str("            {\n");
        code.push_str("                int has_tc = 0;\n");
        code.push_str("                switch (last_accepting_rule) {\n");
        for rule_idx in trailing_dfas.r1.keys() {
            code.push_str(&format!("                    case {}: has_tc = 1; break;\n", rule_idx));
        }
        code.push_str("                    default: break;\n");
        code.push_str("                }\n");
        code.push_str("                if (has_tc) {\n");
        code.push_str(
            "                    match_len = trailing_context_split(last_accepting_rule, start, match_len);\n",
        );
        code.push_str("                    last_accepting_pos = start + match_len;\n");
        code.push_str("                }\n");
        code.push_str("            }\n");
    }
    code.push_str("            lexer->current = last_accepting_pos;\n");
    code.push_str("            TokenType tt = rule_to_token(last_accepting_rule);\n");
    code.push_str("            if (tt == TOKEN_EOF) {\n");
    code.push_str("                // Skip token, continue\n");
    code.push_str("                continue;\n");
    code.push_str("            }\n");
    code.push_str("            token.type = tt;\n");
    code.push_str("            token.start = start;\n");
    code.push_str("            token.length = (int)(last_accepting_pos - start);\n");
    code.push_str("            return token;\n");
    code.push_str("        } else {\n");
    code.push_str("            // No match, error\n");
    code.push_str("            token.type = TOKEN_ERROR;\n");
    code.push_str("            token.start = start;\n");
    code.push_str("            token.length = 1;\n");
    code.push_str("            lexer->current = start + 1;\n");
    code.push_str("            return token;\n");
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    code.push_str("    token.type = TOKEN_EOF;\n");
    code.push_str("    return token;\n");
    code.push_str("}\n\n");

    // Output-param variant of lexer_next(): avoids handing the Token struct
    // itself across a translation-unit boundary (e.g. to a parser's yylex()
    // adapter), since Token's exact layout can differ between this codegen
    // path and generate_c_with_conditions's.
    code.push_str("int lexer_next_token(Lexer* lexer, const char** out_text, int* out_len) {\n");
    code.push_str("    Token tok = lexer_next(lexer);\n");
    code.push_str("    *out_text = tok.start;\n");
    code.push_str("    *out_len = tok.length;\n");
    code.push_str("    return (int)tok.type;\n");
    code.push_str("}\n");

    // Built-in test driver
    code.push_str("\n");
    code.push_str(&generate_c_test_driver());

    Ok(code)
}

fn generate_java_full(
    dfa: &Dfa,
    spec: &LexerSpec,
    trailing_dfas: &TrailingContextDfas,
) -> Result<String> {
    let mut code = String::new();

    code.push_str("import java.util.HashMap;\n");
    code.push_str("import java.util.Map;\n\n");

    code.push_str("public class Lexer {\n");

    // Generate token type enum
    code.push_str("    public enum TokenType {\n");
    code.push_str("        TOKEN_EOF,\n");
    code.push_str("        TOKEN_ERROR,\n");
    let mut seen_tokens = std::collections::HashSet::new();
    seen_tokens.insert("EOF".to_string());
    seen_tokens.insert("ERROR".to_string());
    for rule in &spec.rules {
        if let RuleAction::Token(name) = &rule.action {
            let upper = name.to_uppercase();
            if seen_tokens.insert(upper.clone()) {
                code.push_str(&format!("        TOKEN_{},\n", upper));
            }
        }
    }
    code.push_str("    }\n\n");

    // Token class with position tracking
    code.push_str("    public static class Token {\n");
    code.push_str("        public TokenType type;\n");
    code.push_str("        public String text;\n");
    code.push_str("        public int pos;\n");
    code.push_str("        public int line;\n");
    code.push_str("        public int column;\n");
    code.push_str(
        "        public Token(TokenType type, String text, int pos, int line, int column) {\n",
    );
    code.push_str("            this.type = type;\n");
    code.push_str("            this.text = text;\n");
    code.push_str("            this.pos = pos;\n");
    code.push_str("            this.line = line;\n");
    code.push_str("            this.column = column;\n");
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    // Lexer fields with line/column tracking
    code.push_str("    private String input;\n");
    code.push_str("    private int pos;\n");
    code.push_str("    private int line = 1;\n");
    code.push_str("    private int column = 1;\n");
    code.push_str(
        "    private static final Map<Integer, Integer> ACCEPTING = new HashMap<>();\n\n",
    );

    // Generate transition method using range comparisons
    code.push_str("    private static int getNextState(int state, int codepoint) {\n");
    code.push_str("        switch (state) {\n");
    for state in &dfa.states {
        code.push_str(&format!("            case {}:\n", state.id));
        if !state.range_transitions.is_empty() {
            for &(start_cp, end_cp, target) in &state.range_transitions {
                if start_cp == end_cp {
                    code.push_str(&format!(
                        "                if (codepoint == 0x{:X}) return {};\n",
                        start_cp, target
                    ));
                } else {
                    code.push_str(&format!("                if (codepoint >= 0x{:X} && codepoint <= 0x{:X}) return {};\n", start_cp, end_cp, target));
                }
            }
            code.push_str("                return -1;\n");
        } else if !state.transitions.is_empty() {
            // Fallback to char-based
            for (ch, target) in &state.transitions {
                let cp = *ch as u32;
                code.push_str(&format!(
                    "                if (codepoint == 0x{:X}) return {};\n",
                    cp, target
                ));
            }
            code.push_str("                return -1;\n");
        } else {
            code.push_str("                return -1;\n");
        }
    }
    code.push_str("            default: return -1;\n");
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    // Static initializer for accepting states
    code.push_str("    static {\n");
    for state in &dfa.states {
        if state.is_accepting {
            if let Some(rule_idx) = state.rule_index {
                code.push_str(&format!(
                    "        ACCEPTING.put({}, {});\n",
                    state.id, rule_idx
                ));
            }
        }
    }
    code.push_str("    }\n\n");

    // Constructor
    code.push_str("    public Lexer(String input) {\n");
    code.push_str("        this.input = input;\n");
    code.push_str("        this.pos = 0;\n");
    code.push_str("    }\n\n");

    // Rule to token mapping
    code.push_str("    private TokenType ruleToToken(int ruleIndex) {\n");
    code.push_str("        switch (ruleIndex) {\n");
    for (idx, rule) in spec.rules.iter().enumerate() {
        match &rule.action {
            RuleAction::Token(name) => {
                code.push_str(&format!(
                    "            case {}: return TokenType.TOKEN_{};\n",
                    idx,
                    name.to_uppercase()
                ));
            }
            RuleAction::TokenAndBegin(name, _) => {
                code.push_str(&format!(
                    "            case {}: return TokenType.TOKEN_{};\n",
                    idx,
                    name.to_uppercase()
                ));
            }
            RuleAction::Skip | RuleAction::Begin(_) => {
                code.push_str(&format!(
                    "            case {}: return TokenType.TOKEN_EOF; // Skip or state change\n",
                    idx
                ));
            }
            RuleAction::Error => {
                code.push_str(&format!(
                    "            case {}: return TokenType.TOKEN_ERROR;\n",
                    idx
                ));
            }
            RuleAction::Code(_) => {
                code.push_str(&format!("            case {}: return TokenType.TOKEN_EOF; // Code action handled separately\n", idx));
            }
        }
    }
    code.push_str("            default: return TokenType.TOKEN_ERROR;\n");
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    // Trailing context (r1/r2): a small standalone DFA per rule that has it,
    // used to re-scan a match and find where r1 ends. Rules without trailing
    // context have no methods/dispatch entry here.
    fn emit_tc_dfa_methods(code: &mut String, dfas: &HashMap<usize, Dfa>, prefix: &str) {
      for (rule_idx, tdfa) in dfas {
        code.push_str(&format!(
            "    private static int {prefix}Transition{rule_idx}(int state, int codepoint) {{\n"
        ));
        code.push_str("        switch (state) {\n");
        for state in &tdfa.states {
            code.push_str(&format!("            case {}:\n", state.id));
            if !state.range_transitions.is_empty() {
                for &(start_cp, end_cp, target) in &state.range_transitions {
                    if start_cp == end_cp {
                        code.push_str(&format!(
                            "                if (codepoint == 0x{:X}) return {};\n",
                            start_cp, target
                        ));
                    } else {
                        code.push_str(&format!("                if (codepoint >= 0x{:X} && codepoint <= 0x{:X}) return {};\n", start_cp, end_cp, target));
                    }
                }
            } else {
                for (ch, target) in &state.transitions {
                    code.push_str(&format!(
                        "                if (codepoint == 0x{:X}) return {};\n",
                        *ch as u32, target
                    ));
                }
            }
            code.push_str("                return -1;\n");
        }
        code.push_str("            default: return -1;\n");
        code.push_str("        }\n    }\n\n");

        code.push_str(&format!(
            "    private static boolean {prefix}Accepting{rule_idx}(int state) {{\n"
        ));
        code.push_str("        switch (state) {\n");
        for state in &tdfa.states {
            if state.is_accepting {
                code.push_str(&format!("            case {}: return true;\n", state.id));
            }
        }
        code.push_str("            default: return false;\n");
        code.push_str("        }\n    }\n\n");
      }
    }

    emit_tc_dfa_methods(&mut code, &trailing_dfas.r1, "tc");
    emit_tc_dfa_methods(&mut code, &trailing_dfas.r2, "tc2");

    if !trailing_dfas.is_empty() {
        code.push_str("    // True if ruleIndex's r2 exactly matches text (consumes all of it and ends accepting).\n");
        code.push_str("    private static boolean tcR2Matches(int ruleIndex, String text) {\n");
        code.push_str("        int state = 0;\n");
        code.push_str("        int i = 0;\n");
        code.push_str("        switch (ruleIndex) {\n");
        for rule_idx in trailing_dfas.r1.keys() {
            code.push_str(&format!("            case {}:\n", rule_idx));
            code.push_str("                while (i < text.length()) {\n");
            code.push_str("                    int cp = text.codePointAt(i);\n");
            code.push_str(&format!(
                "                    int next = tc2Transition{}(state, cp);\n",
                rule_idx
            ));
            code.push_str("                    if (next < 0) return false;\n");
            code.push_str("                    state = next;\n");
            code.push_str("                    i += Character.charCount(cp);\n");
            code.push_str("                }\n");
            code.push_str(&format!(
                "                return tc2Accepting{}(state);\n",
                rule_idx
            ));
        }
        code.push_str("            default: return false;\n");
        code.push_str("        }\n");
        code.push_str("    }\n\n");

        code.push_str(
            "    // Longest prefix of `text` accepted by ruleIndex's r1 whose remaining\n",
        );
        code.push_str(
            "    // suffix is an exact match for r2 (not just the longest r1 accepts on\n",
        );
        code.push_str(
            "    // its own, which picks the wrong boundary for e.g. a*/a). Falls back to\n",
        );
        code.push_str(
            "    // the full match (text.length()) if no candidate validates.\n",
        );
        code.push_str("    private static int trailingContextSplit(int ruleIndex, String text) {\n");
        code.push_str("        java.util.List<Integer> candidates = new java.util.ArrayList<>();\n");
        code.push_str("        int state = 0;\n");
        code.push_str("        switch (ruleIndex) {\n");
        for rule_idx in trailing_dfas.r1.keys() {
            code.push_str(&format!("            case {}: {{\n", rule_idx));
            code.push_str(&format!(
                "                if (tcAccepting{}(state)) candidates.add(0);\n",
                rule_idx
            ));
            code.push_str("                int i = 0;\n");
            code.push_str("                while (i < text.length()) {\n");
            code.push_str("                    int cp = text.codePointAt(i);\n");
            code.push_str(&format!(
                "                    int next = tcTransition{}(state, cp);\n",
                rule_idx
            ));
            code.push_str("                    if (next < 0) break;\n");
            code.push_str("                    state = next;\n");
            code.push_str("                    i += Character.charCount(cp);\n");
            code.push_str(&format!(
                "                    if (tcAccepting{}(state)) candidates.add(i);\n",
                rule_idx
            ));
            code.push_str("                }\n");
            code.push_str("                break;\n");
            code.push_str("            }\n");
        }
        code.push_str("            default: break;\n");
        code.push_str("        }\n");
        code.push_str("        // split == 0 is excluded even if r1/r2 both validate it: it would\n");
        code.push_str("        // produce a zero-length token and never advance pos.\n");
        code.push_str("        for (int i = candidates.size() - 1; i >= 0; i--) {\n");
        code.push_str("            int split = candidates.get(i);\n");
        code.push_str("            if (split > 0 && tcR2Matches(ruleIndex, text.substring(split))) {\n");
        code.push_str("                return split;\n");
        code.push_str("            }\n");
        code.push_str("        }\n");
        code.push_str("        return text.length();\n");
        code.push_str("    }\n\n");
    }

    // nextToken method - using range-based transitions with line/column tracking
    code.push_str("    public Token nextToken() {\n");
    code.push_str("        while (pos < input.length()) {\n");
    code.push_str("            int start = pos;\n");
    code.push_str("            int startLine = line;\n");
    code.push_str("            int startColumn = column;\n");
    code.push_str("            int state = 0;\n");
    code.push_str("            int lastAcceptingRule = -1;\n");
    code.push_str("            int lastAcceptingPos = start;\n");
    code.push_str("            int lastAcceptingLine = line;\n");
    code.push_str("            int lastAcceptingColumn = column;\n\n");

    code.push_str("            while (pos < input.length()) {\n");
    code.push_str("                int codepoint = input.codePointAt(pos);\n");
    code.push_str("                int nextState = getNextState(state, codepoint);\n");
    code.push_str("                if (nextState == -1) break;\n\n");
    code.push_str("                state = nextState;\n");
    code.push_str("                pos += Character.charCount(codepoint);\n");
    code.push_str(
        "                if (codepoint == '\\n') { line++; column = 1; } else { column++; }\n\n",
    );
    code.push_str("                Integer ruleIdx = ACCEPTING.get(state);\n");
    code.push_str("                if (ruleIdx != null) {\n");
    code.push_str("                    lastAcceptingRule = ruleIdx;\n");
    code.push_str("                    lastAcceptingPos = pos;\n");
    code.push_str("                    lastAcceptingLine = line;\n");
    code.push_str("                    lastAcceptingColumn = column;\n");
    code.push_str("                }\n");
    code.push_str("            }\n\n");

    code.push_str("            if (lastAcceptingRule >= 0) {\n");
    if !trailing_dfas.is_empty() {
        code.push_str("                switch (lastAcceptingRule) {\n");
        for rule_idx in trailing_dfas.r1.keys() {
            code.push_str(&format!("                    case {}: {{\n", rule_idx));
            code.push_str(
                "                        String matched = input.substring(start, lastAcceptingPos);\n",
            );
            code.push_str(&format!(
                "                        int splitLen = trailingContextSplit({}, matched);\n",
                rule_idx
            ));
            code.push_str("                        if (splitLen > 0) {\n");
            code.push_str("                            String kept = matched.substring(0, splitLen);\n");
            code.push_str("                            lastAcceptingPos = start + splitLen;\n");
            code.push_str("                            long newlines = kept.chars().filter(c -> c == '\\n').count();\n");
            code.push_str("                            if (newlines > 0) {\n");
            code.push_str("                                lastAcceptingLine = (int) (startLine + newlines);\n");
            code.push_str("                                int afterNewline = kept.lastIndexOf('\\n');\n");
            code.push_str("                                lastAcceptingColumn = kept.codePointCount(afterNewline, kept.length());\n");
            code.push_str("                            } else {\n");
            code.push_str("                                lastAcceptingLine = startLine;\n");
            code.push_str("                                lastAcceptingColumn = startColumn + kept.codePointCount(0, kept.length());\n");
            code.push_str("                            }\n");
            code.push_str("                        }\n");
            code.push_str("                        break;\n");
            code.push_str("                    }\n");
        }
        code.push_str("                    default: break;\n");
        code.push_str("                }\n");
    }
    code.push_str("                pos = lastAcceptingPos;\n");
    code.push_str("                line = lastAcceptingLine;\n");
    code.push_str("                column = lastAcceptingColumn;\n");
    code.push_str("                TokenType tt = ruleToToken(lastAcceptingRule);\n");
    code.push_str("                if (tt == TokenType.TOKEN_EOF) continue; // Skip\n");
    code.push_str("                return new Token(tt, input.substring(start, lastAcceptingPos), start, startLine, startColumn);\n");
    code.push_str("            } else {\n");
    code.push_str("                pos = start + 1;\n");
    code.push_str("                int cp = input.codePointAt(start);\n");
    code.push_str("                if (cp == '\\n') { line++; column = 1; } else { column++; }\n");
    code.push_str("                return new Token(TokenType.TOKEN_ERROR, input.substring(start, pos), start, startLine, startColumn);\n");
    code.push_str("            }\n");
    code.push_str("        }\n");
    code.push_str("        return new Token(TokenType.TOKEN_EOF, \"\", pos, line, column);\n");
    code.push_str("    }\n");

    // Built-in test driver
    code.push_str("\n");
    code.push_str(&generate_java_test_driver());

    code.push_str("}\n");

    Ok(code)
}

fn generate_python_full(
    dfa: &Dfa,
    spec: &LexerSpec,
    trailing_dfas: &TrailingContextDfas,
) -> Result<String> {
    let mut code = String::new();

    code.push_str("\"\"\"Lexer generated by OpenLexer with Unicode support.\"\"\"\n\n");
    code.push_str("from enum import Enum, auto\n");
    code.push_str("from dataclasses import dataclass\n");
    code.push_str("from typing import Optional, List, Tuple\n\n");

    // Token type enum
    code.push_str("class TokenType(Enum):\n");
    code.push_str("    EOF = auto()\n");
    code.push_str("    ERROR = auto()\n");
    let mut seen_tokens = std::collections::HashSet::new();
    seen_tokens.insert("EOF".to_string());
    seen_tokens.insert("ERROR".to_string());
    for rule in &spec.rules {
        if let RuleAction::Token(name) = &rule.action {
            let upper = name.to_uppercase();
            if seen_tokens.insert(upper.clone()) {
                code.push_str(&format!("    {} = auto()\n", upper));
            }
        }
    }
    code.push_str("\n\n");

    // Token dataclass
    code.push_str("@dataclass\n");
    code.push_str("class Token:\n");
    code.push_str("    type: TokenType\n");
    code.push_str("    text: str\n");
    code.push_str("    pos: int\n");
    code.push_str("    line: int = 1\n");
    code.push_str("    column: int = 1\n\n");

    // Rule to token mapping
    code.push_str("RULE_TO_TOKEN = {\n");
    for (idx, rule) in spec.rules.iter().enumerate() {
        match &rule.action {
            RuleAction::Token(name) => {
                code.push_str(&format!(
                    "    {}: TokenType.{},\n",
                    idx,
                    name.to_uppercase()
                ));
            }
            RuleAction::TokenAndBegin(name, _) => {
                code.push_str(&format!(
                    "    {}: TokenType.{},\n",
                    idx,
                    name.to_uppercase()
                ));
            }
            RuleAction::Skip | RuleAction::Begin(_) => {
                code.push_str(&format!("    {}: None,  # Skip or state change\n", idx));
            }
            RuleAction::Error => {
                code.push_str(&format!("    {}: TokenType.ERROR,\n", idx));
            }
            RuleAction::Code(_) => {
                code.push_str(&format!(
                    "    {}: None,  # Code action handled separately\n",
                    idx
                ));
            }
        }
    }
    code.push_str("}\n\n");

    // Accepting states with rule index
    code.push_str("ACCEPTING = {\n");
    for state in &dfa.states {
        if state.is_accepting {
            if let Some(rule_idx) = state.rule_index {
                code.push_str(&format!("    {}: {},\n", state.id, rule_idx));
            }
        }
    }
    code.push_str("}\n\n");

    // Generate get_next_state function with range-based transitions
    code.push_str("def get_next_state(state: int, codepoint: int) -> int:\n");
    code.push_str("    \"\"\"Get next DFA state for given state and Unicode codepoint.\"\"\"\n");

    for state in &dfa.states {
        code.push_str(&format!("    if state == {}:\n", state.id));
        if !state.range_transitions.is_empty() {
            let mut first = true;
            for &(start_cp, end_cp, target) in &state.range_transitions {
                let keyword = if first { "if" } else { "elif" };
                first = false;
                if start_cp == end_cp {
                    code.push_str(&format!(
                        "        {} codepoint == 0x{:X}:\n",
                        keyword, start_cp
                    ));
                } else {
                    code.push_str(&format!(
                        "        {} 0x{:X} <= codepoint <= 0x{:X}:\n",
                        keyword, start_cp, end_cp
                    ));
                }
                code.push_str(&format!("            return {}\n", target));
            }
            code.push_str("        return -1\n");
        } else if !state.transitions.is_empty() {
            // Fallback to single-char transitions
            let mut first = true;
            for (ch, target) in &state.transitions {
                let keyword = if first { "if" } else { "elif" };
                first = false;
                code.push_str(&format!(
                    "        {} codepoint == 0x{:X}:\n",
                    keyword, *ch as u32
                ));
                code.push_str(&format!("            return {}\n", target));
            }
            code.push_str("        return -1\n");
        } else {
            code.push_str("        return -1\n");
        }
    }
    code.push_str("    return -1\n\n");

    // Trailing context (r1/r2): a small standalone DFA per rule that has it,
    // used to re-scan a match and find where r1 ends. Rules without trailing
    // context have no entry here.
    fn emit_python_tc_dict(code: &mut String, name: &str, dfas: &HashMap<usize, Dfa>) {
        code.push_str(&format!("{}: dict = {{\n", name));
        for (rule_idx, tdfa) in dfas {
            code.push_str(&format!("    {}: (\n        {{", rule_idx));
            for state in &tdfa.states {
                if !state.range_transitions.is_empty() {
                    code.push_str(&format!("{}: [", state.id));
                    for &(start_cp, end_cp, target) in &state.range_transitions {
                        code.push_str(&format!("(0x{:X}, 0x{:X}, {}), ", start_cp, end_cp, target));
                    }
                    code.push_str("], ");
                } else if !state.transitions.is_empty() {
                    code.push_str(&format!("{}: [", state.id));
                    for (ch, target) in &state.transitions {
                        let cp = *ch as u32;
                        code.push_str(&format!("(0x{:X}, 0x{:X}, {}), ", cp, cp, target));
                    }
                    code.push_str("], ");
                }
            }
            code.push_str("},\n        {");
            for state in &tdfa.states {
                if state.is_accepting {
                    code.push_str(&format!("{}, ", state.id));
                }
            }
            code.push_str("},\n    ),\n");
        }
        code.push_str("}\n\n");
    }

    code.push_str("# Trailing context ('r1/r2'): rule_index -> (transitions, accepting_states)\n");
    emit_python_tc_dict(&mut code, "TRAILING_CONTEXT", &trailing_dfas.r1);
    emit_python_tc_dict(&mut code, "TRAILING_CONTEXT_R2", &trailing_dfas.r2);

    if !trailing_dfas.is_empty() {
        code.push_str("def _tc_r2_matches(rule_index: int, text: str) -> bool:\n");
        code.push_str("    \"\"\"True if rule_index's r2 exactly matches all of `text`.\"\"\"\n");
        code.push_str("    trans, accept = TRAILING_CONTEXT_R2[rule_index]\n");
        code.push_str("    state = 0\n");
        code.push_str("    for ch in text:\n");
        code.push_str("        cp = ord(ch)\n");
        code.push_str("        next_state = -1\n");
        code.push_str("        for start_cp, end_cp, target in trans.get(state, []):\n");
        code.push_str("            if start_cp <= cp <= end_cp:\n");
        code.push_str("                next_state = target\n");
        code.push_str("                break\n");
        code.push_str("        if next_state == -1:\n");
        code.push_str("            return False\n");
        code.push_str("        state = next_state\n");
        code.push_str("    return state in accept\n\n");

        code.push_str("def _trailing_context_split(rule_index: int, text: str) -> int:\n");
        code.push_str(
            "    \"\"\"Longest r1-accepted prefix of `text` whose remaining suffix is an\n",
        );
        code.push_str(
            "    exact match for r2 (not just the longest r1 accepts on its own, which\n",
        );
        code.push_str(
            "    picks the wrong boundary for e.g. a*/a). Falls back to the full match\n",
        );
        code.push_str("    (len(text)) if no candidate validates.\"\"\"\n");
        code.push_str("    trans, accept = TRAILING_CONTEXT[rule_index]\n");
        code.push_str("    state = 0\n");
        code.push_str("    candidates = []\n");
        code.push_str("    if state in accept:\n");
        code.push_str("        candidates.append(0)\n");
        code.push_str("    for i, ch in enumerate(text):\n");
        code.push_str("        cp = ord(ch)\n");
        code.push_str("        next_state = -1\n");
        code.push_str("        for start_cp, end_cp, target in trans.get(state, []):\n");
        code.push_str("            if start_cp <= cp <= end_cp:\n");
        code.push_str("                next_state = target\n");
        code.push_str("                break\n");
        code.push_str("        if next_state == -1:\n");
        code.push_str("            break\n");
        code.push_str("        state = next_state\n");
        code.push_str("        if state in accept:\n");
        code.push_str("            candidates.append(i + 1)\n");
        code.push_str("    # split == 0 is excluded even if r1/r2 both validate it: it would\n");
        code.push_str("    # produce a zero-length token and never advance the input position.\n");
        code.push_str("    for split in reversed(candidates):\n");
        code.push_str("        if split > 0 and _tc_r2_matches(rule_index, text[split:]):\n");
        code.push_str("            return split\n");
        code.push_str("    return len(text)\n\n");
    }

    // Lexer class
    code.push_str("class Lexer:\n");
    code.push_str("    def __init__(self, input_str: str):\n");
    code.push_str("        self.input = input_str\n");
    code.push_str("        self.pos = 0\n");
    code.push_str("        self.line = 1\n");
    code.push_str("        self.column = 1\n\n");

    code.push_str("    def next_token(self) -> Token:\n");
    code.push_str("        while self.pos < len(self.input):\n");
    code.push_str("            start = self.pos\n");
    code.push_str("            start_line = self.line\n");
    code.push_str("            start_column = self.column\n");
    code.push_str("            state = 0\n");
    code.push_str("            last_accepting_rule = -1\n");
    code.push_str("            last_accepting_pos = start\n");
    code.push_str("            last_accepting_line = self.line\n");
    code.push_str("            last_accepting_column = self.column\n\n");

    code.push_str("            while self.pos < len(self.input):\n");
    code.push_str("                codepoint = ord(self.input[self.pos])\n");
    code.push_str("                next_state = get_next_state(state, codepoint)\n");
    code.push_str("                if next_state == -1:\n");
    code.push_str("                    break\n");
    code.push_str("                state = next_state\n");
    code.push_str("                self.pos += 1\n");
    code.push_str("                if codepoint == 0x0A:\n");
    code.push_str("                    self.line += 1\n");
    code.push_str("                    self.column = 1\n");
    code.push_str("                else:\n");
    code.push_str("                    self.column += 1\n\n");

    code.push_str("                if state in ACCEPTING:\n");
    code.push_str("                    last_accepting_rule = ACCEPTING[state]\n");
    code.push_str("                    last_accepting_pos = self.pos\n");
    code.push_str("                    last_accepting_line = self.line\n");
    code.push_str("                    last_accepting_column = self.column\n\n");

    code.push_str("            if last_accepting_rule >= 0:\n");
    code.push_str("                if last_accepting_rule in TRAILING_CONTEXT:\n");
    code.push_str("                    matched = self.input[start:last_accepting_pos]\n");
    code.push_str(
        "                    split_len = _trailing_context_split(last_accepting_rule, matched)\n",
    );
    code.push_str("                    last_accepting_pos = start + split_len\n");
    code.push_str("                    kept = self.input[start:last_accepting_pos]\n");
    code.push_str("                    newlines = kept.count('\\n')\n");
    code.push_str("                    if newlines:\n");
    code.push_str("                        last_accepting_line = start_line + newlines\n");
    code.push_str("                        last_accepting_column = len(kept) - kept.rfind('\\n')\n");
    code.push_str("                    else:\n");
    code.push_str("                        last_accepting_line = start_line\n");
    code.push_str("                        last_accepting_column = start_column + len(kept)\n");
    code.push_str("                self.pos = last_accepting_pos\n");
    code.push_str("                self.line = last_accepting_line\n");
    code.push_str("                self.column = last_accepting_column\n");
    code.push_str("                token_type = RULE_TO_TOKEN.get(last_accepting_rule)\n");
    code.push_str("                if token_type is None:  # Skip\n");
    code.push_str("                    continue\n");
    code.push_str("                return Token(token_type, self.input[start:last_accepting_pos], start, start_line, start_column)\n");
    code.push_str("            else:\n");
    code.push_str("                self.pos = start + 1\n");
    code.push_str("                cp = ord(self.input[start])\n");
    code.push_str("                if cp == 0x0A:\n");
    code.push_str("                    self.line += 1\n");
    code.push_str("                    self.column = 1\n");
    code.push_str("                else:\n");
    code.push_str("                    self.column += 1\n");
    code.push_str("                return Token(TokenType.ERROR, self.input[start:self.pos], start, start_line, start_column)\n\n");

    code.push_str("        return Token(TokenType.EOF, '', self.pos, self.line, self.column)\n\n");

    code.push_str("    def tokenize(self):\n");
    code.push_str("        \"\"\"Generator that yields all tokens.\"\"\"\n");
    code.push_str("        while True:\n");
    code.push_str("            token = self.next_token()\n");
    code.push_str("            yield token\n");
    code.push_str("            if token.type == TokenType.EOF:\n");
    code.push_str("                break\n");

    // Built-in test driver
    code.push_str("\n\n");
    code.push_str(&generate_python_test_driver());

    Ok(code)
}

// =============================================================================
// Simple single-pattern lexer generation (legacy)
// =============================================================================

fn generate_c_simple(dfa: &Dfa) -> Result<String> {
    let mut code = String::new();

    code.push_str("#include <stdio.h>\n");
    code.push_str("#include <stdbool.h>\n\n");
    code.push_str("typedef enum { TOKEN_INVALID, TOKEN_MATCH } TokenType;\n\n");

    code.push_str("TokenType lex(const char* input) {\n");
    code.push_str("    int state = 0;\n");
    code.push_str("    const char* c = input;\n");
    code.push_str("    while (*c != '\\0') {\n");
    code.push_str("        switch (state) {\n");

    for state in &dfa.states {
        code.push_str(&format!("            case {}:\n", state.id));
        code.push_str("                switch (*c) {\n");
        for (ch, target) in &state.transitions {
            let escaped = escape_c_char(*ch);
            code.push_str(&format!(
                "                    case {}: state = {}; break;\n",
                escaped, target
            ));
        }
        code.push_str("                    default: return TOKEN_INVALID;\n");
        code.push_str("                }\n");
        code.push_str("                break;\n");
    }

    code.push_str("        }\n");
    code.push_str("        c++;\n");
    code.push_str("    }\n\n");

    code.push_str("    switch (state) {\n");
    for state in &dfa.states {
        if state.is_accepting {
            code.push_str(&format!("        case {}: return TOKEN_MATCH;\n", state.id));
        }
    }
    code.push_str("        default: return TOKEN_INVALID;\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    Ok(code)
}

fn generate_java_simple(dfa: &Dfa) -> Result<String> {
    let mut code = String::new();
    code.push_str("public class Lexer {\n");
    code.push_str("    public enum TokenType { INVALID, MATCH }\n\n");
    code.push_str("    public static TokenType lex(String input) {\n");
    code.push_str("        int state = 0;\n");
    code.push_str("        for (int i = 0; i < input.length(); i++) {\n");
    code.push_str("            char c = input.charAt(i);\n");
    code.push_str("            switch (state) {\n");

    for state in &dfa.states {
        code.push_str(&format!("                case {}:\n", state.id));
        code.push_str("                    switch (c) {\n");
        for (ch, target) in &state.transitions {
            let escaped = escape_java_char(*ch);
            code.push_str(&format!(
                "                        case {}: state = {}; break;\n",
                escaped, target
            ));
        }
        code.push_str("                        default: return TokenType.INVALID;\n");
        code.push_str("                    }\n");
        code.push_str("                    break;\n");
    }

    code.push_str("            }\n");
    code.push_str("        }\n\n");

    code.push_str("        switch (state) {\n");
    for state in &dfa.states {
        if state.is_accepting {
            code.push_str(&format!(
                "            case {}: return TokenType.MATCH;\n",
                state.id
            ));
        }
    }
    code.push_str("            default: return TokenType.INVALID;\n");
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    Ok(code)
}

fn generate_python_simple(dfa: &Dfa) -> Result<String> {
    let mut code = String::new();

    code.push_str("class TokenType:\n");
    code.push_str("    INVALID = 0\n");
    code.push_str("    MATCH = 1\n\n");

    code.push_str("def lex(input_str):\n");
    code.push_str("    \"\"\"Lexer function generated by OpenLexer.\"\"\"\n");

    code.push_str("    transitions = {\n");
    for state in &dfa.states {
        code.push_str(&format!("        {}: {{", state.id));
        for (ch, target) in &state.transitions {
            let escaped = escape_python_char(*ch);
            code.push_str(&format!("'{}': {}, ", escaped, target));
        }
        code.push_str("},\n");
    }
    code.push_str("    }\n\n");

    let accepting: Vec<String> = dfa
        .states
        .iter()
        .filter(|s| s.is_accepting)
        .map(|s| s.id.to_string())
        .collect();
    code.push_str(&format!(
        "    accepting_states = {{{}}}\n\n",
        accepting.join(", ")
    ));

    code.push_str("    state = 0\n");
    code.push_str("    for c in input_str:\n");
    code.push_str("        state_transitions = transitions.get(state)\n");
    code.push_str("        if state_transitions is None:\n");
    code.push_str("            return TokenType.INVALID\n");
    code.push_str("        next_state = state_transitions.get(c)\n");
    code.push_str("        if next_state is None:\n");
    code.push_str("            return TokenType.INVALID\n");
    code.push_str("        state = next_state\n\n");

    code.push_str(
        "    return TokenType.MATCH if state in accepting_states else TokenType.INVALID\n",
    );

    Ok(code)
}

// =============================================================================
// Built-in Test Drivers
// =============================================================================

/// Generates a standalone Python test driver.
/// Can be appended to a generated lexer or written as a separate file.
pub fn generate_python_test_driver() -> String {
    let mut code = String::new();
    code.push_str("def test(expr: str):\n");
    code.push_str("    \"\"\"Test the lexer with an expression. Prints all tokens found.\"\"\"\n");
    code.push_str("    print(f\"Input: {expr!r}\")\n");
    code.push_str("    lexer = Lexer(expr)\n");
    code.push_str("    tokens = []\n");
    code.push_str("    for token in lexer.tokenize():\n");
    code.push_str("        tokens.append(token)\n");
    code.push_str("        if hasattr(token, 'line'):\n");
    code.push_str("            print(f\"  {token.type.name:12s} | {token.text!r:15s} | pos={token.pos} line={token.line} col={token.column}\")\n");
    code.push_str("        else:\n");
    code.push_str(
        "            print(f\"  {token.type.name:12s} | {token.text!r:15s} | pos={token.pos}\")\n",
    );
    code.push_str("    print()\n");
    code.push_str("    return tokens\n\n\n");
    code.push_str("def test_all(*expressions):\n");
    code.push_str("    \"\"\"Test the lexer with multiple expressions.\n");
    code.push_str("    \n");
    code.push_str("    Usage:\n");
    code.push_str("        from lexer import test, test_all\n");
    code.push_str("        test('3 + 4 * 2')\n");
    code.push_str("        test_all('1+2', '(3*4)', 'hello world')\n");
    code.push_str("    \"\"\"\n");
    code.push_str("    results = []\n");
    code.push_str("    for expr in expressions:\n");
    code.push_str("        results.append(test(expr))\n");
    code.push_str("    return results\n\n\n");
    code.push_str("if __name__ == '__main__':\n");
    code.push_str("    import sys\n");
    code.push_str("    if len(sys.argv) > 1:\n");
    code.push_str("        # Test with command-line arguments\n");
    code.push_str("        for arg in sys.argv[1:]:\n");
    code.push_str("            test(arg)\n");
    code.push_str("    else:\n");
    code.push_str("        # Try reading from stdin\n");
    code.push_str("        _input = sys.stdin.read().strip()\n");
    code.push_str("        if _input:\n");
    code.push_str("            for line in _input.splitlines():\n");
    code.push_str("                test(line)\n");
    code.push_str("        else:\n");
    code.push_str("            print('=== OpenLexer Test Driver ===')\n");
    code.push_str("            test('3 + 4 * 2')\n");
    code.push_str("            test('(10 - 2) / 4')\n");
    code
}

/// Generates a standalone C test driver.
pub fn generate_c_test_driver() -> String {
    let mut code = String::new();
    code.push_str("#ifndef LEXER_NO_TEST\n");
    code.push_str("/* === Built-in Test Driver === */\n\n");
    code.push_str("static void test(const char* expr) {\n");
    code.push_str("    printf(\"Input: \\\"%s\\\"\\n\", expr);\n");
    code.push_str("    Lexer lexer;\n");
    code.push_str("    lexer_init(&lexer, expr);\n");
    code.push_str("    Token token;\n");
    code.push_str("    do {\n");
    code.push_str("        token = lexer_next(&lexer);\n");
    code.push_str("        printf(\"  %-12s | \\\"\", lexer_token_name(token.type));\n");
    code.push_str("        for (int i = 0; i < token.length; i++) putchar(token.start[i]);\n");
    code.push_str("        printf(\"\\\"\\n\");\n");
    code.push_str("    } while (token.type != TOKEN_EOF);\n");
    code.push_str("    printf(\"\\n\");\n");
    code.push_str("}\n");
    code.push_str("#endif\n\n");
    code.push_str("#ifndef LEXER_NO_MAIN\n");
    code.push_str("int main(int argc, char** argv) {\n");
    code.push_str("    if (argc > 1) {\n");
    code.push_str("        for (int i = 1; i < argc; i++) {\n");
    code.push_str("            test(argv[i]);\n");
    code.push_str("        }\n");
    code.push_str("    } else {\n");
    code.push_str("        char line[4096];\n");
    code.push_str("        if (fgets(line, sizeof(line), stdin)) {\n");
    code.push_str("            int len = strlen(line);\n");
    code.push_str("            if (len > 0 && line[len-1] == '\\n') line[len-1] = 0;\n");
    code.push_str("            test(line);\n");
    code.push_str("        } else {\n");
    code.push_str("            printf(\"=== OpenLexer Test Driver ===\\n\");\n");
    code.push_str("            test(\"3 + 4 * 2\");\n");
    code.push_str("            test(\"(10 - 2) / 4\");\n");
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("    return 0;\n");
    code.push_str("}\n");
    code.push_str("#endif /* LEXER_NO_MAIN */\n");
    code
}

/// Generates a standalone Java test driver (methods inside the Lexer class).
pub fn generate_java_test_driver() -> String {
    let mut code = String::new();
    code.push_str("    /* === Built-in Test Driver === */\n\n");
    code.push_str("    public static void test(String expr) {\n");
    code.push_str("        System.out.printf(\"Input: \\\"%s\\\"%n\", expr);\n");
    code.push_str("        Lexer lexer = new Lexer(expr);\n");
    code.push_str("        Token token;\n");
    code.push_str("        do {\n");
    code.push_str("            token = lexer.nextToken();\n");
    code.push_str("            System.out.printf(\"  %-12s | %-15s | pos=%d line=%d col=%d%n\",\n");
    code.push_str("                token.type, \"\\\"\" + token.text.replace(\"\\n\", \"\\\\n\") + \"\\\"\", token.pos, token.line, token.column);\n");
    code.push_str("        } while (token.type != TokenType.TOKEN_EOF);\n");
    code.push_str("        System.out.println();\n");
    code.push_str("    }\n\n");
    code.push_str("    public static void main(String[] args) {\n");
    code.push_str("        if (args.length > 0) {\n");
    code.push_str("            for (String arg : args) {\n");
    code.push_str("                test(arg);\n");
    code.push_str("            }\n");
    code.push_str("        } else {\n");
    code.push_str("            try {\n");
    code.push_str("                java.util.Scanner sc = new java.util.Scanner(System.in);\n");
    code.push_str("                boolean hasInput = false;\n");
    code.push_str("                while (sc.hasNextLine()) {\n");
    code.push_str("                    String line = sc.nextLine().trim();\n");
    code.push_str("                    if (!line.isEmpty()) { test(line); hasInput = true; }\n");
    code.push_str("                }\n");
    code.push_str("                if (!hasInput) {\n");
    code.push_str("                    test(\"3 + 4 * 2\");\n");
    code.push_str("                }\n");
    code.push_str("            } catch (Exception e) {\n");
    code.push_str("                test(\"3 + 4 * 2\");\n");
    code.push_str("            }\n");
    code.push_str("        }\n");
    code.push_str("    }\n");
    code
}

/// Escape a character for use in a Python string literal.
/// Returns just the escaped character content without surrounding quotes.
/// The caller is responsible for adding quotes.
fn escape_python_char(ch: char) -> String {
    match ch {
        '\'' => "\\'".to_string(),
        '\\' => "\\\\".to_string(),
        '\n' => "\\n".to_string(),
        '\t' => "\\t".to_string(),
        '\r' => "\\r".to_string(),
        '\0' => "\\x00".to_string(),
        c if c.is_ascii_control() => format!("\\x{:02x}", c as u32),
        c => c.to_string(),
    }
}

/// Escape a character for use in a C char literal (including quotes).
/// For non-ASCII (Unicode) chars, returns an integer codepoint format.
fn escape_c_char(ch: char) -> String {
    // Non-ASCII characters can't be represented in C char literals
    // Use the codepoint value directly
    if !ch.is_ascii() {
        return format!("0x{:04X}", ch as u32);
    }

    let inner = match ch {
        '\'' => "\\'".to_string(),
        '\\' => "\\\\".to_string(),
        '\n' => "\\n".to_string(),
        '\t' => "\\t".to_string(),
        '\r' => "\\r".to_string(),
        '\0' => "\\0".to_string(),
        c if c.is_ascii_control() => format!("\\x{:02x}", c as u32),
        c => c.to_string(),
    };
    format!("'{}'", inner)
}

/// Escape a character for use in a Java char literal (including quotes).
fn escape_java_char(ch: char) -> String {
    let inner = match ch {
        '\'' => "\\'".to_string(),
        '\\' => "\\\\".to_string(),
        '\n' => "\\n".to_string(),
        '\t' => "\\t".to_string(),
        '\r' => "\\r".to_string(),
        '\0' => "\\0".to_string(),
        c if c.is_ascii_control() => format!("\\u{:04x}", c as u32),
        c if !c.is_ascii() => format!("\\u{:04x}", c as u32),
        c => c.to_string(),
    };
    format!("'{}'", inner)
}
