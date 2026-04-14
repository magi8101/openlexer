use anyhow::Result;
use openlexer_lib::parsegen::{Grammar, ParsingTable};
use std::fs;

fn main() -> Result<()> {
    let content = fs::read_to_string("/tmp/test.y")?;
    let grammar = Grammar::parse(&content)?;
    
    println!("Tokens: {:?}", grammar.tokens);
    println!("Precedence rules: {:?}", grammar.precedence);
    for (i, rule) in grammar.rules.iter().enumerate() {
        println!("Rule {}: {} -> {:?} (prec: {:?})", i, rule.lhs, rule.rhs, rule.precedence_sym);
    }
    
    let table = ParsingTable::build(&grammar)?;
    println!("States: {}", table.action.len());
    println!("Shift-Reduce Conflicts (unresolved): {}", table.shift_reduce_conflicts);
    let mut total_glr_conflicts = 0;
    for row in table.glr_conflict_actions.values() {
        total_glr_conflicts += row.len();
    }
    println!("GLR Conflicts: {}", total_glr_conflicts);
    Ok(())
}
