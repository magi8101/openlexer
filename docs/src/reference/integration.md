# Combining Lexer and Parser

This guide explains how to generate and use a complete lexer+parser system with OpenLexer.

## Overview

A typical language implementation has two phases:

1. **Lexical Analysis (Lexer)**: Convert character stream → token stream
2. **Syntactic Analysis (Parser)**: Convert token stream → parse tree/AST

```
Source Code → [Lexer] → Tokens → [Parser] → Parse Tree → [Your Code]
```

## Method 1: GUI Combined Tab

The easiest way to generate both together:

1. Open the GUI: `cargo run --bin openlexer-gui --features gui`
2. Click the **Combined** tab
3. Enter your lexer spec in the left panel
4. Enter your grammar in the middle panel
5. Click **Generate Combined**
6. Download or copy the output

The output contains both lexer and parser in one file with section markers.

## Method 2: Command Line

```bash
# Generate both at once
openlexer --lexer calc.l --parser calc.y --lang python -o output/

# This creates:
#   output/lexer.py   - The lexer
#   output/parser.py  - The parser (imports lexer)
```

## Method 3: Separate Generation

```bash
# Generate lexer
openlexer --lexer calc.l --lang python -o lexer.py

# Generate parser
openlexer --parser calc.y --lang python -o parser.py
```

## Token Coordination

**Critical**: The tokens in your `.y` file must match those returned by your `.l` file.

### Lexer (.l file)

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

### Parser (.y file)

```yacc
%token NUMBER PLUS MINUS TIMES DIVIDE LPAREN RPAREN

%%
expr: expr PLUS term
    | expr MINUS term
    | term
    ;

term: term TIMES factor
    | term DIVIDE factor
    | factor
    ;

factor: NUMBER
      | LPAREN expr RPAREN
      ;
%%
```

**The token names must match exactly!**

## Integration Code Examples

### Python Integration

```python
# calc.py - Using generated lexer and parser together

from lexer import Lexer, TokenType
from parser import Parser

def evaluate(expression):
    """Evaluate a mathematical expression."""
    # Phase 1: Tokenize
    lexer = Lexer(expression)
    tokens = list(lexer.tokenize())
    
    # Phase 2: Parse
    parser = Parser(tokens)
    ast = parser.parse()
    
    # Phase 3: Evaluate (your code)
    return evaluate_ast(ast)

# Example usage
result = evaluate("3 + 4 * 2")
print(f"Result: {result}")  # Output: 11
```

### C Integration

```c
// calc.c - Using generated lexer and parser together

#include "lexer.h"
#include "parser.h"

int evaluate(const char* expression) {
    // Phase 1: Initialize lexer
    Lexer lexer;
    lexer_init(&lexer, expression);
    
    // Phase 2: Initialize parser with lexer
    Parser parser;
    parser_init(&parser, &lexer);
    
    // Phase 3: Parse and evaluate
    int result = parser_parse(&parser);
    
    return result;
}

int main() {
    printf("3 + 4 * 2 = %d\n", evaluate("3 + 4 * 2"));
    return 0;
}
```

### Java Integration

```java
// Calculator.java - Using generated lexer and parser together

public class Calculator {
    public static int evaluate(String expression) {
        // Phase 1: Tokenize
        Lexer lexer = new Lexer(expression);
        List<Token> tokens = lexer.tokenize();
        
        // Phase 2: Parse
        Parser parser = new Parser(tokens);
        ParseTree tree = parser.parse();
        
        // Phase 3: Evaluate
        return evaluate(tree);
    }
    
    public static void main(String[] args) {
        System.out.println("3 + 4 * 2 = " + evaluate("3 + 4 * 2"));
    }
}
```

## Parser Calling Lexer Directly

In some generated code, the parser calls the lexer automatically:

```python
# Parser internally calls lexer.next_token() as needed
parser = Parser(input_string)  # Lexer created internally
result = parser.parse()
```

Check your generated code's constructor signature to see which style is used.

## Semantic Actions

Connect lexer output to parser semantic values:

### Passing Token Values

In the lexer, the matched text is available via `yytext`:

```lex
[0-9]+   { yylval = atoi(yytext); return NUMBER; }
```

In the parser, access values with `$1`, `$2`, etc.:

```yacc
expr: expr PLUS term  { $$ = $1 + $3; }
    | NUMBER          { $$ = $1; }
    ;
```

## Complete Example: Calculator

### calc.l (Lexer)

```lex
%{
/* Calculator lexer */
%}

%%
[0-9]+      { return NUMBER; }
"+"         { return PLUS; }
"-"         { return MINUS; }
"*"         { return TIMES; }
"/"         { return DIVIDE; }
"("         { return LPAREN; }
")"         { return RPAREN; }
[ \t\n]+    { /* skip */ }
.           { return ERROR; }
%%
```

### calc.y (Parser)

```yacc
%{
/* Calculator parser */
%}

%token NUMBER PLUS MINUS TIMES DIVIDE LPAREN RPAREN ERROR

%left PLUS MINUS
%left TIMES DIVIDE

%%
input: expr { printf("Result: %d\n", $1); }
     ;

expr: expr PLUS expr   { $$ = $1 + $3; }
    | expr MINUS expr  { $$ = $1 - $3; }
    | expr TIMES expr  { $$ = $1 * $3; }
    | expr DIVIDE expr { $$ = $1 / $3; }
    | LPAREN expr RPAREN { $$ = $2; }
    | NUMBER           { $$ = $1; }
    ;
%%
```

### Build and Run

```bash
# Generate both
openlexer --lexer calc.l --parser calc.y --lang c -o calc/

# Compile
cd calc
gcc -o calculator lexer.c parser.c main.c

# Run
./calculator "3 + 4 * 2"
# Output: Result: 11
```

## Troubleshooting

### "Unknown token" errors

**Cause**: Parser received a token not in `%token` declaration

**Fix**: Ensure all tokens returned by lexer are declared in parser

### "Syntax error" on valid input

**Cause**: Usually whitespace/newlines not being skipped

**Fix**: Add whitespace rule to lexer:
```lex
[ \t\n]+    { /* skip whitespace */ }
```

### Parser stuck or infinite loop

**Cause**: Lexer not returning EOF token

**Fix**: Ensure lexer returns EOF/END_OF_FILE when input exhausted

### Token value is wrong

**Cause**: Semantic value (`yylval`) not being set

**Fix**: In lexer action, set the value:
```lex
[0-9]+   { yylval.intval = atoi(yytext); return NUMBER; }
```

## Best Practices

1. **Define tokens in one place** - Use `%token` in parser, reference in lexer
2. **Test lexer first** - Use test driver to verify tokens before parser work
3. **Handle all input** - Include catch-all rule for unexpected characters
4. **Skip whitespace explicitly** - Don't rely on implicit handling
5. **Use precedence** - Resolve ambiguity with `%left`, `%right`, `%nonassoc`
6. **Start simple** - Get basic grammar working, then add complexity
