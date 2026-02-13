# OpenLexer Future Implementations

This document outlines planned enhancements to address current limitations compared to Flex and Bison.

---

## Phase 1: Parser Enhancements

### 1.1 GLR Parser Support [DONE]
**Current**: LALR(1) only  
**Target**: Add GLR option for ambiguous grammars

**Algorithm**: Graph-Structured Stack (GSS)
- When a shift/reduce or reduce/reduce conflict occurs, the parser "forks" into multiple parallel parsers
- Each parser follows a different path, stored as a tree/DAG of states (GSS)
- Parsers proceed in lock-step, consuming tokens together
- Invalid paths die when they encounter errors
- Valid paths merge when they reach the same state

**Implementation**:
1. Add `%glr-parser` directive parsing in `grammar.rs`
2. Create new `src/parsegen/glr.rs` with:
   - `GraphStructuredStack` struct for managing parallel parse stacks
   - `GlrParser` struct with fork/merge logic
   - Semantic action deferral (actions stored but not executed until path chosen)
3. Modify codegen to generate GLR tables when enabled
4. Add `%dprec` (declarative precedence) for resolving merge conflicts
5. Add `%merge` for custom semantic action merging

**New File**: `src/parsegen/glr.rs`

---

### 1.2 Dynamic Stack Size in Generated C
**Current**: Fixed `STACK_SIZE = 1024`  
**Target**: Growable stack with `realloc`

**Implementation** in `src/parsegen/codegen.rs`:
```c
#define INITIAL_STACK_SIZE 128
#define YYMAXDEPTH 10000

StackItem *stack = NULL;
int stack_size = 0;
int top = 0;

void ensure_stack_capacity(int needed) {
    if (needed > YYMAXDEPTH) {
        fprintf(stderr, "Stack overflow\n");
        exit(1);
    }
    if (needed > stack_size) {
        int new_size = stack_size ? stack_size * 2 : INITIAL_STACK_SIZE;
        while (new_size < needed) new_size *= 2;
        stack = realloc(stack, new_size * sizeof(StackItem));
        stack_size = new_size;
    }
}
```

---

### 1.3 Advanced Error Recovery
**Current**: Basic `error` token  
**Target**: LAC (Lookahead Correction) + better synchronization

**Implementation**:
1. Add `%define parse.error detailed` support
2. Implement LAC algorithm in parser codegen:
   - On error, explore what tokens would be valid using trial parsing
   - Report expected tokens accurately
3. Add `%destructor` directive for memory cleanup on error
4. Add `yyerrok`, `yyclearin` macros to generated code
5. Implement custom error reporter function (`yyreport_syntax_error`)

**New File**: `src/parsegen/error_recovery.rs`

---

### 1.4 Complex Semantic Types (%union)
**Current**: Assumes `int` for all values  
**Target**: Support `%union` and typed symbols

**Implementation**:
1. Parse `%union { ... }` in `grammar.rs`
2. Store type tags for tokens/nonterminals: `%token <ival> NUMBER`, `%nterm <sval> expr`
3. Modify codegen to:
   - Generate the union typedef
   - Generate typed stack entries
   - Use correct union member in `$$`, `$1`, `$2`, etc.

**Example**:
```yacc
%union {
    int ival;
    double dval;
    char *sval;
    struct node *nval;
}

%token <ival> INTEGER
%token <dval> FLOAT
%token <sval> STRING
%nterm <nval> expr
```

---

## Phase 2: Lexer Enhancements

### 2.1 Arbitrary User Code in Actions
**Current**: `RuleAction` enum limits to Token/Skip/Begin/Error  
**Target**: Allow arbitrary code blocks `{ ... }`

**Implementation**:
1. Modify `rules.rs` to parse raw code blocks
2. Add `RuleAction::Code(String)` variant
3. Codegen passes through the code with variable substitution:
   - `yytext` - matched text
   - `yyleng` - match length
   - `REJECT` - try next rule
   - `yymore()` - append next match
   - `yyless(n)` - put back characters
   - `BEGIN(state)` - switch condition

---

### 2.2 Advanced Flex Features (yymore, yyless, REJECT)

**yymore** - Append next match to current `yytext`:
```c
int yymore_flag = 0;
#define yymore() (yymore_flag = 1)
```

**yyless** - Put back characters:
```c
#define yyless(n) do { \
    pos -= (yyleng - (n)); \
    yyleng = (n); \
} while(0)
```

**REJECT** - Try next best match:
- Requires storing all matches during DFA simulation, not just longest
- On REJECT, restore state and try next-best accepting state

