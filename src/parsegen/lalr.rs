//! LALR(1) / SLR(1) Table Construction.
//!
//! Implements the core algorithms to build parsing tables from a grammar:
//! 1. Canonical Collection of LR(0) Items
//! 2. Closure and Goto operations
//! 3. Action/Goto Table population
//! 4. Precedence/Associativity conflict resolution
//!
//! Conflict Resolution (per Bison semantics):
//! - Shift/Reduce: Compare lookahead token precedence vs production precedence
//!   - Token prec > Rule prec => Shift
//!   - Rule prec > Token prec => Reduce
//!   - Equal prec => Use associativity: Left=Reduce, Right=Shift, NonAssoc=Error, PrecedenceOnly=Conflict
//! - Reduce/Reduce: First rule in grammar wins (lowest rule index)

use crate::error::Result;
use crate::parsegen::first::FirstFollow;
use crate::parsegen::grammar::{Assoc, Grammar, Rule, Symbol};
use std::collections::{BTreeSet, HashMap, HashSet};

/// Augment the grammar with S' -> S to create a unique accepting state.
/// The augmented rule becomes rule 0.
fn augment_grammar(grammar: &Grammar) -> Grammar {
    let augmented_start = format!("{}'", grammar.start_symbol);

    // Create augmented start rule: S' -> S
    let augmented_rule = Rule {
        lhs: augmented_start.clone(),
        rhs: vec![Symbol::NonTerminal(grammar.start_symbol.clone())],
        action: None,
        precedence_sym: None,
    };

    // Build new rules list with augmented rule first
    let mut new_rules = vec![augmented_rule];
    new_rules.extend(grammar.rules.iter().cloned());

    Grammar {
        tokens: grammar.tokens.clone(),
        start_symbol: augmented_start,
        rules: new_rules,
        precedence: grammar.precedence.clone(),
        union_fields: grammar.union_fields.clone(),
        raw_union_body: grammar.raw_union_body.clone(),
        token_types: grammar.token_types.clone(),
        nterm_types: grammar.nterm_types.clone(),
        glr_mode: grammar.glr_mode,
        locations: grammar.locations,
        destructors: grammar.destructors.clone(),
        error_verbose: grammar.error_verbose,
        lac_enabled: grammar.lac_enabled,
        prologue: grammar.prologue.clone(),
    }
}

/// LR(0) Item: A production with a dot position.
/// A -> alpha . beta
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Item {
    pub rule_index: usize,
    pub dot: usize,
}

/// A set of items representing a parser state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub id: usize,
    pub items: BTreeSet<Item>,
}

#[derive(Debug, Clone)]
pub struct ParsingTable {
    /// State -> (Symbol -> Action)
    pub action: HashMap<usize, HashMap<String, Action>>,
    /// State -> (NonTerminal -> State)
    pub goto: HashMap<usize, HashMap<String, usize>>,
    /// Number of shift-reduce conflicts encountered
    pub shift_reduce_conflicts: usize,
    /// Number of reduce-reduce conflicts encountered
    pub reduce_reduce_conflicts: usize,
    /// Detailed conflict messages for diagnostics
    pub conflict_messages: Vec<String>,
    /// GLR: Alternative actions at conflict points (state -> symbol -> list of alt actions)
    /// When a conflict is resolved, the "losing" action is stored here for GLR to access.
    pub glr_conflict_actions: HashMap<usize, HashMap<String, Vec<Action>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Shift(usize),
    Reduce(usize), // Rule index
    Accept,
}

