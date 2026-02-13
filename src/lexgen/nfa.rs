//! Thompson's Construction Algorithm.
//!
//! Converts a Regex AST into a Non-deterministic Finite Automaton (NFA).

use crate::error::Result;
use crate::lexgen::regex::{CharClass, Regex};
use crate::lexgen::rules::LexerSpec;

/// A state in the NFA.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub id: usize,
    pub transitions: Vec<Transition>,
    pub is_accepting: bool,
    /// If this is an accepting state, which rule index it accepts (for priority).
    /// Lower index = higher priority (first match wins for equal length).
    pub rule_index: Option<usize>,
}

/// A transition between states.
#[derive(Debug, Clone, PartialEq)]
pub enum Transition {
    /// Transition on a specific character.
    Char(char, usize),
    /// Transition on a codepoint range (inclusive). For Unicode support.
    CharRange(u32, u32, usize),
    /// Epsilon transition (no input consumed).
    Epsilon(usize),
}

/// Non-deterministic Finite Automaton.
#[derive(Debug, Clone)]
pub struct Nfa {
    pub states: Vec<State>,
    pub start_state: usize,
    /// For single-regex NFA, this is the single accept state.
    /// For multi-rule NFA, this is unused (each rule has its own accept state).
    pub accept_state: usize,
    /// Maps rule index to its accepting state ID (for multi-rule NFAs).
    pub accept_states: Vec<usize>,
}

impl Nfa {
    pub fn new() -> Self {
        Self {
            states: Vec::new(),
            start_state: 0,
            accept_state: 0,
            accept_states: Vec::new(),
        }
    }

    /// Adds a new state and returns its ID.
    fn add_state(&mut self, is_accepting: bool) -> usize {
        let id = self.states.len();
        self.states.push(State {
            id,
            transitions: Vec::new(),
            is_accepting,
            rule_index: None,
        });
        id
    }

    /// Adds a character transition from `from` to `to`.
    fn add_char_transition(&mut self, from: usize, to: usize, c: char) {
        self.states[from].transitions.push(Transition::Char(c, to));
    }

    /// Adds a codepoint range transition from `from` to `to`.
    fn add_char_range_transition(&mut self, from: usize, to: usize, start_cp: u32, end_cp: u32) {
        self.states[from]
            .transitions
            .push(Transition::CharRange(start_cp, end_cp, to));
    }

    /// Adds an epsilon transition from `from` to `to`.
    fn add_epsilon_transition(&mut self, from: usize, to: usize) {
        self.states[from].transitions.push(Transition::Epsilon(to));
    }

    /// Converts a Regex AST to NFA using Thompson's construction.
    pub fn from_regex(regex: &Regex) -> Result<Self> {
        let mut nfa = Nfa::new();
        let (start, end) = nfa.build_subgraph(regex)?;
        nfa.start_state = start;
        nfa.accept_state = end;
        nfa.states[end].is_accepting = true;
        nfa.states[end].rule_index = Some(0);
        nfa.accept_states.push(end);
        Ok(nfa)
    }

    /// Builds a combined NFA from a lexer specification with multiple rules.
    /// Creates a new start state with epsilon transitions to each rule's NFA.
    pub fn from_lexer_spec(spec: &LexerSpec) -> Result<Self> {
        let mut nfa = Nfa::new();

        // Create a unified start state
        let unified_start = nfa.add_state(false);
        nfa.start_state = unified_start;

        // Build NFA for each rule and connect to unified start
        for (rule_idx, rule) in spec.rules.iter().enumerate() {
            let (rule_start, rule_end) = nfa.build_subgraph(&rule.regex.root)?;

            // Connect unified start to this rule's start
            nfa.add_epsilon_transition(unified_start, rule_start);

            // Mark the rule's end state as accepting with the rule index
            nfa.states[rule_end].is_accepting = true;
            nfa.states[rule_end].rule_index = Some(rule_idx);
            nfa.accept_states.push(rule_end);
        }

        Ok(nfa)
    }

