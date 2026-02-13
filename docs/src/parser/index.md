# Parser Generator

The parser generator converts context-free grammars into LALR(1) parsing tables and generates parser code.

## How It Works

1. **Parsing**: The `.y` file is parsed to extract tokens, precedence, and grammar rules.
2. **Grammar Analysis**: First and Follow sets are computed.
3. **LR(0) Items**: Canonical LR(0) item sets are constructed.
4. **LALR(1) Tables**: Lookahead information is computed and tables are built.
5. **Conflict Resolution**: Shift/reduce and reduce/reduce conflicts are resolved using precedence.
6. **Code Generation**: The parsing table and driver are emitted in the target language.

## Input Format

Grammar specifications use the Bison `.y` file format:

```
%{
/* Prologue code */
%}

/* Token and precedence declarations */
%token NUMBER PLUS MINUS
%left PLUS MINUS

%%

/* Grammar rules */
expr: expr PLUS expr
    | expr MINUS expr
    | NUMBER
    ;

%%

/* Epilogue code */
```

## Output

The generated parser provides:

- A `Parser` class (or struct in C)
- A `parse()` function that returns the parse result
- Action and goto tables
- Semantic action execution

## GLR Mode

For ambiguous grammars, add `%glr-parser` to enable GLR parsing:

```yacc
%glr-parser

%%

expr: expr MINUS expr
    | NUMBER
    ;

%%
```

## Chapters

- [Grammar File Format](./file-format.md) - Complete `.y` file syntax
- [Tokens and Precedence](./tokens.md) - Declaring tokens and resolving conflicts
- [Semantic Actions](./actions.md) - Attaching code to rules
- [Error Handling](./errors.md) - Parser error recovery
