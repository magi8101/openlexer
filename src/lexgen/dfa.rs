//! Subset Construction Algorithm.
//!
//! Converts a Non-deterministic Finite Automaton (NFA) into a
//! Deterministic Finite Automaton (DFA).
//! Uses interval-based transitions for efficient Unicode handling.

use crate::error::Result;
use crate::lexgen::nfa::{Nfa, Transition};
use std::collections::{BTreeSet, HashMap};

/// A range-based transition: (start_codepoint, end_codepoint, target_state)
pub type RangeTransition = (u32, u32, usize);

/// A state in the DFA.
#[derive(Debug, Clone)]
pub struct DfaState {
    pub id: usize,
    /// Range-based transitions: Vec<(start_codepoint, end_codepoint, target_state)>
    pub range_transitions: Vec<RangeTransition>,
    /// Legacy char-based transitions for backward compatibility
    pub transitions: HashMap<char, usize>,
    pub is_accepting: bool,
    /// If this is an accepting state, which rule index it accepts.
    /// When multiple NFA accepting states are present, we use the lowest index (highest priority).
    pub rule_index: Option<usize>,
    /// Original NFA states this DFA state represents (for debugging/verification).
    pub nfa_states: BTreeSet<usize>,
}

/// Deterministic Finite Automaton.
#[derive(Debug, Clone)]
pub struct Dfa {
    pub states: Vec<DfaState>,
    pub start_state: usize,
}

impl Dfa {
    /// Converts an NFA to a DFA using subset construction with interval-based transitions.
    pub fn from_nfa(nfa: &Nfa) -> Result<Self> {
        let mut dfa_states = Vec::new();
        let mut states_map: HashMap<BTreeSet<usize>, usize> = HashMap::new();
        let mut work_list = Vec::new();

        // 1. Calculate epsilon closure of start state
        let start_nfa_set = epsilon_closure(nfa, &vec![nfa.start_state].into_iter().collect());

        let start_dfa_id = 0;
        let (is_accepting, rule_index) = compute_accepting_info(nfa, &start_nfa_set);
        let start_dfa_state = DfaState {
            id: start_dfa_id,
            range_transitions: Vec::new(),
            transitions: HashMap::new(),
            is_accepting,
            rule_index,
            nfa_states: start_nfa_set.clone(),
        };

        dfa_states.push(start_dfa_state);
        states_map.insert(start_nfa_set, start_dfa_id);
        work_list.push(start_dfa_id);

        while let Some(current_dfa_id) = work_list.pop() {
            let current_nfa_states = &dfa_states[current_dfa_id].nfa_states.clone();

            // Collect all transitions from the current NFA states
            let mut all_ranges: Vec<(u32, u32, usize)> = Vec::new(); // (start, end, nfa_target)
            let mut single_chars: Vec<(char, usize)> = Vec::new();

            for &nfa_id in current_nfa_states {
                for trans in &nfa.states[nfa_id].transitions {
                    match trans {
                        Transition::Char(c, target) => {
                            single_chars.push((*c, *target));
                        }
                        Transition::CharRange(start_cp, end_cp, target) => {
                            all_ranges.push((*start_cp, *end_cp, *target));
                        }
                        Transition::Epsilon(_) => {} // Already handled by epsilon closure
                    }
                }
            }

            // Convert single chars to ranges
            for (c, target) in single_chars {
                let cp = c as u32;
                all_ranges.push((cp, cp, target));
            }

            if all_ranges.is_empty() {
                continue;
            }

            // Compute alphabet intervals by partitioning the codepoint space
            let intervals = compute_alphabet_intervals(&all_ranges);

            for (interval_start, interval_end) in intervals {
                // For this interval, find all NFA targets that are reachable
                let mut move_set = BTreeSet::new();
                for &(range_start, range_end, nfa_target) in &all_ranges {
                    // Check if interval overlaps with this range
                    if interval_start <= range_end && interval_end >= range_start {
                        move_set.insert(nfa_target);
                    }
                }

                if move_set.is_empty() {
                    continue;
                }

                // Epsilon closure of the move set
                let next_state_set = epsilon_closure(nfa, &move_set);

                // Get or create DFA state for this set
                let next_state_id = if let Some(&id) = states_map.get(&next_state_set) {
                    id
                } else {
                    let new_id = dfa_states.len();
                    let (is_accepting, rule_index) = compute_accepting_info(nfa, &next_state_set);

                    dfa_states.push(DfaState {
                        id: new_id,
                        range_transitions: Vec::new(),
                        transitions: HashMap::new(),
                        is_accepting,
                        rule_index,
                        nfa_states: next_state_set.clone(),
                    });

                    states_map.insert(next_state_set, new_id);
                    work_list.push(new_id);
                    new_id
                };

                // Add range transition to DFA
                dfa_states[current_dfa_id].range_transitions.push((
                    interval_start,
                    interval_end,
                    next_state_id,
                ));

                // Also add to legacy char transitions for single-character ranges
                if interval_start == interval_end {
                    if let Some(c) = char::from_u32(interval_start) {
                        dfa_states[current_dfa_id]
                            .transitions
                            .insert(c, next_state_id);
                    }
                }
            }

            // Merge adjacent ranges going to the same state
            let merged_ranges = merge_dfa_ranges(&dfa_states[current_dfa_id].range_transitions);
            dfa_states[current_dfa_id].range_transitions = merged_ranges;
        }

        Ok(Dfa {
            states: dfa_states,
            start_state: start_dfa_id,
        })
    }
}