    /// Builds an NFA for a specific start condition.
    /// Only includes rules that are active in the given condition.
    /// - Rules with no start conditions are included if condition_type is Inclusive
    /// - Rules with explicit start conditions are included if condition matches
    /// - The <*> condition (represented by having all conditions) is handled by the caller
    pub fn from_lexer_spec_for_condition(spec: &LexerSpec, condition: &str) -> Result<Self> {
        use crate::lexgen::rules::StartConditionType;

        let mut nfa = Nfa::new();

        // Create a unified start state
        let unified_start = nfa.add_state(false);
        nfa.start_state = unified_start;

        // Determine if this condition is inclusive or exclusive
        let condition_type = spec
            .start_conditions
            .get(condition)
            .copied()
            .unwrap_or(StartConditionType::Inclusive);

        // Build NFA for each rule that's active in this condition
        for (rule_idx, rule) in spec.rules.iter().enumerate() {
            let is_active = if rule.start_conditions.is_empty() {
                // Rule has no explicit conditions - active only in inclusive conditions
                condition_type == StartConditionType::Inclusive
            } else {
                // Rule has explicit conditions - check if our condition is listed
                rule.start_conditions.iter().any(|c| c == condition)
            };

            if !is_active {
                continue;
            }

            let (rule_start, rule_end) = nfa.build_subgraph(&rule.regex.root)?;

            // Connect unified start to this rule's start
            nfa.add_epsilon_transition(unified_start, rule_start);

            // Mark the rule's end state as accepting with the rule index
            nfa.states[rule_end].is_accepting = true;
            nfa.states[rule_end].rule_index = Some(rule_idx);
            nfa.accept_states.push(rule_end);
        }

        Ok(nfa)
    }

    /// Recursively builds NFA subgraph for the regex.
    /// Returns (start_state_id, end_state_id).
    fn build_subgraph(&mut self, regex: &Regex) -> Result<(usize, usize)> {
        match regex {
            Regex::Empty => {
                let start = self.add_state(false);
                let end = self.add_state(false);
                self.add_epsilon_transition(start, end);
                Ok((start, end))
            }
            Regex::Literal(c) => {
                let start = self.add_state(false);
                let end = self.add_state(false);
                self.add_char_transition(start, end, *c);
                Ok((start, end))
            }
            Regex::CharClass(class) => {
                // Create transitions for each character in the class
                self.build_char_class(class)
            }
            Regex::Dot => {
                // Dot matches any ASCII character except newline
                self.build_dot()
            }
            Regex::Concat(lhs, rhs) => {
                let (start1, end1) = self.build_subgraph(lhs)?;
                let (start2, end2) = self.build_subgraph(rhs)?;

                // Connect end of lhs to start of rhs with epsilon
                self.add_epsilon_transition(end1, start2);

                // Result is start1 -> ... -> end1 -> start2 -> ... -> end2
                Ok((start1, end2))
            }
            Regex::Union(lhs, rhs) => {
                let (start1, end1) = self.build_subgraph(lhs)?;
                let (start2, end2) = self.build_subgraph(rhs)?;

                let new_start = self.add_state(false);
                let new_end = self.add_state(false);

                // Split from new start
                self.add_epsilon_transition(new_start, start1);
                self.add_epsilon_transition(new_start, start2);

                // Join to new end
                self.add_epsilon_transition(end1, new_end);
                self.add_epsilon_transition(end2, new_end);

                Ok((new_start, new_end))
            }
            Regex::Star(inner) => {
                let (start, end) = self.build_subgraph(inner)?;

                let new_start = self.add_state(false);
                let new_end = self.add_state(false);

                // 0 or 1 entry
                self.add_epsilon_transition(new_start, start);
                self.add_epsilon_transition(new_start, new_end); // 0 matches

                // Loop back
                self.add_epsilon_transition(end, start);

                // Exit
                self.add_epsilon_transition(end, new_end);

                Ok((new_start, new_end))
            }
            Regex::Plus(inner) => {
                // a+ is aa*
                let (start, end) = self.build_subgraph(inner)?;

                let new_start = self.add_state(false);
                let new_end = self.add_state(false);

                // Must enter at least once
                self.add_epsilon_transition(new_start, start);

                // Loop back
                self.add_epsilon_transition(end, start);

                // Exit
                self.add_epsilon_transition(end, new_end);

                Ok((new_start, new_end))
            }
            Regex::Question(inner) => {
                // a? is union of a and epsilon
                let (start, end) = self.build_subgraph(inner)?;

                let new_start = self.add_state(false);
                let new_end = self.add_state(false); // Can reuse end actually, but let's be strict Thompson

                self.add_epsilon_transition(new_start, start);
                self.add_epsilon_transition(new_start, new_end); // Skip

                self.add_epsilon_transition(end, new_end);

                Ok((new_start, new_end))
            }
        }
    }

