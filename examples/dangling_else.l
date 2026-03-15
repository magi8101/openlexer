/* Dangling Else - Lexer */

%%

"if"            { return IF; }
"else"          { return ELSE; }
"other"         { return OTHER; }
"condition"     { return CONDITION; }
"("             { return LPAREN; }
")"             { return RPAREN; }
[ \t\n]+        { /* skip whitespace */ }
.               { /* skip unknown */ }

%%
