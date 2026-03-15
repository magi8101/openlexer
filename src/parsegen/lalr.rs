//! LALR(1) Table Construction using DeRemer-Pennello Algorithm.
//!
//! Implements efficient LALR(1) parsing table construction:
//! 1. Canonical Collection of LR(0) Items (same as SLR)
//! 2. LALR(1) Lookahead Computation via READS and INCLUDES relations
//! 3. Action/Goto Table population with precise lookaheads
//! 4. Precedence/Associativity conflict resolution
//!
//! The key difference from SLR(1):
//! - SLR(1) uses FOLLOW(A) for all reductions of A -> alpha
//! - LALR(1) computes context-specific lookaheads for each (state, production) pair
//!
//! Algorithm Overview (DeRemer & Pennello, 1982):
//! 1. Build LR(0) automaton
//! 2. Compute Direct Read (DR) sets: terminals readable after nonterminal transitions
//! 3. Compute READS relation: (p,A) READS (r,C) if goto(p,A)=r has [C->.gamma] and C =>* epsilon
//! 4. Compute INCLUDES relation: (p,A) INCLUDES (p',B) if B -> beta A gamma where gamma =>* epsilon
//! 5. Compute Read sets via transitive closure of READS over DR
//! 6. Compute Follow sets (LA) via transitive closure of INCLUDES over Read
//! 7. For reduction [A -> alpha.] in state s, LA(s, A -> alpha) is the lookahead set
//!
//! Conflict Resolution (per Bison semantics):
//! - Shift/Reduce: Compare lookahead token precedence vs production precedence
//!   - Token prec > Rule prec => Shift
//!   - Rule prec > Token prec => Reduce
//!   - Equal prec => Use associativity: Left=Reduce, Right=Shift, NonAssoc=Error
//! - Reduce/Reduce: First rule in grammar wins (lowest rule index)
//!
//! References:
//! - DeRemer, F. & Pennello, T. (1982). "Efficient Computation of LALR(1) Look-Ahead Sets"
//! - Aho, Sethi, Ullman (1986). "Compilers: Principles, Techniques, and Tools" (Dragon Book)

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
        epilogue: grammar.epilogue.clone(),
        token_literals: grammar.token_literals.clone(),
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

        // Compute LALR(1) lookaheads using DeRemer-Pennello algorithm
        let lalr = LalrLookaheads::compute(&augmented, &states, &transitions, &ff);

        let mut table = Self {
            action: HashMap::new(),
            goto: HashMap::new(),
            shift_reduce_conflicts: 0,
            reduce_reduce_conflicts: 0,
            conflict_messages: Vec::new(),
            glr_conflict_actions: HashMap::new(),
        };

        // Populate Action and Goto tables using LALR(1) lookaheads
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

            // Reductions using LALR(1) lookaheads
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
                        // LALR(1): Use computed lookahead set for this specific (state, rule) pair
                        let lookahead = lalr.get_lookahead(state.id, item.rule_index);

                        // If LALR lookahead computation found lookaheads, use them
                        // Otherwise fall back to FOLLOW set (for robustness)
                        let terms: Vec<String> = if let Some(la) = lookahead {
                            la.iter().cloned().collect()
                        } else {
                            // Fallback to SLR(1) FOLLOW set
                            ff.follow
                                .get(&rule.lhs)
                                .cloned()
                                .unwrap_or_default()
                                .into_iter()
                                .collect()
                        };

                        for term in terms {
                            // Adjust rule index: subtract 1 because we added the augmented rule at position 0
                            add_action(
                                &mut table,
                                state.id,
                                &term,
                                Action::Reduce(item.rule_index - 1),
                                &augmented,
                            );
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

// =============================================================================
// LALR(1) Lookahead Computation (DeRemer-Pennello Algorithm)
// =============================================================================

/// A nonterminal transition in the LR(0) automaton: (state_id, nonterminal_name)
/// This represents moving from state p on nonterminal A to some state q = goto(p, A).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct NonterminalTransition {
    state: usize,
    nonterminal: String,
}

