/* GLR Parser Test Lexer */

%%

[0-9]+      { return NUM; }
"-"         { return MINUS; }
"+"         { return PLUS; }
"*"         { return TIMES; }
[ \t\n]+    { /* skip whitespace */ }
.           { /* skip unknown */ }

%%