/// Computes non-overlapping intervals that partition the input space based on range boundaries.
fn compute_alphabet_intervals(ranges: &[(u32, u32, usize)]) -> Vec<(u32, u32)> {
    if ranges.is_empty() {
        return Vec::new();
    }

    // Collect all boundary points
    let mut boundaries: BTreeSet<u32> = BTreeSet::new();
    for &(start, end, _) in ranges {
        boundaries.insert(start);
        if end < u32::MAX {
            boundaries.insert(end + 1);
        }
    }

    let bounds: Vec<u32> = boundaries.into_iter().collect();
    let mut intervals = Vec::new();

    for i in 0..bounds.len() {
        let start = bounds[i];
        let end = if i + 1 < bounds.len() {
            bounds[i + 1] - 1
        } else {
            // Find max end in ranges
            ranges.iter().map(|r| r.1).max().unwrap_or(start)
        };

        if start <= end {
            // Check if this interval is actually covered by any range
            let covered = ranges.iter().any(|&(rs, re, _)| start >= rs && end <= re);
            if covered {
                intervals.push((start, end));
            }
        }
    }

    intervals
}

/// Merges adjacent DFA ranges that go to the same target state.
fn merge_dfa_ranges(ranges: &[RangeTransition]) -> Vec<RangeTransition> {
    if ranges.is_empty() {
        return Vec::new();
    }

    // Sort by (target, start)
    let mut sorted: Vec<RangeTransition> = ranges.to_vec();
    sorted.sort_by_key(|r| (r.2, r.0));

    let mut result: Vec<RangeTransition> = Vec::new();
    let mut current = sorted[0];

    for &(start, end, target) in &sorted[1..] {
        // Can merge if same target and adjacent
        if target == current.2
            && (current.1 >= start || (current.1 < u32::MAX && current.1 + 1 >= start))
        {
            // Merge
            current.1 = std::cmp::max(current.1, end);
        } else {
            result.push(current);
            current = (start, end, target);
        }
    }
    result.push(current);

    // Sort result by start position
    result.sort_by_key(|r| r.0);

    result
}

/// Computes whether a set of NFA states contains an accepting state,
/// and if so, returns the rule index with the highest priority (lowest index).
fn compute_accepting_info(nfa: &Nfa, nfa_states: &BTreeSet<usize>) -> (bool, Option<usize>) {
    let mut min_rule_index: Option<usize> = None;

    for &state_id in nfa_states {
        let state = &nfa.states[state_id];
        if state.is_accepting {
            if let Some(rule_idx) = state.rule_index {
                min_rule_index = Some(match min_rule_index {
                    Some(current_min) => current_min.min(rule_idx),
                    None => rule_idx,
                });
            }
        }
    }

    let is_accepting = min_rule_index.is_some();
    (is_accepting, min_rule_index)
}

/// Computes the set of states reachable via epsilon transitions.
fn epsilon_closure(nfa: &Nfa, states: &BTreeSet<usize>) -> BTreeSet<usize> {
    let mut closure = states.clone();
    let mut stack: Vec<usize> = states.iter().cloned().collect();

    while let Some(state_id) = stack.pop() {
        for trans in &nfa.states[state_id].transitions {
            if let Transition::Epsilon(to) = trans {
                if !closure.contains(to) {
                    closure.insert(*to);
                    stack.push(*to);
                }
            }
        }
    }

    closure
}
