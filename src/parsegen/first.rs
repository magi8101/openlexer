//! FIRST and FOLLOW set computation.

use crate::parsegen::grammar::{Grammar, Symbol};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct FirstFollow {
    pub first: HashMap<String, HashSet<String>>,
    pub follow: HashMap<String, HashSet<String>>,
}

impl FirstFollow {
    pub fn new(grammar: &Grammar) -> Self {
        let mut ff = Self {
            first: HashMap::new(),
            follow: HashMap::new(),
        };
        
        // Initialize sets
        for rule in &grammar.rules {
            ff.first.entry(rule.lhs.clone()).or_default();
            ff.follow.entry(rule.lhs.clone()).or_default();
            
            for sym in &rule.rhs {
                if let Symbol::NonTerminal(nt) = sym {
                    ff.first.entry(nt.clone()).or_default();
                    ff.follow.entry(nt.clone()).or_default();
                }
            }
        }
        
        ff.compute_first(grammar);
        ff.compute_follow(grammar);
        
        ff
    }

    fn compute_first(&mut self, grammar: &Grammar) {
        let mut changed = true;
        while changed {
            changed = false;
            
            for rule in &grammar.rules {
                let lhs = &rule.lhs;
                let mut rhs_nullable = true;
                
                // For each symbol in RHS
                for sym in &rule.rhs {
                    let mut sym_first = HashSet::new();
                    
                    match sym {
                        Symbol::Terminal(t) => {
                            sym_first.insert(t.clone());
                            rhs_nullable = false;
                        }
                        Symbol::NonTerminal(nt) => {
                            if let Some(set) = self.first.get(nt) {
                                sym_first = set.clone();
                            }
                            // Check if this NT contains epsilon (not implemented explicitly yet, assume no epsilon for MVP unless empty rule)
                            if !sym_first.contains("EPSILON") {
                                rhs_nullable = false;
                            }
                        }
                    }
                    
                    // Add sym_first to FIRST(lhs)
                    let lhs_first = self.first.entry(lhs.clone()).or_default();
                    let len_before = lhs_first.len();
                    lhs_first.extend(sym_first);
                    if lhs_first.len() > len_before {
                        changed = true;
                    }
                    
                    if !rhs_nullable {
                        break;
                    }
                }
                
                // If entire RHS is nullable (or empty), add epsilon
                if rule.rhs.is_empty() {
                    let lhs_first = self.first.entry(lhs.clone()).or_default();
                     if !lhs_first.contains("EPSILON") {
                        lhs_first.insert("EPSILON".to_string());
                        changed = true;
                     }
                }
            }
        }
    }

    fn compute_follow(&mut self, grammar: &Grammar) {
        // Start symbol gets EOF ($)
        self.follow.entry(grammar.start_symbol.clone()).or_default().insert("$".to_string());
        
        let mut changed = true;
        while changed {
            changed = false;
            
            // Collect all updates first to avoid conflicting borrows
            let mut updates: Vec<(String, HashSet<String>)> = Vec::new();
            
            for rule in &grammar.rules {
                let lhs = &rule.lhs;
                
                // We need FOLLOW(lhs) to propagate to end of RHS
                // We can clone it efficiently here since we are inside the loop
                // Clone follow_lhs here to avoid borrowing self.follow across the inner loop
                let follow_lhs = self.follow.get(lhs).cloned().unwrap_or_default();
                
                for (i, sym) in rule.rhs.iter().enumerate() {
                    if let Symbol::NonTerminal(nt) = sym {
                        let mut trailer = HashSet::new();
                        let mut trailer_nullable = true;
                        
                        // Look ahead at symbols following this NT
                        for next_sym in rule.rhs.iter().skip(i + 1) {
                            match next_sym {
                                Symbol::Terminal(t) => {
                                    trailer.insert(t.clone());
                                    trailer_nullable = false;
                                    break;
                                }
                                Symbol::NonTerminal(next_nt) => {
                                    // Clone first_next here to avoid borrowing self.first across the inner loop
                                    if let Some(first_next) = self.first.get(next_nt).cloned() {
                                        for f in &first_next {
                                            if f != "EPSILON" {
                                                trailer.insert(f.clone());
                                            }
                                        }
                                        if !first_next.contains("EPSILON") {
                                            trailer_nullable = false;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        
                        // If everything after nt is nullable, it inherits FOLLOW(lhs)
                        if trailer_nullable {
                            trailer.extend(follow_lhs.clone());
                        }
                        
                        if !trailer.is_empty() {
                            updates.push((nt.clone(), trailer));
                        }
                    }
                }
            }
            
            // Apply updates
            for (nt, new_syms) in updates {
                let follow_set = self.follow.entry(nt).or_default();
                let len_before = follow_set.len();
                follow_set.extend(new_syms);
                if follow_set.len() > len_before {
                    changed = true;
                }
            }
        }
    }
}