/// Reduction item identifier: (state_id, rule_index)
/// Represents the reduction A -> alpha in state s.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct ReductionItem {
    state: usize,
    rule_index: usize,
}

/// LALR(1) lookahead computation context.
/// Implements the DeRemer-Pennello algorithm for efficient LALR(1) lookahead computation.
struct LalrLookaheads {
    /// Set of nonterminals that can derive epsilon.
    nullable: HashSet<String>,

    /// Direct Read (DR) sets: for each (state, nonterminal) transition,
    /// the set of terminals that can be read immediately after the transition.
    /// DR(p, A) = { t | goto(p, A) has item [B -> alpha.t beta] }
    direct_read: HashMap<NonterminalTransition, HashSet<String>>,

    /// READS relation: (p, A) READS (r, C) if:
    /// - r = goto(p, A)
    /// - r contains [C -> .gamma] in its closure
    /// - C =>* epsilon (C is nullable)
    reads: HashMap<NonterminalTransition, HashSet<NonterminalTransition>>,

    /// Read sets: transitive closure of DR over READS.
    /// Read(p, A) = DR(p, A) union { Read(r, C) | (p, A) READS (r, C) }
    read_sets: HashMap<NonterminalTransition, HashSet<String>>,

    /// INCLUDES relation: (p, A) INCLUDES (p', B) if:
    /// There exists a production B -> beta A gamma where:
    /// - Following beta from p' leads through transitions to state p
    /// - gamma =>* epsilon
    includes: HashMap<NonterminalTransition, HashSet<NonterminalTransition>>,

    /// LOOKBACK relation: maps reduction items to the nonterminal transitions
    /// that "cause" them.
    /// (s, A -> alpha) LOOKBACK (p, A) if goto(p, A, alpha) = s
    /// i.e., there's a path from p reading A then alpha that ends at s
    lookback: HashMap<ReductionItem, HashSet<NonterminalTransition>>,

    /// Final LA (lookahead) sets for each reduction item.
    /// LA(s, A -> alpha) = union { Follow(p, A) | (s, A -> alpha) LOOKBACK (p, A) }
    lookaheads: HashMap<ReductionItem, HashSet<String>>,
}

impl LalrLookaheads {
    /// Compute LALR(1) lookaheads for all reduction items.
    fn compute(
        grammar: &Grammar,
        states: &[State],
        transitions: &HashMap<usize, HashMap<String, usize>>,
        ff: &FirstFollow,
    ) -> Self {
        let mut ctx = Self {
            nullable: HashSet::new(),
            direct_read: HashMap::new(),
            reads: HashMap::new(),
            read_sets: HashMap::new(),
            includes: HashMap::new(),
            lookback: HashMap::new(),
            lookaheads: HashMap::new(),
        };

        // Step 1: Compute which nonterminals are nullable
        ctx.compute_nullable(grammar, ff);

        // Step 2: Compute Direct Read sets
        ctx.compute_direct_read(grammar, states, transitions);

        // Step 3: Compute READS relation
        ctx.compute_reads_relation(grammar, states, transitions);

        // Step 4: Compute Read sets (transitive closure over READS)
        ctx.compute_read_sets(transitions);

        // Step 5: Compute INCLUDES and LOOKBACK relations
        ctx.compute_includes_and_lookback(grammar, states, transitions);

        // Step 6: Compute final lookahead sets (Follow sets via INCLUDES, then LOOKBACK)
        ctx.compute_final_lookaheads(transitions);

        ctx
    }

    /// Compute which nonterminals can derive epsilon.
    fn compute_nullable(&mut self, grammar: &Grammar, ff: &FirstFollow) {
        // A nonterminal is nullable if FIRST contains EPSILON
        for (nt, first_set) in &ff.first {
            if first_set.contains("EPSILON") {
                self.nullable.insert(nt.clone());
            }
        }

        // Also check for rules with empty RHS directly
        for rule in &grammar.rules {
            if rule.rhs.is_empty() {
                self.nullable.insert(rule.lhs.clone());
            }
        }
    }

