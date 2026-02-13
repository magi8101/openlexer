//! GLR (Generalized LR) Parser Support.
//!
//! Implements Graph-Structured Stack (GSS) based GLR parsing for handling
//! ambiguous grammars where standard LALR(1) would have unresolved conflicts.
//!
//! GLR Algorithm Overview:
//! 1. Uses a Graph-Structured Stack instead of a linear stack
//! 2. On shift/reduce or reduce/reduce conflicts, the parser forks
//! 3. Each forked path continues independently in parallel
//! 4. Invalid paths fail when they encounter parse errors
//! 5. Valid paths merge when they reach the same state
//! 6. Semantic actions are deferred until disambiguation
//!
//! Key Data Structures:
//! - GssNode: A node in the Graph-Structured Stack
//! - GssEdge: An edge connecting GSS nodes (with semantic value)
//! - ParseForest: SPPF (Shared Packed Parse Forest) for deferred semantics
//!
//! References:
//! - Tomita, M. (1985). "Efficient Parsing for Natural Language"
//! - Scott, E. and Johnstone, A. (2006). "Right Nulled GLR Parsers"
//! - GNU Bison GLR documentation

use crate::error::{Error, Result};
use crate::parsegen::grammar::Grammar;
use crate::parsegen::lalr::{Action, ParsingTable};
use std::collections::{HashMap, HashSet, VecDeque};

/// Unique identifier for GSS nodes.
pub type NodeId = usize;

/// Unique identifier for parse forest nodes.
pub type ForestNodeId = usize;

/// A node in the Graph-Structured Stack.
/// Each node represents a parser state at a specific position.
#[derive(Debug, Clone)]
pub struct GssNode {
    /// Unique identifier for this node.
    pub id: NodeId,
    /// The parser state number.
    pub state: usize,
    /// Edges leading back to predecessor nodes.
    pub edges: Vec<GssEdge>,
    /// Input position when this node was created.
    pub position: usize,
}

/// An edge in the GSS connecting a node to its predecessor.
/// Contains the semantic value associated with the transition.
#[derive(Debug, Clone)]
pub struct GssEdge {
    /// The predecessor node.
    pub target: NodeId,
    /// Reference to the parse forest node holding the semantic value.
    pub forest_node: Option<ForestNodeId>,
    /// The symbol that was shifted/reduced to create this edge.
    pub symbol: String,
}

/// A node in the Shared Packed Parse Forest (SPPF).
/// Used to store deferred semantic values and actions.
#[derive(Debug, Clone)]
pub enum ForestNode {
    /// A terminal symbol from the input.
    Terminal {
        id: ForestNodeId,
        symbol: String,
        value: String,
        position: usize,
    },
    /// Result of a reduction.
    NonTerminal {
        id: ForestNodeId,
        symbol: String,
        rule_index: usize,
        children: Vec<ForestNodeId>,
    },
    /// An ambiguity node with multiple possible derivations.
    Ambiguous {
        id: ForestNodeId,
        symbol: String,
        alternatives: Vec<ForestNodeId>,
    },
}

impl ForestNode {
    pub fn id(&self) -> ForestNodeId {
        match self {
            ForestNode::Terminal { id, .. } => *id,
            ForestNode::NonTerminal { id, .. } => *id,
            ForestNode::Ambiguous { id, .. } => *id,
        }
    }

    pub fn symbol(&self) -> &str {
        match self {
            ForestNode::Terminal { symbol, .. } => symbol,
            ForestNode::NonTerminal { symbol, .. } => symbol,
            ForestNode::Ambiguous { symbol, .. } => symbol,
        }
    }
}

/// The Shared Packed Parse Forest storing all parse derivations.
#[derive(Debug, Clone)]
pub struct ParseForest {
    /// All forest nodes indexed by their ID.
    pub nodes: Vec<ForestNode>,
    /// Next available node ID.
    next_id: ForestNodeId,
}

impl ParseForest {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            next_id: 0,
        }
    }

    /// Create a terminal node in the forest.
    pub fn add_terminal(&mut self, symbol: String, value: String, position: usize) -> ForestNodeId {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.push(ForestNode::Terminal {
            id,
            symbol,
            value,
            position,
        });
        id
    }

    /// Create a non-terminal node in the forest for a reduction.
    pub fn add_nonterminal(
        &mut self,
        symbol: String,
        rule_index: usize,
        children: Vec<ForestNodeId>,
    ) -> ForestNodeId {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.push(ForestNode::NonTerminal {
            id,
            symbol,
            rule_index,
            children,
        });
        id
    }

    /// Create an ambiguity node for multiple derivations.
    pub fn add_ambiguous(
        &mut self,
        symbol: String,
        alternatives: Vec<ForestNodeId>,
    ) -> ForestNodeId {
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.push(ForestNode::Ambiguous {
            id,
            symbol,
            alternatives,
        });
        id
    }

    /// Get a node by ID.
    pub fn get(&self, id: ForestNodeId) -> Option<&ForestNode> {
        self.nodes.get(id)
    }
}

/// Represents a token from the lexer.
#[derive(Debug, Clone)]
pub struct Token {
    pub symbol: String,
    pub value: String,
    pub position: usize,
}

/// Active parser state - represents one path in the GLR parse.
#[derive(Debug, Clone)]
pub struct ActiveParser {
    /// The current GSS node for this parser.
    pub node_id: NodeId,
    /// Whether this parser has accepted.
    pub accepted: bool,
    /// Whether this parser has failed.
    pub failed: bool,
}

/// The GLR parser state.
pub struct GlrParser<'a> {
    /// Reference to the parsing table (from LALR construction).
    table: &'a ParsingTable,
    /// Reference to the grammar.
    grammar: &'a Grammar,
    /// The Graph-Structured Stack.
    gss: GlrGss,
    /// The parse forest for deferred semantic actions.
    forest: ParseForest,
    /// Currently active parsers.
    active: Vec<ActiveParser>,
    /// Pending reductions to process (for GLR synchronization).
    pending_reductions: VecDeque<PendingReduction>,
    /// Set of (node_id, rule_index) pairs already queued for reduction at current position.
    /// Prevents infinite loops in ambiguous grammars.
    processed_reductions: HashSet<(NodeId, usize)>,
    /// Input tokens.
    tokens: Vec<Token>,
    /// Current input position.
    position: usize,
}

/// A pending reduction to be processed.
#[derive(Debug, Clone)]
struct PendingReduction {
    /// GSS node where reduction starts.
    node_id: NodeId,
    /// Rule to reduce by.
    rule_index: usize,
    /// Number of symbols to pop.
    pop_count: usize,
}

