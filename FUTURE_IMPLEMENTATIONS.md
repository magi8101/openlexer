# OpenLexer Future Implementations

This document outlines planned enhancements and **current gaps** compared to Flex and Bison.

---

## Current Implementation Status

### What's Working
| Feature | Lexer | Parser |
|---------|-------|--------|
| Basic patterns/rules | Yes | Yes |
| Multi-language output (C/Java/Python) | Yes | Yes |
| Unicode support | Yes | N/A |
| Start conditions | Yes | N/A |
| Precedence/associativity | N/A | Yes |
| GLR parsing | N/A | Yes |
| %union | N/A | Yes |
| %locations | Yes | Yes |
| **LALR(1) parsing** | N/A | **Yes (NEW)** |
| **Reentrant lexers** | **Yes (NEW)** | N/A |
| **%option yylineno** | **Yes (NEW)** | N/A |
| **%option bison-bridge** | **Yes (NEW)** | N/A |
| **%option bison-locations** | **Yes (NEW)** | N/A |
| **Mid-rule actions** | N/A | **Yes (NEW)** |

---

## PHASE 1: Critical Missing Features

### 1.1 Full LALR(1) Instead of SLR(1)
**Status**: IMPLEMENTED (March 2026)

Implemented the DeRemer-Pennello algorithm for efficient LALR(1) lookahead computation:
- LR(0) automaton construction (unchanged)
- Direct Read (DR) sets for each nonterminal transition
- READS relation for nullable symbol handling
- INCLUDES relation for lookahead propagation
- LOOKBACK relation for mapping reductions to transitions
- Transitive closure computation for final lookahead sets

Location: `src/parsegen/lalr.rs`

---

### 1.2 Reentrant/Pure Lexers and Parsers
**Status**: IMPLEMENTED (Lexer - March 2026)

Implemented `%option reentrant` support:
- Scanner context struct (`yyscanner_t`) instead of global state
- All lexer state in context: input, current, yytext, yyleng
- `%option yylineno` - automatic line/column tracking
- `%option bison-bridge` - yylval support
- `%option bison-locations` - YYLTYPE yylloc support
- `%option prefix="xx"` - custom prefix support
- yyextra user data pointer

**Usage**:
```lex
%option reentrant
%option yylineno
%option bison-bridge
%option bison-locations
%%
[a-z]+   { return IDENTIFIER; }
%%
```

**Generated API**:
```c
void yy_lex_init(yyscanner_t* scanner);
void yy_set_input(yyscanner_t* scanner, const char* input);
Token yy_lex(yyscanner_t* scanner);
void yy_lex_destroy(yyscanner_t* scanner);
void* yyget_extra(yyscanner_t* scanner);
void yyset_extra(void* extra, yyscanner_t* scanner);
```

Location: `src/lexgen/codegen.rs`, `src/lexgen/rules.rs`

---

### 1.3 Mid-Rule Actions
**Status**: IMPLEMENTED (March 2026)

Mid-rule actions are now automatically transformed into synthetic epsilon productions.

**Example**:
```yacc
stmt: IF { printf("entering if"); } expr THEN stmt { $$ = make_if($3, $5); }
```

Is automatically transformed to:
```yacc
@1: /* empty */ { printf("entering if"); }
stmt: IF @1 expr THEN stmt { $$ = make_if($3, $5); }
```

Location: `src/parsegen/grammar.rs`

---

### 1.4 Push Parser API
**Status**: NOT IMPLEMENTED
**Current**: Pull parser only (parser calls lexer)
**Impact**: MEDIUM - Required for async/incremental parsing

**Missing API**:
```c
int yypush_parse(yypstate *ps, int token, YYSTYPE *val, YYLTYPE *loc);
int yypull_parse(yypstate *ps);
yypstate *yypstate_new(void);
void yypstate_delete(yypstate *ps);
```

**Bison Directive**:
```yacc
%define api.push-pull push  // or "both"
```

---

### 1.5 Named References in Actions
**Status**: NOT IMPLEMENTED
**Current**: Only `$1`, `$2`, `$$` positional references
**Impact**: MEDIUM - Improves readability for complex rules

**Missing Syntax**:
```yacc
expr[result]: expr[left] PLUS expr[right] { $result = $left + $right; }
```

