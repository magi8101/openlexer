//! Step-through debuggers for lexer and parser.
//!
//! LexerDebugger walks a DFA character by character over an input string,
//! recording which states are visited and which tokens are produced.
//!
//! ParserDebugger walks an LALR parsing table over a token stream,
//! recording shift/reduce/accept actions and stack state.

use crate::lexgen::dfa::Dfa;
use crate::lexgen::rules::{LexerSpec, RuleAction};
use crate::parsegen::grammar::{Grammar, Symbol};
use crate::parsegen::lalr::{Action, ParsingTable};

// =============================================================================
// Lexer Debugger
// =============================================================================

/// One step of the lexer debugger.
#[derive(Debug, Clone)]
pub struct LexerDebugStep {
    /// Character consumed in this step.
    pub current_char: char,
    /// DFA state before consuming the character.
    pub from_state: usize,
    /// DFA state after consuming the character.
    pub to_state: Option<usize>,
    /// Position in the input string (byte offset of the character).
    pub position: usize,
    /// Whether the destination state is accepting.
    pub is_accepting: bool,
    /// If accepting, the rule index that matched.
    pub rule_index: Option<usize>,
    /// If accepting, the token name produced.
    pub token_name: Option<String>,
    /// Whether this step completed a token (used by step_token).
    pub token_completed: bool,
    /// The lexeme accumulated so far for the current token.
    pub current_lexeme: String,
}

/// A completed token found during debugging.
#[derive(Debug, Clone)]
pub struct DebugToken {
    pub token_type: String,
    pub lexeme: String,
    /// Byte offset in the input where this token starts.
    pub start_pos: usize,
    /// Byte offset in the input where this token ends (exclusive).
    pub end_pos: usize,
}

/// Steps through a DFA character by character over an input string.
pub struct LexerDebugger {
    dfa: Dfa,
    spec: LexerSpec,
    input: Vec<char>,
    /// Current position in the input (index into `input`).
    pos: usize,
    /// Current DFA state.
    current_state: usize,
    /// Start position of the current token being matched.
    token_start: usize,
    /// Last accepting state seen while scanning the current token.
    last_accept_state: Option<usize>,
    /// Position after the last accepting state (for longest-match backtracking).
    last_accept_pos: usize,
    /// Last accepting rule index.
    last_accept_rule: Option<usize>,
    /// Tokens found so far.
    pub tokens: Vec<DebugToken>,
    /// Whether the debugger has finished processing all input.
    finished: bool,
}

