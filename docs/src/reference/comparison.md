# Comparison with Flex/Bison

This chapter compares OpenLexer with GNU Flex and Bison.

## Feature Comparison

| Feature | Flex | Bison | OpenLexer |
|---------|------|-------|-----------|
| Lexer generation | Yes | No | Yes |
| Parser generation | No | Yes | Yes |
| C output | Yes | Yes | Yes |
| Java output | JFlex | CUP/ANTLR | Yes |
| Python output | PLY | PLY | Yes |
| GLR parsing | No | Yes | Yes |
| LALR(1) | N/A | Yes | Yes |
| Start conditions | Yes | N/A | Yes |
| Precedence | N/A | Yes | Yes |
| Unicode | Limited | N/A | Planned |
| Web (WASM) | No | No | Yes |

## File Format Compatibility

### Lexer Files (.l)

OpenLexer supports most Flex syntax:

**Supported:**
- Definitions section with `%{ %}` code blocks
- Named patterns
- Rules with actions
- Start conditions (`%x`, `%s`)
- Character classes `[a-z]`
- Predefined classes `[:alpha:]`
- Quantifiers `*`, `+`, `?`, `{n,m}`
- Alternation `|`
- Grouping `()`
- Literal strings `"..."`

**Not supported:**
- `%option` directives
- `yymore()`, `yyless()`
- `REJECT`
- Multiple input buffers
- Interactive mode

### Grammar Files (.y)

OpenLexer supports most Bison syntax:

**Supported:**
- `%token` declarations
- `%union` type declarations
- `%type` non-terminal types
- `%left`, `%right`, `%nonassoc` precedence
- `%start` symbol
- `%prec` contextual precedence
- `%glr-parser` GLR mode
- Semantic actions with `$$`, `$1`, etc.
- `error` token for recovery

**Not supported:**
- `%define` options
- `%code` blocks (use `%{ %}` instead)
- `%destructor`
- `%printer`
- Lookahead correction (`%define lr.type ielr`)
- Named references (`$name` instead of `$1`)

## Migration Guide

### From Flex

1. Remove `%option` lines
2. Replace `yymore()` with manual buffer handling
3. Replace `REJECT` with explicit rules
4. Test start condition behavior

### From Bison

1. Replace `%define` with equivalent features
2. Move `%code` blocks to `%{ %}`
3. Test GLR behavior if used
4. Verify error recovery patterns

## Performance

OpenLexer generates table-driven lexers and parsers similar to Flex/Bison. Performance characteristics:

- Lexer: O(n) where n is input length
- LALR Parser: O(n) for unambiguous grammars
- GLR Parser: O(n) to O(n^3) depending on ambiguity

The generated code size is comparable to Flex/Bison output.

## When to Use OpenLexer

Use OpenLexer when:
- You need multi-language output
- You want a single cross-platform tool
- You need integrated lexer + parser generation
- You want GLR parsing

Use Flex/Bison when:
- You need maximum C performance optimization
- You require specific Flex/Bison features not in OpenLexer
- You have existing build infrastructure for Flex/Bison
