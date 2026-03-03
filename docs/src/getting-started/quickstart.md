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

### Option A: Python

```bash
# Generate Python lexer and parser
openlexer gen-lexer --lexer calc.l -L python -o ./
openlexer gen-parser --parser calc.y -L python -o ./
```

This produces `lexer.py` and `parser.py`.

**Use the generated code:**

```bash
# Test the parser directly
python parser.py "3 + 4 * 2"
# Output: Result: 11
```

Or create `main.py`:

```python
from parser import parse

result = parse("3 + 4 * 2")
print(f"Result: {result}")
```

### Option B: Java

```bash
# Generate Java lexer and parser
openlexer gen-lexer --lexer calc.l -L java -o ./
openlexer gen-parser --parser calc.y -L java -o ./
```

This produces `Lexer.java` and `Parser.java`.

**Compile and run:**

```bash
# Compile both files
javac Lexer.java Parser.java

# Run parser (auto-detects Lexer.class)
java Parser "3 + 4 * 2"
# Output: [Using external Lexer.class]
#         Input: "3 + 4 * 2"
#         Result: 11
```

### Option C: C

```bash
# Generate C lexer and parser
openlexer gen-lexer --lexer calc.l -L c -o ./
openlexer gen-parser --parser calc.y -L c -o ./
```

This produces `lexer.c` and `parser.c`.

**Compile and run:**

```bash
# Compile and link
gcc -o calc parser.c -lm

# Run
./calc "3 + 4 * 2"
# Output: Result: 11
```

## Next Steps

- [Lexer File Format](../lexer/file-format.md) - Full lexer specification syntax
- [Grammar File Format](../parser/file-format.md) - Full grammar specification syntax
- [Calculator Example](../examples/calculator.md) - Complete calculator implementation