impl LexerDebugger {
    pub fn new(dfa: Dfa, spec: LexerSpec, input: &str) -> Self {
        let start = dfa.start_state;
        Self {
            dfa,
            spec,
            input: input.chars().collect(),
            pos: 0,
            current_state: start,
            token_start: 0,
            last_accept_state: None,
            last_accept_pos: 0,
            last_accept_rule: None,
            tokens: Vec::new(),
            finished: false,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn current_position(&self) -> usize {
        self.pos
    }

    pub fn current_state_id(&self) -> usize {
        self.current_state
    }

    pub fn input_len(&self) -> usize {
        self.input.len()
    }

    pub fn input_str(&self) -> String {
        self.input.iter().collect()
    }

    /// Advance one character through the DFA. Returns the step taken.
    /// If the current token is completed (dead state or end of input),
    /// the step will have `token_completed = true` and the token is recorded.
    pub fn step(&mut self) -> Option<LexerDebugStep> {
        if self.finished {
            return None;
        }

        // If we are past end of input, emit any pending token
        if self.pos >= self.input.len() {
            return self.finalize_current_token();
        }

        let ch = self.input[self.pos];
        let cp = ch as u32;
        let from_state = self.current_state;

        // Find transition
        let mut next_state = None;
        let dfa_state = &self.dfa.states[from_state];
        for &(range_start, range_end, target) in &dfa_state.range_transitions {
            if cp >= range_start && cp <= range_end {
                next_state = Some(target);
                break;
            }
        }

        let lexeme_so_far: String = self.input[self.token_start..=self.pos].iter().collect();

        match next_state {
            Some(target) => {
                let target_state = &self.dfa.states[target];
                let is_accepting = target_state.is_accepting;
                let rule_index = target_state.rule_index;
                let token_name = if is_accepting {
                    rule_index.and_then(|idx| self.rule_token_name(idx))
                } else {
                    None
                };

                if is_accepting {
                    self.last_accept_state = Some(target);
                    self.last_accept_pos = self.pos + 1;
                    self.last_accept_rule = rule_index;
                }

                self.current_state = target;
                self.pos += 1;

                Some(LexerDebugStep {
                    current_char: ch,
                    from_state,
                    to_state: Some(target),
                    position: self.pos - 1,
                    is_accepting,
                    rule_index,
                    token_name,
                    token_completed: false,
                    current_lexeme: lexeme_so_far,
                })
            }
            None => {
                // Dead state -- emit token if we had an accepting state
                let step = self.emit_token_from_accept(ch, from_state);
                Some(step)
            }
        }
    }

    /// Advance until the next complete token is produced or input is exhausted.
    pub fn step_token(&mut self) -> Vec<LexerDebugStep> {
        let mut steps = Vec::new();
        loop {
            match self.step() {
                Some(step) => {
                    let completed = step.token_completed;
                    steps.push(step);
                    if completed || self.finished {
                        break;
                    }
                }
                None => break,
            }
        }
        steps
    }

    /// Run the debugger to completion, collecting all steps.
    pub fn run_all(&mut self) -> Vec<LexerDebugStep> {
        let mut steps = Vec::new();
        while !self.finished {
            if let Some(step) = self.step() {
                steps.push(step);
            }
        }
        steps
    }

    /// Reset the debugger to the beginning.
    pub fn reset(&mut self) {
        self.pos = 0;
        self.current_state = self.dfa.start_state;
        self.token_start = 0;
        self.last_accept_state = None;
        self.last_accept_pos = 0;
        self.last_accept_rule = None;
        self.tokens.clear();
        self.finished = false;
    }

    /// Reset with new input text.
    pub fn reset_with_input(&mut self, input: &str) {
        self.input = input.chars().collect();
        self.reset();
    }

    // --- Private helpers ---

    fn rule_token_name(&self, rule_index: usize) -> Option<String> {
        if rule_index < self.spec.rules.len() {
            match &self.spec.rules[rule_index].action {
                RuleAction::Token(name) => Some(name.clone()),
                RuleAction::Skip => Some("(skip)".to_string()),
                RuleAction::Error => Some("(error)".to_string()),
                RuleAction::Begin(state) => Some(format!("BEGIN({})", state)),
                RuleAction::Code(code) => Some(format!("{{...{} chars}}", code.len())),
                RuleAction::TokenAndBegin(token, state) => {
                    Some(format!("{} + BEGIN({})", token, state))
                }
            }
        } else {
            None
        }
    }

    fn finalize_current_token(&mut self) -> Option<LexerDebugStep> {
        if let Some(rule_idx) = self.last_accept_rule {
            let lexeme: String = self.input[self.token_start..self.last_accept_pos]
                .iter()
                .collect();
            let token_name = self.rule_token_name(rule_idx);
            let is_skip = rule_idx < self.spec.rules.len()
                && self.spec.rules[rule_idx].action == RuleAction::Skip;

            if !is_skip {
                self.tokens.push(DebugToken {
                    token_type: token_name.clone().unwrap_or_else(|| "???".to_string()),
                    lexeme: lexeme.clone(),
                    start_pos: self.token_start,
                    end_pos: self.last_accept_pos,
                });
            }

            self.finished = true;

            Some(LexerDebugStep {
                current_char: '\0',
                from_state: self.current_state,
                to_state: None,
                position: self.pos,
                is_accepting: true,
                rule_index: Some(rule_idx),
                token_name,
                token_completed: true,
                current_lexeme: lexeme,
            })
        } else {
            self.finished = true;
            None
        }
    }

    fn emit_token_from_accept(&mut self, ch: char, from_state: usize) -> LexerDebugStep {
        if let Some(rule_idx) = self.last_accept_rule {
            let lexeme: String = self.input[self.token_start..self.last_accept_pos]
                .iter()
                .collect();
            let token_name = self.rule_token_name(rule_idx);
            let is_skip = rule_idx < self.spec.rules.len()
                && self.spec.rules[rule_idx].action == RuleAction::Skip;

            if !is_skip {
                self.tokens.push(DebugToken {
                    token_type: token_name.clone().unwrap_or_else(|| "???".to_string()),
                    lexeme: lexeme.clone(),
                    start_pos: self.token_start,
                    end_pos: self.last_accept_pos,
                });
            }

            // Reset for the next token -- backtrack to just after the accepted position
            self.token_start = self.last_accept_pos;
            self.pos = self.last_accept_pos;
            self.current_state = self.dfa.start_state;
            self.last_accept_state = None;
            self.last_accept_rule = None;

            if self.pos >= self.input.len() {
                self.finished = true;
            }

            LexerDebugStep {
                current_char: ch,
                from_state,
                to_state: None,
                position: self.pos,
                is_accepting: true,
                rule_index: Some(rule_idx),
                token_name,
                token_completed: true,
                current_lexeme: lexeme,
            }
        } else {
            // No previous accepting state -- unrecognized character, skip it
            self.pos += 1;
            self.token_start = self.pos;
            self.current_state = self.dfa.start_state;

            if self.pos >= self.input.len() {
                self.finished = true;
            }

            LexerDebugStep {
                current_char: ch,
                from_state,
                to_state: None,
                position: self.pos - 1,
                is_accepting: false,
                rule_index: None,
                token_name: None,
                token_completed: false,
                current_lexeme: ch.to_string(),
            }
        }
    }
}

// =============================================================================
// Parser Debugger
// =============================================================================

/// One step of the parser debugger.
#[derive(Debug, Clone)]
pub struct ParserDebugStep {
    /// The step number (0-indexed).
    pub step_number: usize,
    /// The action taken: "Shift N", "Reduce R", "Accept", or "Error".
    pub action_description: String,
    /// The lookahead token type.
    pub lookahead: String,
    /// The lookahead token value.
    pub lookahead_value: String,
    /// Snapshot of the parse stack (state IDs).
    pub stack_states: Vec<usize>,
    /// Snapshot of the symbol stack.
    pub stack_symbols: Vec<String>,
    /// If a reduction was performed, the rule description.
    pub reduced_rule: Option<String>,
    /// Whether this step ended parsing (accept or error).
    pub is_terminal: bool,
}

/// A token for the parser debugger.
#[derive(Debug, Clone)]
pub struct ParserToken {
    pub token_type: String,
    pub value: String,
}

/// A node in the parse tree.
#[derive(Debug, Clone)]
pub struct DebugNode {
    pub id: usize,
    pub label: String,
    pub children: Vec<usize>,
}

/// Steps through an LALR parsing table over a token stream.
pub struct ParserDebugger {
    table: ParsingTable,
    grammar: Grammar,
    tokens: Vec<ParserToken>,
    /// Current position in the token stream.
    token_pos: usize,
    /// The parse stack (state IDs).
    state_stack: Vec<usize>,
    /// The symbol stack (symbol names for display).
    symbol_stack: Vec<String>,
    /// Step counter.
    step_count: usize,
    /// Whether parsing has finished.
    finished: bool,
    /// Whether parsing ended successfully.
    pub accepted: bool,
    /// Error message if parsing failed.
    pub error: Option<String>,
    
    // Tree building support
    pub tree_nodes: Vec<DebugNode>,
    /// Stack of node IDs corresponding to the symbol stack.
    /// node_stack.len() should track state_stack.len() - 1 (since state stack has initial state 0).
    pub node_stack: Vec<usize>,
}

impl ParserDebugger {
    pub fn new(table: ParsingTable, grammar: Grammar, tokens: Vec<ParserToken>) -> Self {
        Self {
            table,
            grammar,
            tokens,
            token_pos: 0,
            state_stack: vec![0],
            symbol_stack: Vec::new(),
            step_count: 0,
            finished: false,
            accepted: false,
            error: None,
            tree_nodes: Vec::new(),
            node_stack: Vec::new(),
        }
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn current_token_pos(&self) -> usize {
        self.token_pos
    }

    pub fn token_count(&self) -> usize {
        self.tokens.len()
    }
    
    pub fn get_node(&self, id: usize) -> Option<&DebugNode> {
        self.tree_nodes.get(id)
    }
    
    /// Returns the root node ID if parsing succeeded (and stack has 1 element: the start symbol).
    pub fn get_tree_root(&self) -> Option<usize> {
        if self.accepted && self.node_stack.len() == 1 {
            Some(self.node_stack[0])
        } else {
            None
        }
    }

    /// Get the current lookahead token type.
    fn current_lookahead(&self) -> &str {
        if self.token_pos < self.tokens.len() {
            &self.tokens[self.token_pos].token_type
        } else {
            "$end"
        }
    }

    /// Get the current lookahead token value.
    fn current_lookahead_value(&self) -> &str {
        if self.token_pos < self.tokens.len() {
            &self.tokens[self.token_pos].value
        } else {
            ""
        }
    }

    fn add_node(&mut self, label: String, children: Vec<usize>) -> usize {
        let id = self.tree_nodes.len();
        self.tree_nodes.push(DebugNode { id, label, children });
        id
    }

    /// Perform one shift/reduce/accept step. Returns the step taken.
    pub fn step(&mut self) -> Option<ParserDebugStep> {
        if self.finished {
            return None;
        }

        let current_state = *self.state_stack.last().unwrap();
        let lookahead = self.current_lookahead().to_string();
        let lookahead_value = self.current_lookahead_value().to_string();

        // Look up action
        let action = self
            .table
            .action
            .get(&current_state)
            .and_then(|row| row.get(&lookahead))
            .cloned();

        let step_number = self.step_count;
        self.step_count += 1;

        match action {
            Some(Action::Shift(next_state)) => {
                self.state_stack.push(next_state);
                self.symbol_stack.push(lookahead.clone());
                
                // Tree: Create leaf node for the shifted token
                let label = if lookahead_value.is_empty() || lookahead_value == lookahead {
                    lookahead.clone()
                } else {
                    format!("{}({})", lookahead, lookahead_value)
                };
                let node_id = self.add_node(label, vec![]);
                self.node_stack.push(node_id);
                
                self.token_pos += 1;

                Some(ParserDebugStep {
                    step_number,
                    action_description: format!("Shift {}", next_state),
                    lookahead,
                    lookahead_value,
                    stack_states: self.state_stack.clone(),
                    stack_symbols: self.symbol_stack.clone(),
                    reduced_rule: None,
                    is_terminal: false,
                })
            }
            Some(Action::Reduce(rule_idx)) => {
                let rule = &self.grammar.rules[rule_idx];
                let lhs = rule.lhs.clone();
                let rhs_len = rule.rhs.len();
                let rule_desc = format_rule(&self.grammar, rule_idx);

                // Pop rhs_len symbols
                let mut children = Vec::new();
                for _ in 0..rhs_len {
                    self.state_stack.pop();
                    self.symbol_stack.pop();
                    if let Some(node_id) = self.node_stack.pop() {
                        children.push(node_id);
                    }
                }
                // Children were popped in reverse order (right to left), reverse them back
                children.reverse();

                // Push LHS
                let goto_state = *self.state_stack.last().unwrap();
                let next_state = self
                    .table
                    .goto
                    .get(&goto_state)
                    .and_then(|row| row.get(&lhs))
                    .copied();

                match next_state {
                    Some(ns) => {
                        self.state_stack.push(ns);
                        self.symbol_stack.push(lhs.clone());
                        
                        // Tree: Create parent node
                        let node_id = self.add_node(lhs, children);
                        self.node_stack.push(node_id);

                        Some(ParserDebugStep {
                            step_number,
                            action_description: format!("Reduce {}", rule_idx),
                            lookahead,
                            lookahead_value,
                            stack_states: self.state_stack.clone(),
                            stack_symbols: self.symbol_stack.clone(),
                            reduced_rule: Some(rule_desc),
                            is_terminal: false,
                        })
                    }
                    None => {
                        self.finished = true;
                        self.error = Some(format!(
                            "No GOTO entry for state {} with nonterminal {}",
                            goto_state, lhs
                        ));

                        Some(ParserDebugStep {
                            step_number,
                            action_description: "Error (no GOTO)".to_string(),
                            lookahead,
                            lookahead_value,
                            stack_states: self.state_stack.clone(),
                            stack_symbols: self.symbol_stack.clone(),
                            reduced_rule: Some(rule_desc),
                            is_terminal: true,
                        })
                    }
                }
            }
            Some(Action::Accept) => {
                self.finished = true;
                self.accepted = true;

                Some(ParserDebugStep {
                    step_number,
                    action_description: "Accept".to_string(),
                    lookahead,
                    lookahead_value,
                    stack_states: self.state_stack.clone(),
                    stack_symbols: self.symbol_stack.clone(),
                    reduced_rule: None,
                    is_terminal: true,
                })
            }
            None => {
                self.finished = true;
                let expected: Vec<String> = self
                    .table
                    .action
                    .get(&current_state)
                    .map(|row| row.keys().cloned().collect())
                    .unwrap_or_default();

                self.error = Some(format!(
                    "Syntax error at token '{}' (value: '{}') in state {}. Expected: {:?}",
                    lookahead, lookahead_value, current_state, expected
                ));

                Some(ParserDebugStep {
                    step_number,
                    action_description: format!("Error in state {}", current_state),
                    lookahead,
                    lookahead_value,
                    stack_states: self.state_stack.clone(),
                    stack_symbols: self.symbol_stack.clone(),
                    reduced_rule: None,
                    is_terminal: true,
                })
            }
        }
    }

    /// Run to completion, collecting all steps.
    pub fn run_all(&mut self) -> Vec<ParserDebugStep> {
        let mut steps = Vec::new();
        while !self.finished {
            if let Some(step) = self.step() {
                steps.push(step);
            }
        }
        steps
    }

    /// Reset the debugger to the beginning.
    pub fn reset(&mut self) {
        self.token_pos = 0;
        self.state_stack = vec![0];
        self.symbol_stack.clear();
        self.step_count = 0;
        self.finished = false;
        self.accepted = false;
        self.error = None;
        self.tree_nodes.clear();
        self.node_stack.clear();
    }
}

/// Format a grammar rule for display: "expr -> expr PLUS term"
fn format_rule(grammar: &Grammar, rule_idx: usize) -> String {
    if rule_idx >= grammar.rules.len() {
        return format!("rule {}", rule_idx);
    }
    let rule = &grammar.rules[rule_idx];
    let rhs: Vec<&str> = rule
        .rhs
        .iter()
        .map(|s| match s {
            Symbol::Terminal(name) | Symbol::NonTerminal(name) => name.as_str(),
        })
        .collect();

    if rhs.is_empty() {
        format!("{} -> (empty)", rule.lhs)
    } else {
        format!("{} -> {}", rule.lhs, rhs.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_rule_with_empty_production() {
        let grammar = Grammar::new();
        // No rules, so format_rule with any index should return a fallback
        let result = format_rule(&grammar, 0);
        assert_eq!(result, "rule 0");
    }
}