    /// Compute Direct Read sets.
    /// DR(p, A) = set of terminals t such that goto(p, A) contains an item with dot before t.
    fn compute_direct_read(
        &mut self,
        grammar: &Grammar,
        states: &[State],
        transitions: &HashMap<usize, HashMap<String, usize>>,
    ) {
        for state in states {
            if let Some(state_trans) = transitions.get(&state.id) {
                for (sym, target_state_id) in state_trans {
                    // Only consider nonterminal transitions
                    if grammar.tokens.contains(sym) || sym == "$" {
                        continue;
                    }

                    let nt_trans = NonterminalTransition {
                        state: state.id,
                        nonterminal: sym.clone(),
                    };

                    let target_state = &states[*target_state_id];
                    let mut dr = HashSet::new();

                    // Find all terminals that appear immediately after the dot
                    for item in &target_state.items {
                        let rule = &grammar.rules[item.rule_index];
                        if item.dot < rule.rhs.len() {
                            if let Symbol::Terminal(t) = &rule.rhs[item.dot] {
                                dr.insert(t.clone());
                            }
                        }
                    }

                    // Also include $ if this is the augmented start transition
                    // (handled by checking if target contains accept item)
                    for item in &target_state.items {
                        if item.rule_index == 0 && item.dot == grammar.rules[0].rhs.len() {
                            dr.insert("$".to_string());
                        }
                    }

                    self.direct_read.insert(nt_trans, dr);
                }
            }
        }
    }

    /// Compute READS relation.
    /// (p, A) READS (r, C) if:
    /// - r = goto(p, A)
    /// - The closure of r contains [C -> .gamma]
    /// - C is nullable
    fn compute_reads_relation(
        &mut self,
        grammar: &Grammar,
        states: &[State],
        transitions: &HashMap<usize, HashMap<String, usize>>,
    ) {
        for state in states {
            if let Some(state_trans) = transitions.get(&state.id) {
                for (sym, target_state_id) in state_trans {
                    // Only nonterminal transitions
                    if grammar.tokens.contains(sym) || sym == "$" {
                        continue;
                    }

                    let nt_trans = NonterminalTransition {
                        state: state.id,
                        nonterminal: sym.clone(),
                    };

                    let _target_state = &states[*target_state_id];
                    let mut reads_set = HashSet::new();

                    // Check transitions from target state on nullable nonterminals
                    if let Some(target_trans) = transitions.get(target_state_id) {
                        for (target_sym, _) in target_trans {
                            // Must be a nullable nonterminal
                            if !grammar.tokens.contains(target_sym)
                               && target_sym != "$"
                               && self.nullable.contains(target_sym)
                            {
                                reads_set.insert(NonterminalTransition {
                                    state: *target_state_id,
                                    nonterminal: target_sym.clone(),
                                });
                            }
                        }
                    }

                    if !reads_set.is_empty() {
                        self.reads.insert(nt_trans, reads_set);
                    }
                }
            }
        }
    }

    /// Compute Read sets via transitive closure over READS relation.
    /// Uses Tarjan's algorithm for computing SCCs and digraph traversal.
    fn compute_read_sets(
        &mut self,
        _transitions: &HashMap<usize, HashMap<String, usize>>,
    ) {
        // Collect all nonterminal transitions
        let all_trans: Vec<NonterminalTransition> = self.direct_read.keys().cloned().collect();

        // Initialize read sets with direct read
        for trans in &all_trans {
            let dr = self.direct_read.get(trans).cloned().unwrap_or_default();
            self.read_sets.insert(trans.clone(), dr);
        }

        // Compute transitive closure using a worklist algorithm
        let mut changed = true;
        while changed {
            changed = false;

            for trans in &all_trans {
                if let Some(reads_targets) = self.reads.get(trans).cloned() {
                    let mut to_add = HashSet::new();

                    for target in &reads_targets {
                        if let Some(target_read) = self.read_sets.get(target) {
                            to_add.extend(target_read.clone());
                        }
                    }

                    let read_set = self.read_sets.entry(trans.clone()).or_default();
                    let old_len = read_set.len();
                    read_set.extend(to_add);
                    if read_set.len() > old_len {
                        changed = true;
                    }
                }
            }
        }
    }

