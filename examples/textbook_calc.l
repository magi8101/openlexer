%{
/* OpenLexer version of textbook calculator lexer */
%}
%%
"(" { return LPAREN; }
")" { return RPAREN; }
"+" { return PLUS; }
"*" { return STAR; }
[0-9]+ { return NUMBER; }
[ \t\n]+ { /* skip whitespace */ }
. { return ERROR; }
%%
