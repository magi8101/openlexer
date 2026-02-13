# Calculator Example

A complete calculator supporting arithmetic operations with proper precedence.

## Lexer Specification (calc.l)

```lex
%{
/* Calculator Lexer */
%}

%%

[0-9]+          { return NUMBER; }
[0-9]+\.[0-9]+  { return FLOAT; }
"+"             { return PLUS; }
"-"             { return MINUS; }
"*"             { return TIMES; }
"/"             { return DIVIDE; }
"%"             { return MOD; }
"^"             { return POWER; }
"("             { return LPAREN; }
")"             { return RPAREN; }
[ \t]+          { /* skip whitespace */ }
\n              { return NEWLINE; }
.               { return ERROR; }

%%
```

## Grammar Specification (calc.y)

```yacc
%{
/* Calculator Parser */
#include <stdio.h>
#include <math.h>
%}

%union {
    double dval;
}

%token <dval> NUMBER FLOAT
%token PLUS MINUS TIMES DIVIDE MOD POWER
%token LPAREN RPAREN NEWLINE ERROR

%type <dval> expr term factor primary

%left PLUS MINUS
%left TIMES DIVIDE MOD
%right POWER
%right UMINUS

%%

input:
    /* empty */
    | input line
    ;

line:
    NEWLINE
    | expr NEWLINE          { printf("= %g\n", $1); }
    | error NEWLINE         { yyerrok; }
    ;

expr:
    expr PLUS term          { $$ = $1 + $3; }
    | expr MINUS term       { $$ = $1 - $3; }
    | term                  { $$ = $1; }
    ;

term:
    term TIMES factor       { $$ = $1 * $3; }
    | term DIVIDE factor    { 
        if ($3 == 0) {
            yyerror("division by zero");
            $$ = 0;
        } else {
            $$ = $1 / $3;
        }
    }
    | term MOD factor       { $$ = fmod($1, $3); }
    | factor                { $$ = $1; }
    ;

factor:
    primary POWER factor    { $$ = pow($1, $3); }
    | primary               { $$ = $1; }
    ;

primary:
    NUMBER                  { $$ = $1; }
    | FLOAT                 { $$ = $1; }
    | LPAREN expr RPAREN    { $$ = $2; }
    | MINUS primary %prec UMINUS { $$ = -$2; }
    ;

%%

void yyerror(const char *s) {
    fprintf(stderr, "Error: %s\n", s);
}
```

## Generate Code

```bash
openlexer gen-lexer --lexer calc.l --lang c --output ./
openlexer gen-parser --parser calc.y --lang c --output ./
```

## Main Program (main.c)

```c
#include <stdio.h>
#include <string.h>

/* Include generated code */
#include "lexer.c"
#include "parser.c"

int main() {
    char line[1024];
    
    printf("Calculator (type expressions, Ctrl+C to exit)\n");
    
    while (1) {
        printf("> ");
        fflush(stdout);
        
        if (fgets(line, sizeof(line), stdin) == NULL) {
            break;
        }
        
        lex_init(line);
        yyparse();
    }
    
    return 0;
}
```

## Build and Run

```bash
gcc -o calc main.c -lm -Wall
./calc
```

## Example Session

```
Calculator (type expressions, Ctrl+C to exit)
> 3 + 4
= 7
> 2 * 3 + 4
= 10
> 2 + 3 * 4
= 14
> (2 + 3) * 4
= 20
> 2 ^ 3 ^ 2
= 512
> -5 + 3
= -2
> 10 / 0
Error: division by zero
= 0
```

## Notes

- Power (`^`) is right-associative: `2^3^2 = 2^9 = 512`
- Unary minus has highest precedence
- Parentheses override precedence
- Division by zero is caught with error message