impl ParsingTable {
    pub fn build(grammar: &Grammar) -> Result<Self> {
        // Augment the grammar with S' -> S $ (rule 0)
        let augmented = augment_grammar(grammar);
        let ff = FirstFollow::new(&augmented);

        // Build Canonical Collection of LR(0) states
        let (states, transitions) = build_lr0_collection(&augmented)?;

        let mut table = Self {
            action: HashMap::new(),
            goto: HashMap::new(),
            shift_reduce_conflicts: 0,
            reduce_reduce_conflicts: 0,
            conflict_messages: Vec::new(),
            glr_conflict_actions: HashMap::new(),
        };

        // Populate Action and Goto tables (SLR(1) approach)
        for state in &states {
            // Transitions (Shift and Goto)
            if let Some(trans) = transitions.get(&state.id) {
                for (sym, next_state_id) in trans {
                    if augmented.tokens.contains(sym) || sym == "$" {
                        // Shift
                        add_action(
                            &mut table,
                            state.id,
                            sym,
                            Action::Shift(*next_state_id),
                            &augmented,
                        );
                    } else {
                        // Goto
                        table
                            .goto
                            .entry(state.id)
                            .or_default()
                            .insert(sym.clone(), *next_state_id);
                    }
                }
            }

            // Reductions
            for item in &state.items {
                let rule = &augmented.rules[item.rule_index];

                // If dot is at the end: A -> alpha .
                if item.dot == rule.rhs.len() {
                    // Augmented start rule: S' -> S . means Accept
                    if item.rule_index == 0 {
                        // Accept on EOF ($)
                        add_action(&mut table, state.id, "$", Action::Accept, &augmented);
                    } else {
                        // Reduce A -> alpha
                        // SLR(1): For all terminal a in FOLLOW(A)
                        if let Some(follow) = ff.follow.get(&rule.lhs) {
                            for term in follow {
                                // Adjust rule index: subtract 1 because we added the augmented rule at position 0
                                add_action(
                                    &mut table,
                                    state.id,
                                    term,
                                    Action::Reduce(item.rule_index - 1),
                                    &augmented,
                                );
                            }
                        }
                    }
                }
            }
        }

        // Report conflicts
        if table.shift_reduce_conflicts > 0 || table.reduce_reduce_conflicts > 0 {
            eprintln!(
                "Parser has {} shift/reduce and {} reduce/reduce conflicts",
                table.shift_reduce_conflicts, table.reduce_reduce_conflicts
            );
            for msg in &table.conflict_messages {
                eprintln!("  {}", msg);
            }
        }

        Ok(table)
    }
}

/// Add action with proper conflict detection and resolution.
///
/// Shift/Reduce conflicts:
/// - Compare lookahead token precedence vs rule precedence
/// - Higher precedence wins
/// - Equal precedence uses associativity: Left=Reduce, Right=Shift, NonAssoc=Error
///
/// Reduce/Reduce conflicts:
/// - Lower rule index wins (first rule in grammar)
fn add_action(
    table: &mut ParsingTable,
    state_id: usize,
    sym: &str,
    action: Action,
    grammar: &Grammar,
) {
    let row = table.action.entry(state_id).or_default();

    if let Some(existing) = row.get(sym).cloned() {
        match (&existing, &action) {
            // Shift/Reduce conflict
            (Action::Shift(shift_state), Action::Reduce(rule_idx)) => {
                resolve_shift_reduce(table, state_id, sym, *shift_state, *rule_idx, grammar);
            }
            (Action::Reduce(rule_idx), Action::Shift(shift_state)) => {
                resolve_shift_reduce(table, state_id, sym, *shift_state, *rule_idx, grammar);
            }

            // Reduce/Reduce conflict
            (Action::Reduce(existing_rule), Action::Reduce(new_rule)) => {
                let existing_rule = *existing_rule;
                let new_rule = *new_rule;
                // First rule in grammar wins (lower index)
                let (winner, loser) = if new_rule < existing_rule {
                    row.insert(sym.to_string(), action.clone());
                    (Action::Reduce(new_rule), Action::Reduce(existing_rule))
                } else {
                    (Action::Reduce(existing_rule), Action::Reduce(new_rule))
                };
                // Save the losing action for GLR
                table
                    .glr_conflict_actions
                    .entry(state_id)
                    .or_default()
                    .entry(sym.to_string())
                    .or_default()
                    .push(loser);
                // Also store the winner if not already
                let alts = table
                    .glr_conflict_actions
                    .entry(state_id)
                    .or_default()
                    .entry(sym.to_string())
                    .or_default();
                if !alts.contains(&winner) {
                    alts.insert(0, winner);
                }
                // Always count as conflict
                table.reduce_reduce_conflicts += 1;
                table.conflict_messages.push(format!(
                    "State {}: reduce/reduce conflict on '{}' between rule {} and rule {}",
                    state_id, sym, existing_rule, new_rule
                ));
            }

            // Accept takes precedence over everything
            (_, Action::Accept) | (Action::Accept, _) => {
                row.insert(sym.to_string(), Action::Accept);
            }

            // Shift/Shift should not happen in a correct grammar
            (Action::Shift(_), Action::Shift(_)) => {
                // Keep existing, this indicates a grammar problem
            }
        }
    } else {
        row.insert(sym.to_string(), action);
    }
}

