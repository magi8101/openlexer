%{
#include "P1.tab.h"
#include<stdio.h>
#include<stdlib.h>
%}

%%

[ \t\n\r]+                          { /* Ignore whitespace */ }
"//"[^\n]*                          { /* Ignore single-line comments */ }
"/*"([^*]|"*"[^/])*"*/"             { /* Ignore multi-line comments */ }

"class"                             { printf("%s %d\n", yytext, CLASS); }
"public"                            { printf("%s %d\n", yytext, PUBLIC); }
"static"                            { printf("%s %d\n", yytext, STATIC); }
"void"                              { printf("%s %d\n", yytext, VOID); }
"main"                              { printf("%s %d\n", yytext, MAIN); }
"String"                            { printf("%s %d\n", yytext, STR); }
"extends"                           { printf("%s %d\n", yytext, EXTENDS); }
"return"                            { printf("%s %d\n", yytext, RET); }
"int"                               { printf("%s %d\n", yytext, INT); }
"boolean"                           { printf("%s %d\n", yytext, BOOL); }
"if"                                { printf("%s %d\n", yytext, IF); }
"else"                              { printf("%s %d\n", yytext, ELSE); }
"while"                             { printf("%s %d\n", yytext, WHILE); }
"true"                              { printf("%s %d\n", yytext, TRUE); }
"false"                             { printf("%s %d\n", yytext, FALSE); }
"this"                              { printf("%s %d\n", yytext, THIS); }
"new"                               { printf("%s %d\n", yytext, NEW); }
"length"                            { printf("%s %d\n", yytext, LENGTH); }
"System.out.println"                { printf("%s %d\n", yytext, PRINTLN); }

"+"                                 { printf("%s %d\n", yytext, ADD); }
"-"                                 { printf("%s %d\n", yytext, SUB); }
"*"                                 { printf("%s %d\n", yytext, MUL); }
"/"                                 { printf("%s %d\n", yytext, DIV); }
"%"                                 { printf("%s %d\n", yytext, MOD); }
"&"                                 { printf("%s %d\n", yytext, BIT_AND); }
"<"                                 { printf("%s %d\n", yytext, LESSTHAN); }
"!"                                 { printf("%s %d\n", yytext, NOT); }
"="                                 { printf("%s %d\n", yytext, ASSIGNMENT); }

"{"                                 { printf("%s %d\n", yytext, CURLY_OPEN); }
"}"                                 { printf("%s %d\n", yytext, CURLY_CLOSE); }
"("                                 { printf("%s %d\n", yytext, PAR_OPEN); }
")"                                 { printf("%s %d\n", yytext, PAR_CLOSE); }
"["                                 { printf("%s %d\n", yytext, SQR_OPEN); }
"]"                                 { printf("%s %d\n", yytext, SQR_CLOSE); }
";"                                 { printf("%s %d\n", yytext, SEMI_COLON); }
","                                 { printf("%s %d\n", yytext, COMMA); }
"."                                 { printf("%s %d\n", yytext, DOT); }

[0-9]+                              { printf("%s %d\n", yytext, NUMBER); }
[a-zA-Z_][a-zA-Z0-9_]*              { printf("%s %d\n", yytext, IDENTIFIER); }

.                                   { printf("Error: Invalid token %s\n", yytext); }

%%

int main() {
    yylex();
}

int yywrap() {}
