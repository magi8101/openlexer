# Architecture and Internals

This page explains how OpenLexer works internally. Understanding these concepts helps you write better specifications and debug complex issues.

## System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      OpenLexer                              │
├─────────────────────────────┬───────────────────────────────┤
│        Lexer Generator      │       Parser Generator        │
│          (lexgen)           │         (parsegen)            │
├─────────────────────────────┼───────────────────────────────┤
│ 1. Parse .l file            │ 1. Parse .y file              │
│ 2. Compile regexes → NFA    │ 2. Build grammar structure    │
│ 3. NFA → DFA conversion     │ 3. Compute FIRST/FOLLOW sets  │
│ 4. DFA minimization         │ 4. Build LR(0) item sets      │
│ 5. Generate code            │ 5. Build LALR(1) tables       │
│                             │ 6. Generate code              │
└─────────────────────────────┴───────────────────────────────┘
```

## Lexer Generation Pipeline

### Stage 1: Parsing the .l File

The lexer spec parser (`lexgen::rules`) extracts:

- **Definitions**: Named patterns like `DIGIT [0-9]`
- **Start conditions**: `%x COMMENT`, `%s STRING`  
- **Rules**: Pattern-action pairs
- **Prologue/Epilogue**: User code to embed

```rust
// Internal representation
struct LexerSpec {
    rules: Vec<Rule>,           // Pattern → Action mappings
    definitions: HashMap<String, String>,
    start_conditions: Vec<StartCondition>,
    prologue: String,
    epilogue: String,
}
```

### Stage 2: Regex → NFA (Thompson's Construction)

Each regex pattern is converted to a **Nondeterministic Finite Automaton**:

```
Regex: a(b|c)*

        ε         ε
    ┌──────► 2 ──► 3 ─┐
    │        │b       │ε
    │        ▼        │
1 ──┤   ε   4 ────────┼──► 5
 a  │        │c       │
    │        ▼        │
    └──────► 6 ──► 7 ─┘
             ε
```

Key operations:
- **Concatenation**: Chain NFAs sequentially
- **Alternation (`|`)**: Parallel paths with ε-transitions
- **Kleene star (`*`)**: Loop back with ε-transitions
- **Character classes**: Single state with multiple outgoing edges

### Stage 3: NFA → DFA (Subset Construction)

The combined NFA is converted to a **Deterministic Finite Automaton**:

1. Compute **ε-closure** of start state → initial DFA state
2. For each input symbol, compute **move** then ε-closure
3. New state sets become new DFA states
4. Repeat until no new states

```
NFA States {1,2,3} on input 'a' → NFA States {4,5,6,7}
                                 ↓
                         DFA State D2