/// Resolve a shift/reduce conflict using precedence and associativity.
fn resolve_shift_reduce(
    table: &mut ParsingTable,
    state_id: usize,
    sym: &str,
    shift_state: usize,
    rule_idx: usize,
    grammar: &Grammar,
) {
    let row = table.action.entry(state_id).or_default();
    let sym_prec = get_symbol_prec(grammar, sym);
    let rule_prec = get_rule_prec(grammar, rule_idx);

    // Helper to save both actions for GLR conflict handling
    let save_glr_conflict = |table: &mut ParsingTable, winner: Action, loser: Action| {
        let alts = table
            .glr_conflict_actions
            .entry(state_id)
            .or_default()
            .entry(sym.to_string())
            .or_default();
        if !alts.contains(&winner) {
            alts.push(winner);
        }
        if !alts.contains(&loser) {
            alts.push(loser);
        }
    };

    match (sym_prec, rule_prec) {
        (Some((sym_level, sym_assoc)), Some((rule_level, _))) => {
            if sym_level > rule_level {
                // Token has higher precedence: Shift wins
                row.insert(sym.to_string(), Action::Shift(shift_state));
                // Save for GLR (precedence-resolved, but GLR may want both)
                save_glr_conflict(table, Action::Shift(shift_state), Action::Reduce(rule_idx));
            } else if rule_level > sym_level {
                // Rule has higher precedence: Reduce wins
                row.insert(sym.to_string(), Action::Reduce(rule_idx));
                save_glr_conflict(table, Action::Reduce(rule_idx), Action::Shift(shift_state));
            } else {
                // Equal precedence: use associativity of the token
                match sym_assoc {
                    Assoc::Left => {
                        // Left associative: reduce
                        row.insert(sym.to_string(), Action::Reduce(rule_idx));
                        save_glr_conflict(
                            table,
                            Action::Reduce(rule_idx),
                            Action::Shift(shift_state),
                        );
                    }
                    Assoc::Right => {
                        // Right associative: shift
                        row.insert(sym.to_string(), Action::Shift(shift_state));
                        save_glr_conflict(
                            table,
                            Action::Shift(shift_state),
                            Action::Reduce(rule_idx),
                        );
                    }
                    Assoc::NonAssoc => {
                        // Non-associative: syntax error if used associatively
                        // Remove both actions - this will cause parse error
                        row.remove(sym);
                        table.conflict_messages.push(format!(
                            "State {}: %nonassoc conflict on '{}' - using operator associatively is an error",
                            state_id, sym
                        ));
                    }
                    Assoc::PrecedenceOnly => {
                        // No associativity defined: unresolved conflict
                        // Default: shift (traditional yacc behavior)
                        row.insert(sym.to_string(), Action::Shift(shift_state));
                        save_glr_conflict(
                            table,
                            Action::Shift(shift_state),
                            Action::Reduce(rule_idx),
                        );
                        table.shift_reduce_conflicts += 1;
                        table.conflict_messages.push(format!(
                            "State {}: shift/reduce conflict on '{}' (rule {}) - %precedence has no associativity, defaulting to shift",
                            state_id, sym, rule_idx
                        ));
                    }
                }
            }
            // Conflict was resolved by precedence - don't count it
        }
        _ => {
            // No precedence info: unresolved conflict, default to shift
            row.insert(sym.to_string(), Action::Shift(shift_state));
            // Save both for GLR
            save_glr_conflict(table, Action::Shift(shift_state), Action::Reduce(rule_idx));
            table.shift_reduce_conflicts += 1;
            table.conflict_messages.push(format!(
                "State {}: shift/reduce conflict on '{}' (rule {}) - no precedence, defaulting to shift",
                state_id, sym, rule_idx
            ));
        }
    }
}

fn get_symbol_prec(grammar: &Grammar, sym: &str) -> Option<(usize, Assoc)> {
    for (i, p) in grammar.precedence.iter().enumerate() {
        if p.symbols.contains(&sym.to_string()) {
            return Some((i, p.assoc.clone()));
        }
    }
    None
}

