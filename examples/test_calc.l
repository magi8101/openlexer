/* Minimal Calculator Lexer for Combined Lexer+Parser Testing */

%{
#include <stdio.h>
#include <stdlib.h>
%}

%%

[0-9]+(\.[0-9]+)?([eE][+-]?[0-9]+)?   { return NUMBER; }

"+"     { return PLUS; }
"-"     { return MINUS; }
"*"     { return TIMES; }
"/"     { return DIVIDE; }
"%"     { return MOD; }
"^"     { return POWER; }
"("     { return LPAREN; }
")"     { return RPAREN; }

\n      { return NEWLINE; }
[ \t]+  { /* skip whitespace */ }

.       { /* skip unknown */ }

%%