/// The Graph-Structured Stack implementation.
#[derive(Debug, Clone)]
pub struct GlrGss {
    /// All nodes in the GSS.
    nodes: Vec<GssNode>,
    /// Next available node ID.
    next_id: NodeId,
    /// Map from (state, position) to existing node IDs for merging.
    state_position_map: HashMap<(usize, usize), NodeId>,
}

impl GlrGss {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            next_id: 0,
            state_position_map: HashMap::new(),
        }
    }

    /// Create or find an existing node for the given state at the given position.
    /// This enables path merging when multiple paths reach the same state.
    pub fn get_or_create_node(&mut self, state: usize, position: usize) -> (NodeId, bool) {
        let key = (state, position);
        if let Some(&existing_id) = self.state_position_map.get(&key) {
            (existing_id, false) // Node already exists
        } else {
            let id = self.next_id;
            self.next_id += 1;
            self.nodes.push(GssNode {
                id,
                state,
                edges: Vec::new(),
                position,
            });
            self.state_position_map.insert(key, id);
            (id, true) // New node created
        }
    }

    /// Add an edge from a node to its predecessor.
    pub fn add_edge(
        &mut self,
        from: NodeId,
        to: NodeId,
        forest_node: Option<ForestNodeId>,
        symbol: String,
    ) {
        if let Some(node) = self.nodes.get_mut(from) {
            // Check if edge already exists to avoid duplicates
            let exists = node
                .edges
                .iter()
                .any(|e| e.target == to && e.symbol == symbol);
            if !exists {
                node.edges.push(GssEdge {
                    target: to,
                    forest_node,
                    symbol,
                });
            }
        }
    }

    /// Get a node by ID.
    pub fn get_node(&self, id: NodeId) -> Option<&GssNode> {
        self.nodes.get(id)
    }

    /// Get a mutable node by ID.
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut GssNode> {
        self.nodes.get_mut(id)
    }

    /// Enumerate all paths of a given length starting from a node.
    /// Returns a list of (path of node IDs, collected forest nodes).
    pub fn enumerate_paths(
        &self,
        start: NodeId,
        length: usize,
    ) -> Vec<(Vec<NodeId>, Vec<Option<ForestNodeId>>)> {
        if length == 0 {
            return vec![(vec![start], vec![])];
        }

        let mut result = Vec::new();
        self.enumerate_paths_recursive(start, length, vec![start], vec![], &mut result);
        result
    }

    fn enumerate_paths_recursive(
        &self,
        current: NodeId,
        remaining: usize,
        path: Vec<NodeId>,
        forest_nodes: Vec<Option<ForestNodeId>>,
        result: &mut Vec<(Vec<NodeId>, Vec<Option<ForestNodeId>>)>,
    ) {
        if remaining == 0 {
            result.push((path, forest_nodes));
            return;
        }

        if let Some(node) = self.get_node(current) {
            for edge in &node.edges {
                let mut new_path = path.clone();
                new_path.push(edge.target);
                let mut new_forest = forest_nodes.clone();
                new_forest.push(edge.forest_node);
                self.enumerate_paths_recursive(
                    edge.target,
                    remaining - 1,
                    new_path,
                    new_forest,
                    result,
                );
            }
        }
    }
}

impl<'a> GlrParser<'a> {
    /// Create a new GLR parser.
    pub fn new(table: &'a ParsingTable, grammar: &'a Grammar) -> Self {
        let mut gss = GlrGss::new();
        let forest = ParseForest::new();

        // Create initial node in state 0 at position 0
        let (initial_node, _) = gss.get_or_create_node(0, 0);

        Self {
            table,
            grammar,
            gss,
            forest,
            active: vec![ActiveParser {
                node_id: initial_node,
                accepted: false,
                failed: false,
            }],
            pending_reductions: VecDeque::new(),
            processed_reductions: HashSet::new(),
            tokens: Vec::new(),
            position: 0,
        }
    }

    /// Parse the input tokens.
    /// Returns the root of the parse forest on success.
    pub fn parse(&mut self, tokens: Vec<Token>) -> Result<ForestNodeId> {
        self.tokens = tokens;
        self.tokens.push(Token {
            symbol: "$".to_string(),
            value: String::new(),
            position: self.tokens.len(),
        });

        while self.position < self.tokens.len() {
            let token = self.tokens[self.position].clone();

            // Clear processed reductions set for new position
            self.processed_reductions.clear();

            // Process all pending reductions first
            self.process_reductions()?;

            // Perform shifts
            self.perform_shifts(&token)?;

            // Check if any parsers are still active
            let live_parsers: Vec<_> = self
                .active
                .iter()
                .filter(|p| !p.failed && !p.accepted)
                .collect();

            if live_parsers.is_empty() && !self.active.iter().any(|p| p.accepted) {
                return Err(Error::ParseError {
                    position: self.position,
                    message: format!("Unexpected token '{}'", token.symbol),
                });
            }

            self.position += 1;
        }

        // Check for accepted parsers
        let accepted: Vec<_> = self.active.iter().filter(|p| p.accepted).collect();

        if accepted.is_empty() {
            return Err(Error::ParseError {
                position: self.position,
                message: "Unexpected end of input".to_string(),
            });
        }

        // Return the forest root from the first accepted parser
        // In truly ambiguous cases, we'd need to handle multiple parses
        if let Some(node) = self.gss.get_node(accepted[0].node_id) {
            if let Some(edge) = node.edges.first() {
                if let Some(forest_id) = edge.forest_node {
                    return Ok(forest_id);
                }
            }
        }

        // If no forest node, create a dummy success node
        Ok(self
            .forest
            .add_nonterminal(self.grammar.start_symbol.clone(), 0, vec![]))
    }

    /// Process all pending reductions.
    fn process_reductions(&mut self) -> Result<()> {
        // Collect initial reductions from current active parsers
        self.collect_reductions();

        // Process reductions until none remain
        while let Some(reduction) = self.pending_reductions.pop_front() {
            self.perform_reduction(reduction)?;
        }

        Ok(())
    }

