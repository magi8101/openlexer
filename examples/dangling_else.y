/* Dangling Else Grammar
 * Classic ambiguous grammar demonstrating the shift/reduce conflict
 * when parsing nested if-else statements.
 *
 * S -> if (E) S
 * S -> if (E) S else S
 * S -> other
 * E -> condition
 */

%{
#include <stdio.h>
%}

%token IF ELSE OTHER CONDITION LPAREN RPAREN

%type <ival> S E

%%

program
    : S                         { printf("Parse complete.\n"); }
    ;

S
    : IF LPAREN E RPAREN S ELSE S   { printf("if-else statement\n"); }
    | IF LPAREN E RPAREN S          { printf("if statement\n"); }
    | OTHER                         { printf("other\n"); }
    ;

E
    : CONDITION                     { printf("condition\n"); }
    ;

%%