    /// Compute INCLUDES and LOOKBACK relations.
    ///
    /// For each production B -> beta A gamma where gamma =>* epsilon:
    /// - Find the state p' from which reading beta leads to state p
    /// - Then (p, A) INCLUDES (p', B)
    /// - Also, if the full production is reduced in state s, (s, B -> beta A gamma) LOOKBACK (p, A)
    fn compute_includes_and_lookback(
        &mut self,
        grammar: &Grammar,
        states: &[State],
        transitions: &HashMap<usize, HashMap<String, usize>>,
    ) {
        // For each state and each item that is a "kernel" item for a reduction
        for state in states {
            for item in &state.items {
                let _rule = &grammar.rules[item.rule_index];

                // We need to trace where we came from to reach this item
                // For each position in the RHS, we track possible states

                // Start from states that have this rule's LHS as a transition
                // and trace through the RHS symbols

                // Find all "origin" states: states p where [A -> . rhs] is in closure
                // and following rhs leads to current state with current dot position
                self.trace_production_paths(
                    grammar,
                    states,
                    transitions,
                    item.rule_index,
                    item.dot,
                    state.id,
                );
            }
        }
    }

    /// Trace all paths that lead to a specific item in a specific state.
    /// This establishes INCLUDES and LOOKBACK relations.
    fn trace_production_paths(
        &mut self,
        grammar: &Grammar,
        states: &[State],
        transitions: &HashMap<usize, HashMap<String, usize>>,
        rule_index: usize,
        dot_position: usize,
        current_state: usize,
    ) {
        let rule = &grammar.rules[rule_index];

        // Only process complete items for LOOKBACK, but we need partial items for INCLUDES
        let is_complete = dot_position == rule.rhs.len();

        // Find all origin states where [A -> . alpha] and trace forward
        for origin_state in states {
            // Check if origin state contains [rule_index, dot=0] in its closure
            let origin_items = closure(grammar, &origin_state.items);
            let has_initial_item = origin_items.contains(&Item {
                rule_index,
                dot: 0,
            });

            if !has_initial_item {
                continue;
            }

            // Trace the path from origin through the RHS symbols
            // We need to verify that following rhs[0..dot_position] from origin leads to current_state
            let path_valid = self.verify_path(
                grammar,
                transitions,
                origin_state.id,
                rule_index,
                dot_position,
                current_state,
            );

            if !path_valid {
                continue;
            }

            // Now establish INCLUDES relations for this path
            // For each nonterminal A at position i in beta, where gamma (rest after A) is nullable:
            // (state_at_position_i, A) INCLUDES (origin, rule.lhs)
            let mut current_trace_state = origin_state.id;
            for (i, sym) in rule.rhs.iter().enumerate().take(dot_position) {
                let sym_name = match sym {
                    Symbol::Terminal(t) => t,
                    Symbol::NonTerminal(nt) => nt,
                };

                // Check if rest of RHS after this symbol (gamma) is nullable
                let gamma_nullable = self.is_suffix_nullable(grammar, rule_index, i + 1);

                // If this is a nonterminal and gamma is nullable, add INCLUDES
                if let Symbol::NonTerminal(nt) = sym {
                    if gamma_nullable || i + 1 == rule.rhs.len() {
                        // (current_trace_state, nt) INCLUDES (origin, rule.lhs)
                        let from_trans = NonterminalTransition {
                            state: current_trace_state,
                            nonterminal: nt.clone(),
                        };
                        let to_trans = NonterminalTransition {
                            state: origin_state.id,
                            nonterminal: rule.lhs.clone(),
                        };

                        self.includes
                            .entry(from_trans)
                            .or_default()
                            .insert(to_trans);
                    }
                }

                // Move to next state
                if let Some(state_trans) = transitions.get(&current_trace_state) {
                    if let Some(&next_state) = state_trans.get(sym_name) {
                        current_trace_state = next_state;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            // If this is a complete item, add LOOKBACK
            if is_complete {
                let reduction = ReductionItem {
                    state: current_state,
                    rule_index,
                };
                let nt_trans = NonterminalTransition {
                    state: origin_state.id,
                    nonterminal: rule.lhs.clone(),
                };

                self.lookback
                    .entry(reduction)
                    .or_default()
                    .insert(nt_trans);
            }
        }
    }

    /// Verify that following rhs[0..dot] from origin_state leads to target_state.
    fn verify_path(
        &self,
        grammar: &Grammar,
        transitions: &HashMap<usize, HashMap<String, usize>>,
        origin_state: usize,
        rule_index: usize,
        dot_position: usize,
        target_state: usize,
    ) -> bool {
        let rule = &grammar.rules[rule_index];

        if dot_position == 0 {
            return origin_state == target_state;
        }

        let mut current = origin_state;
        for sym in rule.rhs.iter().take(dot_position) {
            let sym_name = match sym {
                Symbol::Terminal(t) => t,
                Symbol::NonTerminal(nt) => nt,
            };

            if let Some(state_trans) = transitions.get(&current) {
                if let Some(&next) = state_trans.get(sym_name) {
                    current = next;
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }

        current == target_state
    }

    /// Check if the suffix of a production (from position `start` to end) is nullable.
    fn is_suffix_nullable(&self, grammar: &Grammar, rule_index: usize, start: usize) -> bool {
        let rule = &grammar.rules[rule_index];

        for sym in rule.rhs.iter().skip(start) {
            match sym {
                Symbol::Terminal(_) => return false,
                Symbol::NonTerminal(nt) => {
                    if !self.nullable.contains(nt) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Compute final lookahead sets for all reduction items.
    /// LA(q, A -> alpha) = union { Follow(p, A) | (q, A -> alpha) LOOKBACK (p, A) }
    /// where Follow(p, A) = Read(p, A) union { Follow(p', B) | (p, A) INCLUDES (p', B) }
    fn compute_final_lookaheads(
        &mut self,
        _transitions: &HashMap<usize, HashMap<String, usize>>,
    ) {
        // First compute Follow sets (union of Read sets through INCLUDES)
        // Follow(p, A) = Read(p, A) union { Follow(p', B) | (p, A) INCLUDES (p', B) }

        let all_trans: Vec<NonterminalTransition> = self.read_sets.keys().cloned().collect();
        let mut follow_sets: HashMap<NonterminalTransition, HashSet<String>> = HashMap::new();

        // Initialize with read sets
        for trans in &all_trans {
            let read_set = self.read_sets.get(trans).cloned().unwrap_or_default();
            follow_sets.insert(trans.clone(), read_set);
        }

        // Compute transitive closure over INCLUDES
        let mut changed = true;
        while changed {
            changed = false;

            for trans in &all_trans {
                if let Some(includes_targets) = self.includes.get(trans).cloned() {
                    let mut to_add = HashSet::new();

                    for target in &includes_targets {
                        if let Some(target_follow) = follow_sets.get(target) {
                            to_add.extend(target_follow.clone());
                        }
                    }

                    let follow_set = follow_sets.entry(trans.clone()).or_default();
                    let old_len = follow_set.len();
                    follow_set.extend(to_add);
                    if follow_set.len() > old_len {
                        changed = true;
                    }
                }
            }
        }

        // Now compute LA for each reduction using LOOKBACK
        for (reduction, lookback_set) in &self.lookback {
            let mut la = HashSet::new();

            for trans in lookback_set {
                if let Some(follow_set) = follow_sets.get(trans) {
                    la.extend(follow_set.clone());
                }
            }

            self.lookaheads.insert(reduction.clone(), la);
        }
    }

    /// Get the lookahead set for a specific reduction item.
    /// Returns None if not found (should not happen for valid grammars).
    fn get_lookahead(&self, state: usize, rule_index: usize) -> Option<&HashSet<String>> {
        let key = ReductionItem { state, rule_index };
        self.lookaheads.get(&key)
    }
}