    /// Builds an NFA subgraph for a character class.
    /// Uses range-based transitions to avoid exploding state space for Unicode.
    fn build_char_class(&mut self, class: &CharClass) -> Result<(usize, usize)> {
        let start = self.add_state(false);
        let end = self.add_state(false);

        // Collect all ranges (both ASCII CharRange and Unicode ranges)
        let mut all_ranges: Vec<(u32, u32)> = Vec::new();

        // Convert ASCII CharRange to u32 ranges
        for range in &class.ranges {
            match range {
                super::regex::CharRange::Single(c) => {
                    let cp = *c as u32;
                    all_ranges.push((cp, cp));
                }
                super::regex::CharRange::Range(start_c, end_c) => {
                    all_ranges.push((*start_c as u32, *end_c as u32));
                }
            }
        }

        // Add Unicode property ranges
        for range in &class.unicode_ranges {
            all_ranges.push((*range.start(), *range.end()));
        }

        // Handle negation by computing complement
        let ranges_to_use = if class.negated {
            compute_complement_ranges(&all_ranges)
        } else {
            // Sort and merge overlapping ranges for efficiency
            merge_ranges(&all_ranges)
        };

        // Add range transitions
        for (range_start, range_end) in ranges_to_use {
            self.add_char_range_transition(start, end, range_start, range_end);
        }

        Ok((start, end))
    }

    /// Builds an NFA subgraph for the dot metacharacter.
    /// Matches any ASCII character except newline.
    fn build_dot(&mut self) -> Result<(usize, usize)> {
        let start = self.add_state(false);
        let end = self.add_state(false);

        // Add transitions for all printable ASCII and common control chars except \n
        for code in 0u8..128u8 {
            let c = code as char;
            if c != '\n' {
                self.add_char_transition(start, end, c);
            }
        }

        Ok((start, end))
    }
}
/// Merges overlapping and adjacent ranges into a minimal set.
fn merge_ranges(ranges: &[(u32, u32)]) -> Vec<(u32, u32)> {
    if ranges.is_empty() {
        return Vec::new();
    }

    let mut sorted: Vec<(u32, u32)> = ranges.to_vec();
    sorted.sort_by_key(|r| r.0);

    let mut result: Vec<(u32, u32)> = Vec::new();
    let mut current = sorted[0];

    for &(start, end) in &sorted[1..] {
        // Check if overlapping or adjacent (current.1 + 1 >= start)
        if current.1 >= start || (current.1 < u32::MAX && current.1 + 1 >= start) {
            // Merge
            current.1 = std::cmp::max(current.1, end);
        } else {
            result.push(current);
            current = (start, end);
        }
    }
    result.push(current);

    result
}

/// Computes the complement of a set of ranges within Unicode BMP (0x0000 - 0xFFFF).
/// Excludes surrogate pairs (0xD800 - 0xDFFF) which are invalid Unicode scalar values.
fn compute_complement_ranges(ranges: &[(u32, u32)]) -> Vec<(u32, u32)> {
    let merged = merge_ranges(ranges);
    let mut complement: Vec<(u32, u32)> = Vec::new();

    // Unicode valid ranges: 0x0000 - 0xD7FF and 0xE000 - 0x10FFFF
    // For now, limit to BMP excluding surrogates
    let valid_ranges = vec![(0u32, 0xD7FFu32), (0xE000u32, 0xFFFFu32)];

    for (valid_start, valid_end) in valid_ranges {
        let mut pos = valid_start;

        for &(ex_start, ex_end) in &merged {
            // Skip ranges outside current valid range
            if ex_end < valid_start || ex_start > valid_end {
                continue;
            }

            let ex_start_clamped = std::cmp::max(ex_start, valid_start);
            let ex_end_clamped = std::cmp::min(ex_end, valid_end);

            if pos < ex_start_clamped {
                complement.push((pos, ex_start_clamped - 1));
            }
            pos = ex_end_clamped.saturating_add(1);
        }

        if pos <= valid_end {
            complement.push((pos, valid_end));
        }
    }

    complement
}
