# OpenLexer

A modern Flex/Bison replacement that generates lexers and parsers in **C**, **Java**, and **Python**.
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **Lexer Generator** - Convert regex patterns to efficient DFA-based lexers
- **Parser Generator** - Generate LALR(1) parsers from BNF grammars  
- **Multi-language Output** - Generate code in C, Java, or Python
- **Flex/Bison Compatible** - Uses familiar `.l` and `.y` file formats
- **Web GUI** - Browser-based interface via WebAssembly

## Try Online

Visit the [web demo](https://magi8101.github.io/openlexer/) to try OpenLexer in your browser.

## Installation

### Build from Source

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/example/openlexer.git
cd openlexer
cargo build --release
```

## Usage

### Generate Lexer

```bash
openlexer gen-lexer --lexer rules.l --lang python --output output/
```

### Generate Parser

```bash
openlexer gen-parser --parser grammar.y --lang java --output output/
```

### Supported Languages

| Language | Lexer | Parser |
|----------|-------|--------|
| Python   | Yes   | Yes    |
| C        | Yes   | Yes    |
| Java     | Yes   | Yes    |

## Example

### Lexer Specification (calc.l)

```lex
%%
[0-9]+      { return NUMBER; }
"+"         { return PLUS; }
"-"         { return MINUS; }
"*"         { return TIMES; }
"/"         { return DIVIDE; }
"("         { return LPAREN; }
")"         { return RPAREN; }
[ \t\n]+    { /* skip whitespace */ }
%%
```

### Grammar Specification (calc.y)

```yacc
%token NUMBER PLUS MINUS TIMES DIVIDE LPAREN RPAREN

%%

expr:
    expr PLUS term   { $$ = $1 + $3; }
  | expr MINUS term  { $$ = $1 - $3; }
  | term             { $$ = $1; }
  ;

term:
    term TIMES factor { $$ = $1 * $3; }
  | term DIVIDE factor { $$ = $1 / $3; }
  | factor            { $$ = $1; }
  ;

factor:
    LPAREN expr RPAREN { $$ = $2; }
  | NUMBER             { $$ = $1; }
  ;

%%
```

### Generate Code

```bash
# Generate Python lexer
openlexer gen-lexer --lexer calc.l --lang python --output calc_py/

# Generate Python parser
openlexer gen-parser --parser calc.y --lang python --output calc_py/
```

## Architecture

```
.l file (Flex format)
    |
    v
+-------------------+
| Regex Parser      |
+-------------------+
    |
    v
+-------------------+
| NFA Construction  | (Thompson's algorithm)
+-------------------+
    |
    v
+-------------------+
| DFA Construction  | (Subset construction)
+-------------------+
    |
    v
+-------------------+
| Code Generation   | --> lexer.py / lexer.c / Lexer.java
+-------------------+


.y file (Bison format)
    |
    v
+-------------------+
| Grammar Parser    |
+-------------------+
    |
    v
+-------------------+
| FIRST/FOLLOW Sets |
+-------------------+
    |
    v
+-------------------+
| LALR(1) Tables    |
+-------------------+
    |
    v
+-------------------+
| Code Generation   | --> parser.py / parser.c / Parser.java
+-------------------+
```

## GUI

Run the desktop GUI:

```bash
cargo run --features gui --bin openlexer-gui
```

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please open an issue or PR.
