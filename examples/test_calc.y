%{
/* Minimal Calculator Grammar for Combined Lexer+Parser Testing
 * Self-contained semantic actions - no external dependencies
 * Tests: arithmetic expressions, precedence, associativity
 */
#include <stdio.h>
#include <stdlib.h>
#include <math.h>

static double result = 0.0;
%}

%union {
    double dval;
}

%token <dval> NUMBER
%token PLUS MINUS TIMES DIVIDE MOD POWER
%token LPAREN RPAREN
%token NEWLINE

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
    | expr NEWLINE      { result = $1; printf("= %g\n", result); }
    | error NEWLINE     { yyerrok; }
    ;

expr:
    term                { $$ = $1; }
    | expr PLUS term    { $$ = $1 + $3; }
    | expr MINUS term   { $$ = $1 - $3; }
    ;

term:
    factor              { $$ = $1; }
    | term TIMES factor { $$ = $1 * $3; }
    | term DIVIDE factor { 
        if ($3 == 0.0) {
            yyerror("division by zero");
            $$ = 0.0;
        } else {
            $$ = $1 / $3;
        }
    }
    | term MOD factor   { $$ = fmod($1, $3); }
    ;

factor:
    primary             { $$ = $1; }
    | factor POWER primary { $$ = pow($1, $3); }
    | MINUS factor %prec UMINUS { $$ = -$2; }
    ;

primary:
    NUMBER              { $$ = $1; }
    | LPAREN expr RPAREN { $$ = $2; }
    ;

%%
