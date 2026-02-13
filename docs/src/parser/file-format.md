# Grammar File Format (.y)

The grammar specification file has three sections separated by `%%`:

```
DECLARATIONS
%%
RULES
%%
USER CODE
```

## Declarations Section

### Prologue Code

Code in `%{` and `%}` is copied to the output:

```yacc
%{
#include <stdio.h>
#include <math.h>
extern int yylex();
void yyerror(const char *s);
%}
```

### Token Declarations

Declare terminal symbols with `%token`:

```yacc
%token NUMBER
%token PLUS MINUS TIMES DIVIDE
%token LPAREN RPAREN
```

Tokens with types:

```yacc
%token <ival> NUMBER
%token <str> IDENTIFIER
```

### Union Declaration

Define the semantic value type:

```yacc
%union {
    int ival;
    double dval;
    char *str;
}
```

### Type Declarations

Assign types to non-terminals:

```yacc
%type <dval> expr term factor
```

### Precedence Declarations

Declare operator precedence and associativity:

```yacc
%left PLUS MINUS
%left TIMES DIVIDE
%right POWER
%nonassoc UMINUS
```

- `%left`: Left-associative
- `%right`: Right-associative
- `%nonassoc`: Non-associative (error on chaining)

Later declarations have higher precedence.

### Start Symbol

Specify the grammar's start symbol:

```yacc
%start program
```

If omitted, the left-hand side of the first rule is used.

## Rules Section

Production rules define the grammar:

```yacc
%%

program:
    statement_list
    ;

statement_list:
    statement
    | statement_list statement
    ;

statement:
    expr SEMICOLON      { printf("Result: %d\n", $1); }
    | error SEMICOLON   { yyerrok; }
    ;

expr:
    expr PLUS expr      { $$ = $1 + $3; }
    | expr MINUS expr   { $$ = $1 - $3; }
    | NUMBER            { $$ = $1; }
    ;

%%
```

### Rule Syntax

```
nonterminal:
    production1     { action1 }
    | production2   { action2 }
    ;
```

### Empty Productions

Use `/* empty */` or nothing:

```yacc
optional_items:
    /* empty */
    | item_list
    ;
```

## User Code Section

Auxiliary functions copied to the output:

```yacc
%%

void yyerror(const char *s) {
    fprintf(stderr, "Error: %s\n", s);
}

int main() {
    return yyparse();
}
```
