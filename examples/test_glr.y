/* GLR Parser Test Grammar
 * 
 * Tests ambiguous grammars that require GLR parsing.
 * Contains intentional shift/reduce conflicts.
 */

%{
#include <stdio.h>
#include <stdlib.h>

/* Results storage for testing */
static int parse_count = 0;
%}

/* Enable GLR parsing for handling ambiguous grammars */
%glr-parser

%union {
    int ival;
    double dval;
}

%token <ival> NUM
%token MINUS PLUS TIMES

%type <dval> expr

/* No precedence declarations - intentionally ambiguous */

%%

input
    : expr          { printf("Result: %g\n", $1); }
    ;

/* Ambiguous expression grammar - no associativity specified */
expr
    : expr MINUS expr   { $$ = $1 - $3; printf("  (%g - %g) = %g\n", $1, $3, $$); }
    | expr PLUS expr    { $$ = $1 + $3; printf("  (%g + %g) = %g\n", $1, $3, $$); }
    | expr TIMES expr   { $$ = $1 * $3; printf("  (%g * %g) = %g\n", $1, $3, $$); }
    | NUM               { $$ = (double)$1; }
    ;

%%

void yyerror(const char *s) {
    fprintf(stderr, "Parse error: %s\n", s);
}

int main(void) {
    printf("GLR Parser Test\n");
    printf("===============\n");
    printf("This grammar is intentionally ambiguous.\n");
    printf("Expressions like '1 - 2 - 3' have multiple valid parses.\n\n");
    
    if (yyparse() == 0) {
        printf("\nParse successful!\n");
    } else {
        printf("\nParse failed!\n");
    }
    return 0;
}
