%{
/* Exercise 4: Simple Calculator Lexer
 * Tokens: NUMBER (multi-digit), LETTER (variables a-z), operators
 */
%}

%%

[ \t]+          { /* skip whitespace */ }
[0-9]+          { return NUMBER; }
[a-z]           { return LETTER; }
\n              { return NEWLINE; }
"+"             { return PLUS; }
"-"             { return MINUS; }
"*"             { return TIMES; }
"/"             { return DIVIDE; }
"%"             { return MOD; }
"^"             { return POWER; }
"&"             { return BITAND; }
"|"             { return BITOR; }
"("             { return LPAREN; }
")"             { return RPAREN; }
"="             { return ASSIGN; }
.               { return ERROR; }

%%