fn get_rule_prec(grammar: &Grammar, rule_idx: usize) -> Option<(usize, Assoc)> {
    // rule_idx is in original grammar numbering (0-based before augmentation)
    // In augmented grammar, rules are shifted by 1 (augmented start rule is at index 0)
    // So original rule N is at augmented index N+1
    let augmented_idx = rule_idx + 1;
    if augmented_idx >= grammar.rules.len() {
        return None;
    }
    let rule = &grammar.rules[augmented_idx];

    // Explicit %prec
    if let Some(sym) = &rule.precedence_sym {
        return get_symbol_prec(grammar, sym);
    }

    // Find rightmost terminal
    for sym in rule.rhs.iter().rev() {
        if let Symbol::Terminal(t) = sym {
            return get_symbol_prec(grammar, t);
        }
    }
    None
}

// --- LR(0) Construction Algorithms ---

fn build_lr0_collection(
    grammar: &Grammar,
) -> Result<(Vec<State>, HashMap<usize, HashMap<String, usize>>)> {
    let mut states = Vec::new();
    let mut transitions: HashMap<usize, HashMap<String, usize>> = HashMap::new();
    let mut state_map: HashMap<BTreeSet<Item>, usize> = HashMap::new(); // Core -> ID
    let mut work_list = Vec::new();

    // Initial item: S -> . alpha (Rule 0)
    let initial_core: BTreeSet<Item> = [Item {
        rule_index: 0,
        dot: 0,
    }]
    .into();
    let initial_set = closure(grammar, &initial_core);

    let start_state = State {
        id: 0,
        items: initial_set.clone(),
    };
    states.push(start_state);
    state_map.insert(initial_set, 0);
    work_list.push(0);

    while let Some(state_id) = work_list.pop() {
        let state_items = states[state_id].items.clone();
        let state_items_ref = &state_items;

        // Collect symbols after dots
        let mut symbols = HashSet::new();
        for item in state_items_ref {
            if item.dot < grammar.rules[item.rule_index].rhs.len() {
                let sym = &grammar.rules[item.rule_index].rhs[item.dot];
                let sym_name = match sym {
                    Symbol::Terminal(s) => s,
                    Symbol::NonTerminal(s) => s,
                };
                symbols.insert(sym_name.clone());
            }
        }

        for sym in symbols {
            let next_core = goto(grammar, state_items_ref, &sym);
            if next_core.is_empty() {
                continue;
            }

            let next_set = closure(grammar, &next_core);

            let next_state_id = if let Some(&id) = state_map.get(&next_set) {
                id
            } else {
                let id = states.len();
                states.push(State {
                    id,
                    items: next_set.clone(),
                });
                state_map.insert(next_set, id);
                work_list.push(id);
                id
            };

            transitions
                .entry(state_id)
                .or_default()
                .insert(sym.clone(), next_state_id);
        }
    }

    Ok((states, transitions))
}

fn closure(grammar: &Grammar, core: &BTreeSet<Item>) -> BTreeSet<Item> {
    let mut set = core.clone();
    let mut work_list: Vec<Item> = core.iter().cloned().collect();

    while let Some(item) = work_list.pop() {
        if item.dot < grammar.rules[item.rule_index].rhs.len() {
            let sym = &grammar.rules[item.rule_index].rhs[item.dot];
            if let Symbol::NonTerminal(nt) = sym {
                // Expand NonTerminal: Add B -> . gamma for all rules for B
                for (i, rule) in grammar.rules.iter().enumerate() {
                    if &rule.lhs == nt {
                        let new_item = Item {
                            rule_index: i,
                            dot: 0,
                        };
                        if !set.contains(&new_item) {
                            set.insert(new_item.clone());
                            work_list.push(new_item);
                        }
                    }
                }
            }
        }
    }
    set
}

fn goto(grammar: &Grammar, items: &BTreeSet<Item>, sym: &str) -> BTreeSet<Item> {
    let mut next = BTreeSet::new();
    for item in items {
        if item.dot < grammar.rules[item.rule_index].rhs.len() {
            let current_sym = &grammar.rules[item.rule_index].rhs[item.dot];
            let current_name = match current_sym {
                Symbol::Terminal(s) => s,
                Symbol::NonTerminal(s) => s,
            };

            if current_name == sym {
                next.insert(Item {
                    rule_index: item.rule_index,
                    dot: item.dot + 1,
                });
            }
        }
    }
    next // Not full closure, just the kernel. Caller applies closure.
}