**Implementation**:
1. Parse `[name]` after symbols in grammar rules
2. Build name-to-position mapping
3. Replace `$name` with `$N` in action code

---

### 1.5 Mid-Rule Actions
**Status**: ❌ NOT IMPLEMENTED (properly)
**Current**: Actions only at end of rules
**Impact**: MEDIUM

**Missing Syntax**:
```yacc
stmt: IF { printf("entering if"); } expr THEN stmt { $$ = make_if($3, $5); }
```

**Implementation**:
Transform to:
```yacc
@1: /* empty */ { printf("entering if"); }
stmt: IF @1 expr THEN stmt { $$ = make_if($3, $5); }
```

---

## PHASE 2: Lexer (Flex) Missing Features

### 2.1 REJECT
**Status**: ⚠️ PARTIAL (detected, not emulated)
**Current**: Detected in code blocks, passed through
**Impact**: LOW - Rarely used

**What's Needed**:
- Store all accepting states during DFA simulation, not just longest
- On REJECT, backtrack and try next-best match
- Significant performance impact when used

---

### 2.2 yymore() and yyless(n)
**Status**: ⚠️ PARTIAL (detected, not emulated)

**yymore()**: Append next match to current yytext
```c
// Missing implementation:
int yymore_flag = 0;
if (yymore_flag) {
    // Don't reset yytext, append new match
    yymore_flag = 0;
}
```

**yyless(n)**: Put back all but first n characters
```c
#define yyless(n) do { \
    input_pos -= (yyleng - (n)); \
    yyleng = (n); \
    yytext[yyleng] = '\0'; \
} while(0)
```

---

### 2.3 Input Buffer Management
**Status**: ❌ NOT IMPLEMENTED
**Impact**: HIGH for real compilers

**Missing Functions**:
```c
YY_BUFFER_STATE yy_create_buffer(FILE *file, int size);
void yy_switch_to_buffer(YY_BUFFER_STATE buf);
void yy_delete_buffer(YY_BUFFER_STATE buf);
YY_BUFFER_STATE yy_scan_string(const char *str);
YY_BUFFER_STATE yy_scan_bytes(const char *bytes, int len);
void yy_flush_buffer(YY_BUFFER_STATE buf);
void yyrestart(FILE *input_file);
```

**Use Case**: Include file handling, string scanning

---

### 2.4 <<EOF>> Rules
**Status**: ❌ NOT IMPLEMENTED

**Syntax**:
```lex
<<EOF>>     { return END_OF_FILE; }
<COMMENT><<EOF>> { error("Unterminated comment"); }
```

**Implementation**:
1. Parse `<<EOF>>` as special pattern
2. Generate end-of-input handling per start condition

---

### 2.5 Trailing Context
**Status**: ❌ NOT IMPLEMENTED

**Syntax**:
```lex
abc/def     { /* match "abc" only if followed by "def" */ }
abc$        { /* match "abc" at end of line */ }
^abc        { /* match "abc" at start of line */ }
```