```

### Stage 4: DFA Minimization (Hopcroft's Algorithm)

The DFA is minimized to reduce state count:

1. Partition states into accepting/non-accepting
2. Refine partitions based on transitions
3. Merge equivalent states

This reduces memory usage and improves runtime performance.

### Stage 5: Code Generation

The DFA is emitted as:

- **Transition table**: 2D array `table[state][char] → next_state`
- **Accept table**: `accept[state] → token_type or -1`
- **Lexer driver**: Loop that walks the DFA

## Parser Generation Pipeline

### Stage 1: Parsing the .y File

The grammar parser (`parsegen::grammar`) extracts:

- **Token declarations**: `%token NUMBER PLUS`
- **Precedence rules**: `%left PLUS MINUS`, `%right POWER`
- **Grammar rules**: Productions with semantic actions
- **Start symbol**: First rule's LHS or explicit `%start`

```rust
// Internal representation
struct Grammar {
    terminals: Vec<Symbol>,
    nonterminals: Vec<Symbol>,
    rules: Vec<Production>,
    precedence: Vec<PrecedenceLevel>,
    start: Symbol,
}
```

### Stage 2: FIRST and FOLLOW Sets

**FIRST(X)**: Set of terminals that can begin strings derived from X

```
FIRST(expr) = { NUMBER, LPAREN, MINUS }
```

**FOLLOW(X)**: Set of terminals that can appear after X

```
FOLLOW(expr) = { PLUS, MINUS, TIMES, RPAREN, $ }
```

These sets drive lookahead decisions in the parser.

### Stage 3: LR(0) Item Sets

An **LR(0) item** is a production with a dot showing parsing progress:

```
expr → expr • + term    (we've seen expr, expecting +)
expr → expr + • term    (we've seen expr +, expecting term)
expr → expr + term •    (complete, ready to reduce)
```

The parser builds the **canonical collection** of item sets:

```
State I0:
  S' → • expr
  expr → • expr + term
  expr → • term
  term → • NUMBER

State I1: (after seeing expr)
  S' → expr •
  expr → expr • + term
```

### Stage 4: LALR(1) Tables

LALR(1) adds lookahead to resolve conflicts:

```
ACTION Table: state × terminal → shift/reduce/accept/error
GOTO Table:   state × nonterminal → next state
```

Example:
```
State 5, lookahead '+': SHIFT to state 7
State 5, lookahead '$': REDUCE by rule expr → expr + term
```

### Stage 5: Conflict Resolution

**Shift/Reduce Conflicts**: Resolved by precedence and associativity

```yacc
%left PLUS MINUS      /* Left-associative, same precedence */
%left TIMES DIVIDE    /* Higher precedence than +/- */
%right POWER          /* Right-associative */
```

**Reduce/Reduce Conflicts**: First matching rule wins (or use GLR)

### Stage 6: Code Generation

The parser is emitted as:

- **ACTION table**: Encodes shift/reduce/accept/error
- **GOTO table**: State transitions on nonterminals
- **Reduction logic**: Switch statement executing semantic actions
- **Stack management**: State and value stacks

## GLR Parsing

When `%glr-parser` is specified, conflicts create **multiple parse paths**:

```
                     ┌─ Parse path A (shift)
Input: "a - b - c" ──┤
                     └─ Parse path B (reduce)
```

All paths are explored in parallel. Invalid paths die when they encounter errors. If multiple paths succeed, the grammar is ambiguous.

## DFA Transition Table Encoding

The generated transition table uses efficient encoding:

### Dense Table (Default)
```c
// Full 256-entry table per state
int table[NUM_STATES][256] = {
    { -1, -1, ..., 5, 5, ..., 3, 3, ... },  // state 0
    { -1, -1, ..., 2, 2, ..., 7, 7, ... },  // state 1
};
```

### Optimized Encoding (When Enabled)
- **Row compression**: Factor out common default transitions
- **Column compression**: Merge equivalent character columns
- **Direct-coded DFA**: Generate switch statements instead of tables

## Token Matching Strategy

OpenLexer uses **maximal munch** (longest match) with **rule priority**:

1. Try all patterns from current position
2. Select the longest matching pattern
3. On ties, prefer the pattern defined first in the `.l` file
4. Execute the associated action
5. Advance position and repeat

## Memory Layout

### Generated Lexer

```
┌────────────────────────────┐
│ Input buffer (user string) │
├────────────────────────────┤
│ Position tracking          │
│ (pos, line, column)        │
├────────────────────────────┤
│ Start condition stack      │
├────────────────────────────┤
│ Token result buffer        │
└────────────────────────────┘
```

### Generated Parser

```
┌────────────────────────────┐
│ State stack                │
│ (current parsing states)   │
├────────────────────────────┤
│ Value stack                │
│ (semantic values for $1,$2)│
├────────────────────────────┤
│ Lookahead token            │
├────────────────────────────┤
│ Error recovery state       │
└────────────────────────────┘
```

## Performance Characteristics

| Operation | Lexer | Parser |
|-----------|-------|--------|
| Time complexity | O(n) | O(n) for LALR, O(n³) worst-case for GLR |
| Space complexity | O(states × alphabet) | O(states × symbols) |
| Typical states | 50-500 | 100-2000 |

## Error Handling

### Lexer Errors

When no pattern matches:
1. Report position with line/column
2. Skip one character (or configurable recovery)
3. Continue lexing

### Parser Errors

When no valid action exists:
1. Report token and expected tokens
2. Pop states until error recovery possible
3. Skip tokens until synchronization point
4. Resume parsing

## File Format References

The `.l` and `.y` formats are compatible with **Flex** and **Bison** respectively, with these extensions:

### OpenLexer Extensions to Flex
- `%unicode` - Enable Unicode mode
- Full Unicode character class support (`\p{L}`, `\p{Greek}`)

### OpenLexer Extensions to Bison  
- `%glr-parser` - Enable GLR parsing
- Multi-language output (Python, Java, C)

## Internal Module Structure

```
openlexer/
├── src/
│   ├── lib.rs          # Public API
│   ├── error.rs        # Error types
│   ├── lexgen/         # Lexer generator
│   │   ├── mod.rs      # Public interface
│   │   ├── regex.rs    # Regex parser
│   │   ├── nfa.rs      # NFA construction
│   │   ├── dfa.rs      # DFA conversion + minimization
│   │   ├── rules.rs    # .l file parser
│   │   └── codegen.rs  # Code generation
│   └── parsegen/       # Parser generator
│       ├── mod.rs      # Public interface
│       ├── grammar.rs  # .y file parser
│       ├── first.rs    # FIRST/FOLLOW computation
│       ├── lalr.rs     # LALR(1) table construction
│       └── codegen.rs  # Code generation
```
