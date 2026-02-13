# Quick Start

This guide walks through creating a calculator lexer and parser.

## Step 1: Create the Lexer Specification

Create `calc.l`:

```lex
%{
/* Calculator lexer */
%}

%%

[0-9]+          { return NUMBER; }
"+"             { return PLUS; }
"-"             { return MINUS; }
"*"             { return TIMES; }
"/"             { return DIVIDE; }
"("             { return LPAREN; }
")"             { return RPAREN; }
[ \t\n]+        { /* skip whitespace */ }
.               { return ERROR; }

%%
```

## Step 2: Create the Grammar Specification

Create `calc.y`:

```yacc
%token NUMBER PLUS MINUS TIMES DIVIDE LPAREN RPAREN

%left PLUS MINUS
%left TIMES DIVIDE

%%

expr:
    expr PLUS expr    { $$ = $1 + $3; }
  | expr MINUS expr   { $$ = $1 - $3; }
  | expr TIMES expr   { $$ = $1 * $3; }
  | expr DIVIDE expr  { $$ = $1 / $3; }
  | LPAREN expr RPAREN { $$ = $2; }
  | NUMBER            { $$ = $1; }
  ;

%%
```

## Step 3: Generate Code

```bash
# Generate Python lexer and parser
openlexer gen-lexer --lexer calc.l --lang python --output ./
openlexer gen-parser --parser calc.y --lang python --output ./
```

This produces `lexer.py` and `parser.py`.

## Step 4: Use the Generated Code

Create `main.py`:

```python
from lexer import Lexer
from parser import Parser

lexer = Lexer("3 + 4 * 2")
parser = Parser(lexer)
result = parser.parse()
print(f"Result: {result}")
```

Run:

```bash
python main.py
```

Output:

```
Result: 11
```

## Next Steps

- [Lexer File Format](../lexer/file-format.md) - Full lexer specification syntax
- [Grammar File Format](../parser/file-format.md) - Full grammar specification syntax
- [Calculator Example](../examples/calculator.md) - Complete calculator implementation
