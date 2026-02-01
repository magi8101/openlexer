//! Subset Construction Algorithm.
//!
//! Converts a Non-deterministic Finite Automaton (NFA) into a
//! Deterministic Finite Automaton (DFA).

use crate::lexgen::nfa::{Nfa, Transition};
use crate::error::Result;
use std::collections::{HashSet, HashMap, BTreeSet};

/// A state in the DFA.
#[derive(Debug, Clone)]
pub struct DfaState {
    pub id: usize,
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
    /// Converts an NFA to a DFA using subset construction.
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
            transitions: HashMap::new(),
            is_accepting,
            rule_index,
            nfa_states: start_nfa_set.clone(),
        };

        dfa_states.push(start_dfa_state);
        states_map.insert(start_nfa_set, start_dfa_id);
        work_list.push(start_dfa_id);

        while let Some(current_dfa_id) = work_list.pop() {
            // Need to get inputs - find all possible char transitions from current set of NFA states
            let current_nfa_states = &dfa_states[current_dfa_id].nfa_states.clone();
            
            // Collect all symbols that trigger a transition from any state in the current set
            let mut inputs = HashSet::new();
            for &nfa_id in current_nfa_states {
                for trans in &nfa.states[nfa_id].transitions {
                    if let Transition::Char(c, _) = trans {
                        inputs.insert(*c);
                    }
                }
            }

            for &c in &inputs {
                // Move: Where do we go on input 'c'?
                let mut move_set = BTreeSet::new();
                for &nfa_id in current_nfa_states {
                    for trans in &nfa.states[nfa_id].transitions {
                        if let Transition::Char(ch, to) = trans {
                            if *ch == c {
                                move_set.insert(*to);
                            }
                        }
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
                        transitions: HashMap::new(),
                        is_accepting,
                        rule_index,
                        nfa_states: next_state_set.clone(),
                    });
                    
                    states_map.insert(next_state_set, new_id);
                    work_list.push(new_id);
                    new_id
                };

                // Add transition to DFA
                dfa_states[current_dfa_id].transitions.insert(c, next_state_id);
            }
        }

        Ok(Dfa {
            states: dfa_states,
            start_state: start_dfa_id,
        })
    }
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