**Implementation**:
- `r/s`: Match r only if followed by s (don't consume s)
- `$`: Equivalent to `/\n`
- `^`: Track BOL (beginning of line) flag

---

### 2.6 unput(c) and input()
**Status**: ❌ NOT IMPLEMENTED

```c
int input(void);      // Read next character directly
void unput(int c);    // Push character back to input
```

---

### 2.7 Automatic yylineno
**Status**: ❌ NOT IMPLEMENTED

**Flex Option**:
```lex
%option yylineno
```

Automatically increments `yylineno` when newlines are matched.

---

### 2.8 Debug Mode
**Status**: ❌ NOT IMPLEMENTED

**Flex Option**:
```lex
%option debug
```

Generates code that prints matched rules when `yy_flex_debug` is set.

---

### 2.9 Common %option Directives
**Status**: ❌ Most NOT IMPLEMENTED

| Option | Status | Description |
|--------|--------|-------------|
| `%option noyywrap` | ❌ | Don't call yywrap() |
| `%option noinput` | ❌ | Don't generate input() |
| `%option nounput` | ❌ | Don't generate unput() |
| `%option batch` | ❌ | Optimize for batch input |
| `%option interactive` | ❌ | Optimize for interactive |
| `%option case-insensitive` | ❌ | Case-insensitive matching |
| `%option prefix="xx"` | ❌ | Change yy prefix to xx |
| `%option outfile="name"` | ❌ | Specify output file |
| `%option header-file="name"` | ❌ | Generate header file |
| `%option c++` | ❌ | Generate C++ scanner class |
| `%option yyclass="name"` | ❌ | Name of C++ class |

---

## PHASE 3: Parser (Bison) Missing Features

### 3.1 %define Directives
**Status**: ❌ NOT IMPLEMENTED

| Directive | Description |
|-----------|-------------|
| `%define api.pure full` | Generate reentrant parser |
| `%define api.push-pull push` | Generate push parser |
| `%define api.token.constructor` | Token constructors (C++) |
| `%define api.value.type variant` | Use std::variant (C++) |
| `%define parse.error detailed` | Detailed error messages |
| `%define parse.error verbose` | Verbose error messages |
| `%define parse.lac full` | Lookahead correction |
| `%define parse.trace` | Enable tracing |

---

### 3.2 %code Blocks
**Status**: ❌ NOT IMPLEMENTED

```yacc
%code requires {
    // Goes before YYSTYPE definition
    struct Node;
}

%code provides {
    // Goes in header file
    void yyerror(const char *msg);
}

%code top {
    // Goes at very top of output
    #define _GNU_SOURCE
}

%code {
    // Goes in implementation (like %{ %})
    #include "ast.h"
}
```

---

### 3.3 Error Recovery Enhancements
**Status**: ⚠️ PARTIAL

**Missing**:
- `yyerrok` macro (clear error state)
- `yyclearin` macro (discard lookahead)
- `YYERROR` (trigger error from action)
- `YYACCEPT` (accept immediately)
- `YYABORT` (abort immediately)
- `yynerrs` error counter
- LAC (Lookahead Correction) for better expected token lists

---

### 3.4 %expect and %expect-rr
**Status**: ❌ NOT IMPLEMENTED

```yacc
%expect 1        // Expect exactly 1 shift/reduce conflict
%expect-rr 0     // Expect 0 reduce/reduce conflicts
```

Suppresses warnings when conflict count matches expectation.

---

### 3.5 %skeleton Customization
**Status**: ❌ NOT IMPLEMENTED

```yacc
%skeleton "lalr1.cc"    // Use C++ skeleton
%skeleton "glr.c"       // Use GLR skeleton
```

---

### 3.6 %initial-action
**Status**: ❌ NOT IMPLEMENTED

```yacc
%initial-action {
    @$.first_line = @$.last_line = 1;
    @$.first_column = @$.last_column = 0;
}
```

Code executed before parsing starts.

---

### 3.7 %printer Directive
**Status**: ❌ NOT IMPLEMENTED

```yacc
%printer { fprintf(yyoutput, "%d", $$); } <ival>
%printer { fprintf(yyoutput, "%s", $$); } <sval>
```

For debugging - prints symbol values.

---

### 3.8 @N Location References
**Status**: ⚠️ PARTIAL (basic @$ only)

**Missing**:
```yacc
expr: expr PLUS expr {
    @$.first_line = @1.first_line;
    @$.last_line = @3.last_line;
}
```

Need `@1`, `@2`, `@N` access to component locations.

---

### 3.9 Debug/Trace Mode
**Status**: ❌ NOT IMPLEMENTED

```yacc
%define parse.trace
```

Generates:
```c
extern int yydebug;  // Set to 1 for tracing
#define YYDEBUG 1
```

---

### 3.10 Counterexample Generation
**Status**: ❌ NOT IMPLEMENTED

Modern Bison can show input that triggers conflicts:
```
Shift/reduce conflict on token ELSE:
  Example: IF expr THEN IF expr THEN stmt . ELSE stmt
```

---

### 3.11 %language Directive
**Status**: ⚠️ PARTIAL (via CLI only)

```yacc
%language "c++"
%language "java"
```

---

## PHASE 4: Code Generation Quality

### 4.1 Table Compression
**Status**: ❌ NOT IMPLEMENTED
**Current**: Full uncompressed tables
**Impact**: Large output for big grammars

**Techniques to implement**:
1. Default reductions (most common action per state)
2. Row/column compression
3. Sparse table representation
4. Graph coloring for state merging

---

### 4.2 Header File Generation
**Status**: ❌ NOT IMPLEMENTED

Flex: `%option header-file="lexer.h"`
Bison: `%defines "parser.h"`

Should generate:
- Token constants
- YYSTYPE definition
- Function declarations

---

### 4.3 Line Directive (#line)
**Status**: ❌ NOT IMPLEMENTED

```c
#line 42 "grammar.y"
// Errors point to original .y file, not generated .c
```

---

## Implementation Priority (Revised)

| Priority | Feature | Complexity | Impact |
|----------|---------|------------|--------|
| **P0** | LALR(1) instead of SLR(1) | High | Critical |
| **P0** | Reentrant lexers/parsers | High | Critical |
| **P1** | Input buffer management | Medium | High |
| **P1** | Push parser API | Medium | High |
| **P1** | %define directives | Medium | High |
| **P1** | Error recovery macros | Low | High |
| **P2** | <<EOF>> rules | Low | Medium |
| **P2** | Trailing context (r/s) | High | Medium |
| **P2** | Named references | Low | Medium |
| **P2** | Mid-rule actions | Medium | Medium |
| **P2** | %code blocks | Low | Medium |
| **P2** | Header file generation | Low | Medium |
| **P3** | %option directives | Low | Low |
| **P3** | Table compression | High | Low |
| **P3** | Debug/trace mode | Low | Low |
| **P3** | Counterexamples | High | Low |
| **P3** | yymore/yyless/REJECT | Medium | Low |

---

## Compatibility Matrix vs Flex/Bison

### Lexer Features

| Feature | Flex | OpenLexer | Notes |
|---------|------|-----------|-------|
| Basic regex | ✅ | ✅ | |
| Character classes | ✅ | ✅ | |
| Start conditions | ✅ | ✅ | |
| Named definitions | ✅ | ✅ | |
| Unicode | ⚠️ | ✅ | OpenLexer is better |
| REJECT | ✅ | ⚠️ | Detected only |
| yymore/yyless | ✅ | ⚠️ | Detected only |
| <<EOF>> | ✅ | ❌ | |
| Trailing context | ✅ | ❌ | |
| Reentrant | ✅ | ❌ | |
| Buffer switching | ✅ | ❌ | |
| C++ classes | ✅ | ❌ | |
| %option | ✅ | ❌ | |

### Parser Features

| Feature | Bison | OpenLexer | Notes |
|---------|-------|-----------|-------|
| LALR(1) | ✅ | ❌ | Uses SLR(1) |
| GLR | ✅ | ✅ | |
| Precedence | ✅ | ✅ | |
| %union | ✅ | ✅ | Basic |
| %locations | ✅ | ⚠️ | Basic |
| %destructor | ✅ | ⚠️ | Basic |
| Push parser | ✅ | ❌ | |
| Reentrant | ✅ | ❌ | |
| %define | ✅ | ❌ | |
| %code blocks | ✅ | ❌ | |
| Named refs | ✅ | ❌ | |
| Mid-rule actions | ✅ | ❌ | |
| Error macros | ✅ | ❌ | |
| Debug/trace | ✅ | ❌ | |
| Counterexamples | ✅ | ❌ | |
| %expect | ✅ | ❌ | |
| %skeleton | ✅ | ❌ | |
| C++/Java class | ✅ | ⚠️ | Basic Java |

---

## Summary

**OpenLexer Coverage**:
- **Lexer**: ~60% of Flex features
- **Parser**: ~55% of Bison features

**Blockers for Production Use**:
1. SLR(1) instead of LALR(1)
2. No reentrant/thread-safe mode
3. No input buffer management
4. Limited error recovery

**Advantages over Flex/Bison**:
1. Multi-language output (C, Java, Python)
2. Better Unicode support
3. Single tool (not two)
4. Simpler codebase
5. WebAssembly support

---

## References

- [GNU Bison Manual](https://www.gnu.org/software/bison/manual/bison.html)
- [Flex Manual](https://westes.github.io/flex/manual/)
- DeRemer & Pennello (1982) "Efficient Computation of LALR(1) Look-Ahead Sets"
- Tomita, M. (1985) "Efficient Parsing for Natural Language" - GLR
- Aho, Sethi, Ullman (1986) "Compilers: Principles, Techniques, and Tools"