---

### 2.3 Unicode/UTF-8 Support [DONE]
**Current**: ASCII only (0-127)  
**Target**: Full Unicode support

**Implementation**:
1. Add `unicode-segmentation` crate to Cargo.toml
2. Modify `regex.rs`:
   - Parse Unicode escapes: `\u{XXXX}`, `\p{Category}`
   - Support Unicode character classes: `\p{L}` (Letter), `\p{Nd}` (Decimal Number)
3. Modify `nfa.rs`:
   - Use `char` (32-bit Unicode code point) instead of ASCII byte
   - Add Unicode-aware transitions
4. Modify `CharClass::expand()` to support Unicode ranges
5. Update DFA for larger alphabets using range-based transitions

**Alphabet Compression Strategy**:
- Group Unicode code points into equivalence classes
- Store transitions as ranges `[start_char, end_char) -> state`
- Use interval trees for efficient lookup

**New File**: `src/lexgen/unicode.rs`

---

## Phase 3: Library & Tooling

### 3.1 Runtime Libraries (-lfl, -ly equivalents)
**Target**: Provide default `main()`, `yywrap()`, `yyerror()`

**Implementation**:
1. Create template library files:
   - `lib/c/libol_lexer.c` - default `yywrap()`, `main()` for lexers
   - `lib/c/libol_parser.c` - default `yyerror()`, `main()` for parsers
2. Add CLI option: `--library` or `-l` to include library code

**New Files**:
- `lib/c/libol.h`
- `lib/c/libol_lexer.c`
- `lib/c/libol_parser.c`

---

### 3.2 Location Tracking
**Target**: Track line/column numbers for tokens and rules

**Implementation**:
1. Add `%locations` directive support
2. Generate `YYLTYPE` struct:
```c
typedef struct YYLTYPE {
    int first_line;
    int first_column;
    int last_line;
    int last_column;
} YYLTYPE;
```
3. Generate `yylloc` variable in lexer
4. Support `@1`, `@2`, `@$` in grammar actions for location access

---

## Implementation Priority

| # | Feature | Complexity | Impact | Status |
|---|---------|------------|--------|--------|
| 1 | Dynamic Stack | Low | Medium | **Done** |
| 2 | %union Support | Medium | High | **Done** |
| 3 | Error Recovery (LAC) | Medium | High | **Done** |
| 4 | User Code Actions | Medium | High | **Done** |
| 5 | Unicode Support | High | Medium | **Done** |
| 6 | yymore/yyless/REJECT | Medium | Low | **Done** |
| 7 | GLR Parser | High | Medium | **Done** |
| 8 | Location Tracking | Medium | Medium | **Done** |
| 9 | Runtime Libraries | Low | Low | **Done** (C, Python, Java)

---

## Files Summary

### New Files to Create
| File | Purpose | Status |
|------|---------|--------|
| `src/parsegen/glr.rs` | GLR parsing algorithm | **Done** |
| `src/lexgen/unicode.rs` | Unicode support utilities | **Done** |
| `lib/c/libol.h` | Common C header | **Done** |
| `lib/c/libol_lexer.c` | Lexer runtime library | **Done** |
| `lib/c/libol_parser.c` | Parser runtime library | **Done** |
| `lib/python/libol.py` | Python runtime library | **Done** |
| `lib/java/org/openlexer/runtime/*.java` | Java runtime package | **Done** |

### Files to Modify
| File | Changes |
|------|---------|
| `src/parsegen/grammar.rs` | Parse %union, %glr-parser, %locations, %destructor |
| `src/parsegen/codegen.rs` | Dynamic stack, union types, error recovery |
| `src/parsegen/lalr.rs` | Export conflicts for GLR |
| `src/lexgen/rules.rs` | User code actions, yymore/yyless/REJECT |
| `src/lexgen/regex.rs` | Unicode patterns |
| `src/lexgen/nfa.rs` | Unicode transitions |
| `src/lexgen/dfa.rs` | Range-based transitions for Unicode |
| `src/lexgen/codegen.rs` | Advanced lexer features |

---

## References

- [GNU Bison Manual](https://www.gnu.org/software/bison/manual/bison.html)
- [Flex Manual](https://westes.github.io/flex/manual/)
- Tomita, M. (1985). "Efficient Parsing for Natural Language" - GLR algorithm
- DeRemer, F. & Pennello, T. (1982). "Efficient Computation of LALR(1) Look-Ahead Sets"
