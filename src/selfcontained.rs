//! Experimental "self-contained parser" support: given a grammar's `%token`
//! declarations, guess a lexer rule for each token so a `.y` file with no
//! matching `.l` file can still get a usable (if approximate) lexer.
//!
//! This is deliberately opt-in and clearly labeled "experimental" in the
//! GUI - guessed patterns are a starting point to review and edit, not a
//! substitute for writing a real `.l` file. Every guess records where it
//! came from so the caller can show its confidence.

use crate::parsegen::grammar::Grammar;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuessSource {
    /// The grammar itself gave an exact literal, e.g. `%token PLUS '+'`
    /// or `'+'` used directly in a rule.
    Literal,
    /// The token's name matched a common naming convention (NUMBER,
    /// IDENTIFIER, STRING, ...) with a known regex pattern.
    KnownConvention,
    /// No literal or known convention matched; the lowercased token name
    /// itself is guessed as a literal keyword (e.g. IF -> "if").
    KeywordFallback,
}

impl GuessSource {
    pub fn label(&self) -> &'static str {
        match self {
            GuessSource::Literal => "from grammar literal",
            GuessSource::KnownConvention => "known convention",
            GuessSource::KeywordFallback => "guessed keyword",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenGuess {
    pub name: String,
    /// A `.l` pattern: either a quoted literal (`"+"`) or a bare regex
    /// (`[0-9]+`), exactly as it would appear before the rule's action.
    pub pattern: String,
    pub source: GuessSource,
}

/// Common token-name conventions and the regex pattern they suggest.
/// Checked case-insensitively; order matters only in that the first match
/// wins, and none of these overlap in practice.
const KNOWN_CONVENTIONS: &[(&[&str], &str)] = &[
    (&["NUMBER", "NUM", "INTEGER", "INT"], "[0-9]+"),
    (&["FLOAT", "DOUBLE", "REAL"], "[0-9]+\\.[0-9]+"),
    (&["IDENTIFIER", "ID", "NAME"], "[a-zA-Z_][a-zA-Z0-9_]*"),
    (&["STRING", "STR", "STRING_LITERAL"], "\"[^\"]*\""),
];

fn known_convention_pattern(name: &str) -> Option<&'static str> {
    let upper = name.to_uppercase();
    KNOWN_CONVENTIONS
        .iter()
        .find(|(names, _)| names.contains(&upper.as_str()))
        .map(|(_, pattern)| *pattern)
}

/// Quote a literal value for use as a `.l` pattern, e.g. `+` -> `"+"`.
/// Doubles any embedded `"` so the quoted text stays well-formed; good
/// enough for the operator/punctuation/keyword literals this is meant for.
fn quote_literal(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

/// Guess a lexer rule for every token the grammar declares, in an order
/// safe to hand to the lexer generator as-is: literal and keyword-fallback
/// rules (specific, exact-text matches) before known-convention rules
/// (broader patterns like IDENTIFIER) - otherwise a keyword guess like
/// "if" would never win against an IDENTIFIER rule matching the same text,
/// since both match the same length and earlier-declared rules win ties.
pub fn infer_token_guesses(grammar: &Grammar) -> Vec<TokenGuess> {
    let mut literal_or_keyword = Vec::new();
    let mut known = Vec::new();

    for name in &grammar.tokens {
        if let Some(literal) = grammar.token_literals.get(name) {
            literal_or_keyword.push(TokenGuess {
                name: name.clone(),
                pattern: quote_literal(literal),
                source: GuessSource::Literal,
            });
        } else if let Some(pattern) = known_convention_pattern(name) {
            known.push(TokenGuess {
                name: name.clone(),
                pattern: pattern.to_string(),
                source: GuessSource::KnownConvention,
            });
        } else {
            literal_or_keyword.push(TokenGuess {
                name: name.clone(),
                pattern: quote_literal(&name.to_lowercase()),
                source: GuessSource::KeywordFallback,
            });
        }
    }

    literal_or_keyword.extend(known);
    literal_or_keyword
}

/// Assembles a full `.l` source from a set of (possibly user-edited)
/// guesses, ready to hand to `LexerSpec::parse`. Always appends a
/// whitespace-skip rule last, since a lexer with no way to skip whitespace
/// is unusable on any realistic input.
pub fn build_lexer_spec_text(guesses: &[TokenGuess]) -> String {
    let mut text = String::new();
    text.push_str("%%\n");
    text.push_str("/* Auto-generated (EXPERIMENTAL) - review before use */\n");
    for guess in guesses {
        text.push_str(&format!(
            "{}  {{ return {}; }}  /* {} */\n",
            guess.pattern,
            guess.name,
            guess.source.label()
        ));
    }
    text.push_str("[ \\t\\r\\n]+  { /* skip whitespace (auto-added) */ }\n");
    text.push_str("%%\n");
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsegen::grammar::Grammar;

    #[test]
    fn literal_beats_keyword_beats_convention() {
        let grammar = Grammar::parse(
            r#"%token NUMBER IDENTIFIER IF
%token PLUS "+"
%left PLUS
%%
expr: NUMBER | IDENTIFIER | PLUS | IF ;
%%
"#,
        )
        .unwrap();

        let guesses = infer_token_guesses(&grammar);
        let by_name = |n: &str| guesses.iter().find(|g| g.name == n).unwrap().clone();

        assert_eq!(by_name("PLUS").source, GuessSource::Literal);
        assert_eq!(by_name("PLUS").pattern, "\"+\"");
        assert_eq!(by_name("IF").source, GuessSource::KeywordFallback);
        assert_eq!(by_name("IF").pattern, "\"if\"");
        assert_eq!(by_name("NUMBER").source, GuessSource::KnownConvention);
        assert_eq!(by_name("IDENTIFIER").source, GuessSource::KnownConvention);

        // Keyword/literal rules must precede convention rules in the
        // assembled text so "if" wins over IDENTIFIER on a tie.
        let text = build_lexer_spec_text(&guesses);
        let if_pos = text.find("\"if\"").unwrap();
        let ident_pos = text.find("[a-zA-Z_]").unwrap();
        assert!(if_pos < ident_pos);

        // The assembled text must actually parse as a lexer spec.
        let spec = crate::lexgen::parse_lexer_spec(&text).unwrap();
        assert_eq!(spec.rules.len(), guesses.len() + 1); // + whitespace skip
    }
}
