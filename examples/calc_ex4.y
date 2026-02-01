%{
/* Exercise 4: Simple Calculator - Bison Grammar
 * Supports: +, -, *, /, ^, %, (), variable assignment, and unary minus
 * Based on GNU Bison infix calculator example
 */
%}

%token NUMBER
%token LETTER
%token NEWLINE
%token LPAREN RPAREN
%token ASSIGN

%left BITOR
%left BITAND
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
    | expr NEWLINE      { printf("%d\n", $1); }
    | LETTER ASSIGN expr NEWLINE { regs[$1] = $3; }
    | error NEWLINE     { yyerrok; }
    ;

expr:
    NUMBER              { $$ = $1; }
    | LETTER            { $$ = regs[$1]; }
    | expr PLUS expr    { $$ = $1 + $3; }
    | expr MINUS expr   { $$ = $1 - $3; }
    | expr TIMES expr   { $$ = $1 * $3; }
    | expr DIVIDE expr  { $$ = $1 / $3; }
    | expr MOD expr     { $$ = $1 % $3; }
    | expr POWER expr   { $$ = pow($1, $3); }
    | expr BITAND expr  { $$ = $1 & $3; }
    | expr BITOR expr   { $$ = $1 | $3; }
    | MINUS expr %prec UMINUS { $$ = -$2; }
    | LPAREN expr RPAREN { $$ = $2; }
    ;

%%