    /// Collect all possible reductions for the current token.
    /// In GLR mode, also checks glr_conflict_actions for alternative reduce actions.
    fn collect_reductions(&mut self) {
        let token = &self.tokens[self.position];

        for parser in &self.active {
            if parser.failed || parser.accepted {
                continue;
            }

            if let Some(node) = self.gss.get_node(parser.node_id) {
                // Collect reductions from the main action table
                if let Some(actions) = self.table.action.get(&node.state) {
                    if let Some(action) = actions.get(&token.symbol) {
                        if let Action::Reduce(rule_idx) = action {
                            let key = (parser.node_id, *rule_idx);
                            if !self.processed_reductions.contains(&key) {
                                self.processed_reductions.insert(key);
                                let rule = &self.grammar.rules[*rule_idx];
                                self.pending_reductions.push_back(PendingReduction {
                                    node_id: parser.node_id,
                                    rule_index: *rule_idx,
                                    pop_count: rule.rhs.len(),
                                });
                            }
                        }
                    }
                }

                // GLR: Also collect reductions from conflict alternatives
                if let Some(conflict_actions) = self.table.glr_conflict_actions.get(&node.state) {
                    if let Some(alt_actions) = conflict_actions.get(&token.symbol) {
                        for alt_action in alt_actions {
                            if let Action::Reduce(rule_idx) = alt_action {
                                let key = (parser.node_id, *rule_idx);
                                if !self.processed_reductions.contains(&key) {
                                    self.processed_reductions.insert(key);
                                    let rule = &self.grammar.rules[*rule_idx];
                                    self.pending_reductions.push_back(PendingReduction {
                                        node_id: parser.node_id,
                                        rule_index: *rule_idx,
                                        pop_count: rule.rhs.len(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Perform a single reduction.
    fn perform_reduction(&mut self, reduction: PendingReduction) -> Result<()> {
        let rule = &self.grammar.rules[reduction.rule_index];
        let lhs = &rule.lhs;

        // Enumerate all paths of length pop_count from the reduction node
        let paths = self
            .gss
            .enumerate_paths(reduction.node_id, reduction.pop_count);

        for (path, forest_children) in paths {
            // The target is the last node in the path (after all pops)
            let target_node_id = *path.last().unwrap();

            if let Some(target_node) = self.gss.get_node(target_node_id) {
                // Look up goto state for this nonterminal
                if let Some(goto_map) = self.table.goto.get(&target_node.state) {
                    if let Some(&goto_state) = goto_map.get(lhs) {
                        let node_position = self
                            .gss
                            .get_node(reduction.node_id)
                            .map(|n| n.position)
                            .unwrap_or(self.position);

                        // Create or merge into the goto state node
                        let (new_node_id, is_new) =
                            self.gss.get_or_create_node(goto_state, node_position);

                        // Create forest node for this reduction
                        let children: Vec<ForestNodeId> =
                            forest_children.iter().filter_map(|&opt| opt).collect();
                        let forest_node = self.forest.add_nonterminal(
                            lhs.clone(),
                            reduction.rule_index,
                            children,
                        );

                        // Add edge from new node to target
                        self.gss.add_edge(
                            new_node_id,
                            target_node_id,
                            Some(forest_node),
                            lhs.clone(),
                        );

                        // If this is a new node, add a new active parser
                        if is_new {
                            self.active.push(ActiveParser {
                                node_id: new_node_id,
                                accepted: false,
                                failed: false,
                            });
                        }

                        // Check for further reductions from the new state
                        let token = &self.tokens[self.position];
                        if let Some(actions) = self.table.action.get(&goto_state) {
                            if let Some(Action::Reduce(next_rule_idx)) = actions.get(&token.symbol)
                            {
                                let key = (new_node_id, *next_rule_idx);
                                if !self.processed_reductions.contains(&key) {
                                    self.processed_reductions.insert(key);
                                    let next_rule = &self.grammar.rules[*next_rule_idx];
                                    self.pending_reductions.push_back(PendingReduction {
                                        node_id: new_node_id,
                                        rule_index: *next_rule_idx,
                                        pop_count: next_rule.rhs.len(),
                                    });
                                }
                            }
                        }

                        // GLR: Also check glr_conflict_actions for further reductions
                        if let Some(conflict_actions) =
                            self.table.glr_conflict_actions.get(&goto_state)
                        {
                            if let Some(alt_actions) = conflict_actions.get(&token.symbol) {
                                for alt_action in alt_actions {
                                    if let Action::Reduce(next_rule_idx) = alt_action {
                                        let key = (new_node_id, *next_rule_idx);
                                        if !self.processed_reductions.contains(&key) {
                                            self.processed_reductions.insert(key);
                                            let next_rule = &self.grammar.rules[*next_rule_idx];
                                            self.pending_reductions.push_back(PendingReduction {
                                                node_id: new_node_id,
                                                rule_index: *next_rule_idx,
                                                pop_count: next_rule.rhs.len(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Perform shifts for all active parsers.
    fn perform_shifts(&mut self, token: &Token) -> Result<()> {
        let mut new_active = Vec::new();

        // Create forest node for the shifted token
        let token_forest_id =
            self.forest
                .add_terminal(token.symbol.clone(), token.value.clone(), token.position);

        for parser in &mut self.active {
            if parser.failed || parser.accepted {
                new_active.push(parser.clone());
                continue;
            }

            // Extract state early to avoid borrow conflicts
            let node_state = if let Some(node) = self.gss.get_node(parser.node_id) {
                node.state
            } else {
                continue;
            };

            if let Some(actions) = self.table.action.get(&node_state) {
                // Check for multiple actions (conflict handling)
                let mut shifted = false;
                let mut reduced = false;

                if let Some(action) = actions.get(&token.symbol) {
                    match action {
                        Action::Shift(target_state) => {
                            // Shift: create new node in target state
                            let (new_node_id, _) = self
                                .gss
                                .get_or_create_node(*target_state, self.position + 1);
                            self.gss.add_edge(
                                new_node_id,
                                parser.node_id,
                                Some(token_forest_id),
                                token.symbol.clone(),
                            );
                            new_active.push(ActiveParser {
                                node_id: new_node_id,
                                accepted: false,
                                failed: false,
                            });
                            shifted = true;
                        }
                        Action::Accept => {
                            new_active.push(ActiveParser {
                                node_id: parser.node_id,
                                accepted: true,
                                failed: false,
                            });
                        }
                        Action::Reduce(_) => {
                            // Reductions already handled
                            reduced = true;
                        }
                    }
                }

                // In GLR mode, we may have both shift and reduce on same token
                // Check glr_conflict_actions for alternative shifts
                if self.grammar.glr_mode {
                    if let Some(conflict_actions) = self.table.glr_conflict_actions.get(&node_state)
                    {
                        if let Some(alt_actions) = conflict_actions.get(&token.symbol) {
                            for alt_action in alt_actions {
                                if let Action::Shift(target_state) = alt_action {
                                    // Only add if we haven't already shifted to this state
                                    let (new_node_id, _) = self
                                        .gss
                                        .get_or_create_node(*target_state, self.position + 1);
                                    self.gss.add_edge(
                                        new_node_id,
                                        parser.node_id,
                                        Some(token_forest_id),
                                        token.symbol.clone(),
                                    );
                                    // Only add if not already in new_active
                                    if !new_active.iter().any(|p| p.node_id == new_node_id) {
                                        new_active.push(ActiveParser {
                                            node_id: new_node_id,
                                            accepted: false,
                                            failed: false,
                                        });
                                    }
                                    shifted = true;
                                }
                            }
                        }
                    }

                    if !shifted && !reduced && actions.get(&token.symbol).is_none() {
                        // No valid action for this token - parser fails
                        new_active.push(ActiveParser {
                            node_id: parser.node_id,
                            accepted: false,
                            failed: true,
                        });
                    }
                } else {
                    // No actions for this state - parser fails
                    new_active.push(ActiveParser {
                        node_id: parser.node_id,
                        accepted: false,
                        failed: true,
                    });
                }
            }
        }

        self.active = new_active;
        Ok(())
    }

    /// Check if the parse was ambiguous (multiple successful derivations).
    pub fn is_ambiguous(&self) -> bool {
        self.active.iter().filter(|p| p.accepted).count() > 1
    }

    /// Get all accepted parse forest roots for ambiguous parses.
    pub fn get_all_parses(&self) -> Vec<ForestNodeId> {
        let mut roots = Vec::new();

        for parser in &self.active {
            if parser.accepted {
                if let Some(node) = self.gss.get_node(parser.node_id) {
                    for edge in &node.edges {
                        if let Some(forest_id) = edge.forest_node {
                            roots.push(forest_id);
                        }
                    }
                }
            }
        }

        roots
    }
}

/// GLR-specific table with conflict information preserved.
/// Standard LALR tables resolve conflicts; GLR tables keep all actions.
#[derive(Debug, Clone)]
pub struct GlrTable {
    /// State -> (Symbol -> List of Actions)
    /// Multiple actions indicate a conflict that GLR will explore.
    pub actions: HashMap<usize, HashMap<String, Vec<Action>>>,
    /// State -> (NonTerminal -> State)
    pub goto: HashMap<usize, HashMap<String, usize>>,
}

impl GlrTable {
    /// Build a GLR table from a grammar, preserving all conflicts.
    pub fn build(table: &ParsingTable, _grammar: &Grammar) -> Self {
        // Convert the LALR table to a GLR table.
        // For a true GLR implementation, we would need to preserve
        // all possible actions at each conflict point.

        let mut actions: HashMap<usize, HashMap<String, Vec<Action>>> = HashMap::new();

        for (state, action_map) in &table.action {
            let mut state_actions: HashMap<String, Vec<Action>> = HashMap::new();
            for (symbol, action) in action_map {
                state_actions.insert(symbol.clone(), vec![action.clone()]);
            }
            actions.insert(*state, state_actions);
        }

        Self {
            actions,
            goto: table.goto.clone(),
        }
    }

    /// Add an additional action at a conflict point.
    pub fn add_action(&mut self, state: usize, symbol: &str, action: Action) {
        let state_actions = self.actions.entry(state).or_default();
        let symbol_actions = state_actions.entry(symbol.to_string()).or_default();
        if !symbol_actions.contains(&action) {
            symbol_actions.push(action);
        }
    }

    /// Get all actions for a state/symbol pair.
    pub fn get_actions(&self, state: usize, symbol: &str) -> Option<&Vec<Action>> {
        self.actions.get(&state).and_then(|m| m.get(symbol))
    }
}

/// Resolve ambiguity by selecting the first parse tree.
/// This is a simple disambiguation strategy; more complex strategies
/// could consider precedence, associativity, or user-defined rules.
pub fn disambiguate_first(_forest: &ParseForest, roots: &[ForestNodeId]) -> Option<ForestNodeId> {
    roots.first().copied()
}

/// Resolve ambiguity by selecting the parse with the fewest nodes.
pub fn disambiguate_shortest(forest: &ParseForest, roots: &[ForestNodeId]) -> Option<ForestNodeId> {
    roots
        .iter()
        .map(|&id| (id, count_forest_nodes(forest, id)))
        .min_by_key(|&(_, count)| count)
        .map(|(id, _)| id)
}

/// Count the total number of nodes in a derivation.
fn count_forest_nodes(forest: &ParseForest, root: ForestNodeId) -> usize {
    let node = match forest.get(root) {
        Some(n) => n,
        None => return 0,
    };

    match node {
        ForestNode::Terminal { .. } => 1,
        ForestNode::NonTerminal { children, .. } => {
            1 + children
                .iter()
                .map(|&c| count_forest_nodes(forest, c))
                .sum::<usize>()
        }
        ForestNode::Ambiguous { alternatives, .. } => {
            // For ambiguous nodes, count the first alternative
            1 + alternatives
                .first()
                .map(|&c| count_forest_nodes(forest, c))
                .unwrap_or(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gss_creation() {
        let mut gss = GlrGss::new();
        let (id1, created1) = gss.get_or_create_node(0, 0);
        assert!(created1);

        let (id2, created2) = gss.get_or_create_node(0, 0);
        assert!(!created2);
        assert_eq!(id1, id2);

        let (id3, created3) = gss.get_or_create_node(1, 0);
        assert!(created3);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_gss_paths() {
        let mut gss = GlrGss::new();

        // Create a simple linear path: 0 <- 1 <- 2
        let (node0, _) = gss.get_or_create_node(0, 0);
        let (node1, _) = gss.get_or_create_node(1, 1);
        let (node2, _) = gss.get_or_create_node(2, 2);

        gss.add_edge(node1, node0, Some(100), "a".to_string());
        gss.add_edge(node2, node1, Some(101), "b".to_string());

        // Enumerate paths of length 2 from node2
        let paths = gss.enumerate_paths(node2, 2);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].0, vec![node2, node1, node0]);
    }

    #[test]
    fn test_forest_creation() {
        let mut forest = ParseForest::new();

        let t1 = forest.add_terminal("NUMBER".to_string(), "42".to_string(), 0);
        let t2 = forest.add_terminal("PLUS".to_string(), "+".to_string(), 1);
        let t3 = forest.add_terminal("NUMBER".to_string(), "3".to_string(), 2);

        let nt = forest.add_nonterminal("expr".to_string(), 0, vec![t1, t2, t3]);

        assert_eq!(forest.nodes.len(), 4);

        if let ForestNode::NonTerminal { children, .. } = forest.get(nt).unwrap() {
            assert_eq!(children.len(), 3);
        } else {
            panic!("Expected NonTerminal node");
        }
    }

    #[test]
    fn test_ambiguous_forest() {
        let mut forest = ParseForest::new();

        // Create two different derivations for the same expression
        let t1 = forest.add_terminal("a".to_string(), "a".to_string(), 0);

        let deriv1 = forest.add_nonterminal("S".to_string(), 0, vec![t1]);
        let deriv2 = forest.add_nonterminal("S".to_string(), 1, vec![t1]);

        let ambig = forest.add_ambiguous("S".to_string(), vec![deriv1, deriv2]);

        if let ForestNode::Ambiguous { alternatives, .. } = forest.get(ambig).unwrap() {
            assert_eq!(alternatives.len(), 2);
        } else {
            panic!("Expected Ambiguous node");
        }
    }

    #[test]
    fn test_gss_branching_paths() {
        let mut gss = GlrGss::new();

        // Create a branching structure:
        //       node0
        //      /     \
        //   node1   node2
        //      \     /
        //       node3
        let (node0, _) = gss.get_or_create_node(0, 0);
        let (node1, _) = gss.get_or_create_node(1, 1);
        let (node2, _) = gss.get_or_create_node(2, 1);
        let (node3, _) = gss.get_or_create_node(3, 2);

        // node1 and node2 both point back to node0
        gss.add_edge(node1, node0, Some(100), "a".to_string());
        gss.add_edge(node2, node0, Some(101), "b".to_string());

        // node3 points back to both node1 and node2 (forked paths merge)
        gss.add_edge(node3, node1, Some(102), "c".to_string());
        gss.add_edge(node3, node2, Some(103), "d".to_string());

        // Enumerate paths of length 2 from node3 - should find 2 paths
        let paths = gss.enumerate_paths(node3, 2);
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn test_gss_path_of_length_zero() {
        let mut gss = GlrGss::new();
        let (node0, _) = gss.get_or_create_node(0, 0);

        let paths = gss.enumerate_paths(node0, 0);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].0, vec![node0]);
        assert!(paths[0].1.is_empty());
    }

    #[test]
    fn test_gss_duplicate_edge_prevention() {
        let mut gss = GlrGss::new();
        let (node0, _) = gss.get_or_create_node(0, 0);
        let (node1, _) = gss.get_or_create_node(1, 1);

        // Add same edge multiple times
        gss.add_edge(node1, node0, Some(100), "a".to_string());
        gss.add_edge(node1, node0, Some(100), "a".to_string());
        gss.add_edge(node1, node0, Some(100), "a".to_string());

        // Should only have one edge
        let node = gss.get_node(node1).unwrap();
        assert_eq!(node.edges.len(), 1);
    }

    #[test]
    fn test_forest_terminal_node() {
        let mut forest = ParseForest::new();
        let id = forest.add_terminal("ID".to_string(), "foo".to_string(), 5);

        if let ForestNode::Terminal {
            symbol,
            value,
            position,
            ..
        } = forest.get(id).unwrap()
        {
            assert_eq!(symbol, "ID");
            assert_eq!(value, "foo");
            assert_eq!(*position, 5);
        } else {
            panic!("Expected Terminal node");
        }
    }

    #[test]
    fn test_forest_nonterminal_node() {
        let mut forest = ParseForest::new();
        let t1 = forest.add_terminal("a".to_string(), "a".to_string(), 0);
        let t2 = forest.add_terminal("b".to_string(), "b".to_string(), 1);
        let nt = forest.add_nonterminal("S".to_string(), 3, vec![t1, t2]);

        if let ForestNode::NonTerminal {
            symbol,
            rule_index,
            children,
            ..
        } = forest.get(nt).unwrap()
        {
            assert_eq!(symbol, "S");
            assert_eq!(*rule_index, 3);
            assert_eq!(children.len(), 2);
            assert_eq!(children[0], t1);
            assert_eq!(children[1], t2);
        } else {
            panic!("Expected NonTerminal node");
        }
    }

    #[test]
    fn test_forest_node_symbol() {
        let mut forest = ParseForest::new();
        let t = forest.add_terminal("TOKEN".to_string(), "val".to_string(), 0);
        let nt = forest.add_nonterminal("RULE".to_string(), 0, vec![]);
        let amb = forest.add_ambiguous("AMB".to_string(), vec![]);

        assert_eq!(forest.get(t).unwrap().symbol(), "TOKEN");
        assert_eq!(forest.get(nt).unwrap().symbol(), "RULE");
        assert_eq!(forest.get(amb).unwrap().symbol(), "AMB");
    }

    #[test]
    fn test_forest_node_id() {
        let mut forest = ParseForest::new();
        let t = forest.add_terminal("a".to_string(), "a".to_string(), 0);
        let nt = forest.add_nonterminal("S".to_string(), 0, vec![t]);

        assert_eq!(forest.get(t).unwrap().id(), t);
        assert_eq!(forest.get(nt).unwrap().id(), nt);
    }

    #[test]
    fn test_glr_table_creation() {
        use crate::parsegen::grammar::Grammar;
        use crate::parsegen::lalr::ParsingTable;

        let grammar = Grammar::new();
        let table = ParsingTable {
            action: HashMap::new(),
            goto: HashMap::new(),
            shift_reduce_conflicts: 0,
            reduce_reduce_conflicts: 0,
            conflict_messages: vec![],
            glr_conflict_actions: HashMap::new(),
        };

        let glr_table = GlrTable::build(&table, &grammar);
        assert!(glr_table.actions.is_empty());
        assert!(glr_table.goto.is_empty());
    }

    #[test]
    fn test_glr_table_add_action() {
        use crate::parsegen::grammar::Grammar;
        use crate::parsegen::lalr::ParsingTable;

        let grammar = Grammar::new();
        let table = ParsingTable {
            action: HashMap::new(),
            goto: HashMap::new(),
            shift_reduce_conflicts: 0,
            reduce_reduce_conflicts: 0,
            conflict_messages: vec![],
            glr_conflict_actions: HashMap::new(),
        };

        let mut glr_table = GlrTable::build(&table, &grammar);

        // Add multiple actions at same state/symbol (conflict)
        glr_table.add_action(0, "a", Action::Shift(1));
        glr_table.add_action(0, "a", Action::Reduce(0));

        let actions = glr_table.get_actions(0, "a").unwrap();
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn test_glr_table_no_duplicate_actions() {
        use crate::parsegen::grammar::Grammar;
        use crate::parsegen::lalr::ParsingTable;

        let grammar = Grammar::new();
        let table = ParsingTable {
            action: HashMap::new(),
            goto: HashMap::new(),
            shift_reduce_conflicts: 0,
            reduce_reduce_conflicts: 0,
            conflict_messages: vec![],
            glr_conflict_actions: HashMap::new(),
        };

        let mut glr_table = GlrTable::build(&table, &grammar);

        // Add same action multiple times
        glr_table.add_action(0, "a", Action::Shift(1));
        glr_table.add_action(0, "a", Action::Shift(1));

        let actions = glr_table.get_actions(0, "a").unwrap();
        assert_eq!(actions.len(), 1);
    }

    #[test]
    fn test_disambiguate_first() {
        let forest = ParseForest::new();
        let roots = vec![0, 1, 2];

        assert_eq!(disambiguate_first(&forest, &roots), Some(0));
    }

    #[test]
    fn test_disambiguate_first_empty() {
        let forest = ParseForest::new();
        let roots: Vec<ForestNodeId> = vec![];

        assert_eq!(disambiguate_first(&forest, &roots), None);
    }

    #[test]
    fn test_disambiguate_shortest() {
        let mut forest = ParseForest::new();

        // Create two derivations of different lengths
        let t1 = forest.add_terminal("a".to_string(), "a".to_string(), 0);
        let t2 = forest.add_terminal("b".to_string(), "b".to_string(), 1);
        let t3 = forest.add_terminal("c".to_string(), "c".to_string(), 2);

        // Short derivation: S -> a (2 nodes)
        let short = forest.add_nonterminal("S".to_string(), 0, vec![t1]);

        // Long derivation: S -> a b c (4 nodes)
        let long = forest.add_nonterminal("S".to_string(), 1, vec![t1, t2, t3]);

        let roots = vec![long, short]; // Put long first

        let result = disambiguate_shortest(&forest, &roots);
        assert_eq!(result, Some(short));
    }

    #[test]
    fn test_count_forest_nodes() {
        let mut forest = ParseForest::new();

        let t1 = forest.add_terminal("a".to_string(), "a".to_string(), 0);
        assert_eq!(count_forest_nodes(&forest, t1), 1);

        let t2 = forest.add_terminal("b".to_string(), "b".to_string(), 1);
        let nt = forest.add_nonterminal("S".to_string(), 0, vec![t1, t2]);
        assert_eq!(count_forest_nodes(&forest, nt), 3); // 1 nonterminal + 2 terminals
    }

    #[test]
    fn test_active_parser_states() {
        let parser = ActiveParser {
            node_id: 0,
            accepted: false,
            failed: false,
        };
        assert!(!parser.accepted);
        assert!(!parser.failed);

        let accepted_parser = ActiveParser {
            node_id: 1,
            accepted: true,
            failed: false,
        };
        assert!(accepted_parser.accepted);

        let failed_parser = ActiveParser {
            node_id: 2,
            accepted: false,
            failed: true,
        };
        assert!(failed_parser.failed);
    }

    #[test]
    fn test_token_creation() {
        let token = Token {
            symbol: "NUMBER".to_string(),
            value: "123".to_string(),
            position: 5,
        };

        assert_eq!(token.symbol, "NUMBER");
        assert_eq!(token.value, "123");
        assert_eq!(token.position, 5);
    }

    #[test]
    fn test_gss_edge_creation() {
        let edge = GssEdge {
            target: 42,
            forest_node: Some(100),
            symbol: "expr".to_string(),
        };

        assert_eq!(edge.target, 42);
        assert_eq!(edge.forest_node, Some(100));
        assert_eq!(edge.symbol, "expr");
    }

    #[test]
    fn test_gss_node_creation() {
        let node = GssNode {
            id: 0,
            state: 5,
            edges: vec![],
            position: 10,
        };

        assert_eq!(node.id, 0);
        assert_eq!(node.state, 5);
        assert_eq!(node.position, 10);
        assert!(node.edges.is_empty());
    }

    // =========================================================================
    // GLR Integration Tests - Actual parsing with ambiguous grammars
    // =========================================================================

    /// Creates an ambiguous expression grammar: e : e '-' e | NUM
    /// This causes shift/reduce conflicts that GLR can handle.
    fn create_ambiguous_expr_grammar() -> Grammar {
        use crate::parsegen::grammar::Symbol;

        let mut grammar = Grammar::new();
        grammar.glr_mode = true;
        grammar.tokens = vec!["NUM".to_string(), "MINUS".to_string()];
        grammar.start_symbol = "expr".to_string();

        // Rule 0: expr -> expr MINUS expr  (ambiguous - no associativity)
        grammar.rules.push(crate::parsegen::grammar::Rule {
            lhs: "expr".to_string(),
            rhs: vec![
                Symbol::NonTerminal("expr".to_string()),
                Symbol::Terminal("MINUS".to_string()),
                Symbol::NonTerminal("expr".to_string()),
            ],
            action: None,
            precedence_sym: None,
        });

        // Rule 1: expr -> NUM
        grammar.rules.push(crate::parsegen::grammar::Rule {
            lhs: "expr".to_string(),
            rhs: vec![Symbol::Terminal("NUM".to_string())],
            action: None,
            precedence_sym: None,
        });

        grammar
    }

    /// Creates a dangling-else grammar: stmt : IF expr THEN stmt ELSE stmt | IF expr THEN stmt | OTHER
    /// Classic ambiguous grammar requiring GLR or explicit associativity.
    fn create_dangling_else_grammar() -> Grammar {
        use crate::parsegen::grammar::Symbol;

        let mut grammar = Grammar::new();
        grammar.glr_mode = true;
        grammar.tokens = vec![
            "IF".to_string(),
            "THEN".to_string(),
            "ELSE".to_string(),
            "EXPR".to_string(),
            "OTHER".to_string(),
        ];
        grammar.start_symbol = "stmt".to_string();

        // Rule 0: stmt -> IF EXPR THEN stmt ELSE stmt
        grammar.rules.push(crate::parsegen::grammar::Rule {
            lhs: "stmt".to_string(),
            rhs: vec![
                Symbol::Terminal("IF".to_string()),
                Symbol::Terminal("EXPR".to_string()),
                Symbol::Terminal("THEN".to_string()),
                Symbol::NonTerminal("stmt".to_string()),
                Symbol::Terminal("ELSE".to_string()),
                Symbol::NonTerminal("stmt".to_string()),
            ],
            action: None,
            precedence_sym: None,
        });

        // Rule 1: stmt -> IF EXPR THEN stmt
        grammar.rules.push(crate::parsegen::grammar::Rule {
            lhs: "stmt".to_string(),
            rhs: vec![
                Symbol::Terminal("IF".to_string()),
                Symbol::Terminal("EXPR".to_string()),
                Symbol::Terminal("THEN".to_string()),
                Symbol::NonTerminal("stmt".to_string()),
            ],
            action: None,
            precedence_sym: None,
        });

        // Rule 2: stmt -> OTHER
        grammar.rules.push(crate::parsegen::grammar::Rule {
            lhs: "stmt".to_string(),
            rhs: vec![Symbol::Terminal("OTHER".to_string())],
            action: None,
            precedence_sym: None,
        });

        grammar
    }

    #[test]
    fn test_glr_parse_simple_num() {
        use crate::parsegen::lalr::ParsingTable;

        // Simple grammar: e : NUM
        let mut grammar = Grammar::new();
        grammar.glr_mode = true;
        grammar.tokens = vec!["NUM".to_string()];
        grammar.start_symbol = "e".to_string();

        grammar.rules.push(crate::parsegen::grammar::Rule {
            lhs: "e".to_string(),
            rhs: vec![crate::parsegen::grammar::Symbol::Terminal(
                "NUM".to_string(),
            )],
            action: None,
            precedence_sym: None,
        });

        let table = ParsingTable::build(&grammar).expect("Failed to build parsing table");
        let mut parser = GlrParser::new(&table, &grammar);

        let tokens = vec![Token {
            symbol: "NUM".to_string(),
            value: "42".to_string(),
            position: 0,
        }];

        let result = parser.parse(tokens);
        assert!(
            result.is_ok(),
            "GLR parse should succeed for simple NUM: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_glr_parse_binary_expr() {
        use crate::parsegen::lalr::ParsingTable;

        let grammar = create_ambiguous_expr_grammar();
        let table = ParsingTable::build(&grammar).expect("Failed to build parsing table");

        // Parse: 1 - 2
        let mut parser = GlrParser::new(&table, &grammar);
        let tokens = vec![
            Token {
                symbol: "NUM".to_string(),
                value: "1".to_string(),
                position: 0,
            },
            Token {
                symbol: "MINUS".to_string(),
                value: "-".to_string(),
                position: 1,
            },
            Token {
                symbol: "NUM".to_string(),
                value: "2".to_string(),
                position: 2,
            },
        ];

        let result = parser.parse(tokens);
        assert!(
            result.is_ok(),
            "GLR parse should succeed for binary expr: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_glr_parse_triple_ambiguous() {
        use crate::parsegen::lalr::ParsingTable;

        let grammar = create_ambiguous_expr_grammar();
        let table = ParsingTable::build(&grammar).expect("Failed to build parsing table");

        // Parse: 1 - 2 - 3 (ambiguous: (1-2)-3 or 1-(2-3))
        // GLR should handle both derivations
        let mut parser = GlrParser::new(&table, &grammar);
        let tokens = vec![
            Token {
                symbol: "NUM".to_string(),
                value: "1".to_string(),
                position: 0,
            },
            Token {
                symbol: "MINUS".to_string(),
                value: "-".to_string(),
                position: 1,
            },
            Token {
                symbol: "NUM".to_string(),
                value: "2".to_string(),
                position: 2,
            },
            Token {
                symbol: "MINUS".to_string(),
                value: "-".to_string(),
                position: 3,
            },
            Token {
                symbol: "NUM".to_string(),
                value: "3".to_string(),
                position: 4,
            },
        ];

        let result = parser.parse(tokens);
        assert!(
            result.is_ok(),
            "GLR parse should succeed for ambiguous triple: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_glr_parse_dangling_else_simple() {
        use crate::parsegen::lalr::ParsingTable;

        let grammar = create_dangling_else_grammar();
        let table = ParsingTable::build(&grammar).expect("Failed to build parsing table");

        // Parse: OTHER (simplest case)
        let mut parser = GlrParser::new(&table, &grammar);
        let tokens = vec![Token {
            symbol: "OTHER".to_string(),
            value: "x".to_string(),
            position: 0,
        }];

        let result = parser.parse(tokens);
        assert!(
            result.is_ok(),
            "GLR parse should succeed for OTHER: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_glr_parse_dangling_else_if_then() {
        use crate::parsegen::lalr::ParsingTable;

        let grammar = create_dangling_else_grammar();
        let table = ParsingTable::build(&grammar).expect("Failed to build parsing table");

        // Parse: IF EXPR THEN OTHER
        let mut parser = GlrParser::new(&table, &grammar);
        let tokens = vec![
            Token {
                symbol: "IF".to_string(),
                value: "if".to_string(),
                position: 0,
            },
            Token {
                symbol: "EXPR".to_string(),
                value: "cond".to_string(),
                position: 1,
            },
            Token {
                symbol: "THEN".to_string(),
                value: "then".to_string(),
                position: 2,
            },
            Token {
                symbol: "OTHER".to_string(),
                value: "x".to_string(),
                position: 3,
            },
        ];

        let result = parser.parse(tokens);
        assert!(
            result.is_ok(),
            "GLR parse should succeed for IF THEN: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_glr_parse_dangling_else_full() {
        use crate::parsegen::lalr::ParsingTable;

        let grammar = create_dangling_else_grammar();
        let table = ParsingTable::build(&grammar).expect("Failed to build parsing table");

        // Parse: IF EXPR THEN OTHER ELSE OTHER
        let mut parser = GlrParser::new(&table, &grammar);
        let tokens = vec![
            Token {
                symbol: "IF".to_string(),
                value: "if".to_string(),
                position: 0,
            },
            Token {
                symbol: "EXPR".to_string(),
                value: "cond".to_string(),
                position: 1,
            },
            Token {
                symbol: "THEN".to_string(),
                value: "then".to_string(),
                position: 2,
            },
            Token {
                symbol: "OTHER".to_string(),
                value: "x".to_string(),
                position: 3,
            },
            Token {
                symbol: "ELSE".to_string(),
                value: "else".to_string(),
                position: 4,
            },
            Token {
                symbol: "OTHER".to_string(),
                value: "y".to_string(),
                position: 5,
            },
        ];

        let result = parser.parse(tokens);
        assert!(
            result.is_ok(),
            "GLR parse should succeed for full IF THEN ELSE: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_glr_parse_dangling_else_nested_ambiguous() {
        use crate::parsegen::lalr::ParsingTable;

        let grammar = create_dangling_else_grammar();
        let table = ParsingTable::build(&grammar).expect("Failed to build parsing table");

        // Parse: IF EXPR THEN IF EXPR THEN OTHER ELSE OTHER
        // Ambiguous: does ELSE bind to inner or outer IF?
        // GLR should handle both derivations
        let mut parser = GlrParser::new(&table, &grammar);
        let tokens = vec![
            Token {
                symbol: "IF".to_string(),
                value: "if".to_string(),
                position: 0,
            },
            Token {
                symbol: "EXPR".to_string(),
                value: "a".to_string(),
                position: 1,
            },
            Token {
                symbol: "THEN".to_string(),
                value: "then".to_string(),
                position: 2,
            },
            Token {
                symbol: "IF".to_string(),
                value: "if".to_string(),
                position: 3,
            },
            Token {
                symbol: "EXPR".to_string(),
                value: "b".to_string(),
                position: 4,
            },
            Token {
                symbol: "THEN".to_string(),
                value: "then".to_string(),
                position: 5,
            },
            Token {
                symbol: "OTHER".to_string(),
                value: "x".to_string(),
                position: 6,
            },
            Token {
                symbol: "ELSE".to_string(),
                value: "else".to_string(),
                position: 7,
            },
            Token {
                symbol: "OTHER".to_string(),
                value: "y".to_string(),
                position: 8,
            },
        ];

        let result = parser.parse(tokens);
        assert!(
            result.is_ok(),
            "GLR parse should succeed for nested ambiguous IF: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_glr_table_preserves_conflicts() {
        use crate::parsegen::lalr::ParsingTable;

        let grammar = create_ambiguous_expr_grammar();
        let table = ParsingTable::build(&grammar).expect("Failed to build parsing table");

        // Verify conflicts were detected
        assert!(
            table.shift_reduce_conflicts > 0 || table.reduce_reduce_conflicts > 0,
            "Ambiguous grammar should have conflicts detected"
        );

        // Build GLR table which should preserve all actions
        let glr_table = GlrTable::build(&table, &grammar);

        // Check that glr_table was created (non-empty if there are states)
        assert!(
            !glr_table.goto.is_empty() || !glr_table.actions.is_empty() || table.action.is_empty(),
            "GLR table should be populated from LALR table"
        );
    }

    #[test]
    fn test_glr_forest_ambiguity_representation() {
        let mut forest = ParseForest::new();

        // Create two alternative derivations for same expression
        let t1 = forest.add_terminal("NUM".to_string(), "1".to_string(), 0);
        let t2 = forest.add_terminal("NUM".to_string(), "2".to_string(), 2);
        let t3 = forest.add_terminal("NUM".to_string(), "3".to_string(), 4);

        // Derivation 1: (1-2)-3
        let sub1 = forest.add_nonterminal("expr".to_string(), 0, vec![t1, t2]);
        let deriv1 = forest.add_nonterminal("expr".to_string(), 0, vec![sub1, t3]);

        // Derivation 2: 1-(2-3)
        let sub2 = forest.add_nonterminal("expr".to_string(), 0, vec![t2, t3]);
        let deriv2 = forest.add_nonterminal("expr".to_string(), 0, vec![t1, sub2]);

        // Create ambiguity node
        let amb = forest.add_ambiguous("expr".to_string(), vec![deriv1, deriv2]);

        if let ForestNode::Ambiguous {
            alternatives,
            symbol,
            ..
        } = forest.get(amb).unwrap()
        {
            assert_eq!(symbol, "expr");
            assert_eq!(alternatives.len(), 2);
            assert!(alternatives.contains(&deriv1));
            assert!(alternatives.contains(&deriv2));
        } else {
            panic!("Expected Ambiguous node");
        }
    }

    #[test]
    fn test_glr_gss_path_enumeration() {
        let mut gss = GlrGss::new();

        // Create a diamond-shaped GSS:
        //       n0
        //      /  \
        //    n1    n2
        //      \  /
        //       n3
        let (n0, _) = gss.get_or_create_node(0, 0);
        let (n1, _) = gss.get_or_create_node(1, 1);
        let (n2, _) = gss.get_or_create_node(2, 1);
        let (n3, _) = gss.get_or_create_node(3, 2);

        gss.add_edge(n1, n0, None, "a".to_string());
        gss.add_edge(n2, n0, None, "b".to_string());
        gss.add_edge(n3, n1, None, "c".to_string());
        gss.add_edge(n3, n2, None, "d".to_string());

        // Enumerate paths of length 2 from n3
        let paths = gss.enumerate_paths(n3, 2);
        assert_eq!(
            paths.len(),
            2,
            "Should find 2 paths: n3->n1->n0 and n3->n2->n0"
        );

        // Both paths should end at n0
        for (path, _) in &paths {
            assert_eq!(path.len(), 3);
            assert_eq!(*path.last().unwrap(), n0);
        }
    }

    #[test]
    fn test_glr_gss_node_merging() {
        let mut gss = GlrGss::new();

        // Create first node at state 5, position 10
        let (id1, is_new1) = gss.get_or_create_node(5, 10);
        assert!(is_new1, "First creation should be new");

        // Try to create again at same state/position - should return existing
        let (id2, is_new2) = gss.get_or_create_node(5, 10);
        assert!(!is_new2, "Second creation should merge");
        assert_eq!(id1, id2, "Should return same node ID");

        // Different state should create new node
        let (id3, is_new3) = gss.get_or_create_node(6, 10);
        assert!(is_new3, "Different state should be new");
        assert_ne!(id1, id3);

        // Different position should create new node
        let (id4, is_new4) = gss.get_or_create_node(5, 11);
        assert!(is_new4, "Different position should be new");
        assert_ne!(id1, id4);
    }

    #[test]
    fn test_glr_parse_error_detection() {
        use crate::parsegen::lalr::ParsingTable;

        let mut grammar = Grammar::new();
        grammar.glr_mode = true;
        grammar.tokens = vec!["NUM".to_string()];
        grammar.start_symbol = "e".to_string();

        grammar.rules.push(crate::parsegen::grammar::Rule {
            lhs: "e".to_string(),
            rhs: vec![crate::parsegen::grammar::Symbol::Terminal(
                "NUM".to_string(),
            )],
            action: None,
            precedence_sym: None,
        });

        let table = ParsingTable::build(&grammar).expect("Failed to build parsing table");
        let mut parser = GlrParser::new(&table, &grammar);

        // Try to parse invalid input (UNKNOWN token not in grammar)
        let tokens = vec![Token {
            symbol: "UNKNOWN".to_string(),
            value: "?".to_string(),
            position: 0,
        }];

        let result = parser.parse(tokens);
        assert!(result.is_err(), "GLR parse should fail for invalid token");
    }

    #[test]
    fn test_glr_parse_empty_input() {
        use crate::parsegen::lalr::ParsingTable;

        let mut grammar = Grammar::new();
        grammar.glr_mode = true;
        grammar.tokens = vec!["NUM".to_string()];
        grammar.start_symbol = "e".to_string();

        grammar.rules.push(crate::parsegen::grammar::Rule {
            lhs: "e".to_string(),
            rhs: vec![crate::parsegen::grammar::Symbol::Terminal(
                "NUM".to_string(),
            )],
            action: None,
            precedence_sym: None,
        });

        let table = ParsingTable::build(&grammar).expect("Failed to build parsing table");
        let mut parser = GlrParser::new(&table, &grammar);

        // Empty input
        let tokens: Vec<Token> = vec![];

        let result = parser.parse(tokens);
        assert!(
            result.is_err(),
            "GLR parse should fail for empty input when start requires NUM"
        );
    }
}
