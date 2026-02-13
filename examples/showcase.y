/* ============================================================================
 * OpenLexer Feature Showcase Grammar
 * 
 * This grammar demonstrates ALL features of the OpenLexer parser generator:
 * - %union for typed semantic values
 * - %token <type> for typed tokens
 * - %type <type> for typed nonterminals
 * - %left, %right, %nonassoc for operator precedence/associativity
 * - %prec for contextual precedence override
 * - %glr-parser for GLR (Generalized LR) parsing
 * - %locations for source location tracking
 * - %destructor for memory cleanup
 * - %define parse.error detailed for better error messages
 * - %define parse.lac full for lookahead correction
 * - Semantic actions with $$, $1, $2, etc.
 * - Mid-rule actions
 * - Error recovery with 'error' token
 *
 * Language: A small expression language with:
 * - Integer and float literals
 * - String literals
 * - Variables
 * - Arithmetic operators (+, -, *, /, %, ^)
 * - Comparison operators (<, >, <=, >=, ==, !=)
 * - Logical operators (&&, ||, !)
 * - Ternary operator (? :)
 * - Function calls
 * - Array access
 * - Assignment
 * ============================================================================ */

%{
/* Prologue: C declarations */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

/* Symbol table for variables */
typedef struct {
    char *name;
    double value;
} Variable;

#define MAX_VARS 100
Variable variables[MAX_VARS];
int var_count = 0;

/* Function prototypes */
double get_var(const char *name);
void set_var(const char *name, double value);
double call_func(const char *name, double arg);
void yyerror(const char *s);
int yylex(void);

/* Runtime support */
double *array_new(int size);
double array_get(double *arr, int index);
void array_set(double *arr, int index, double value);
%}

/* ============================================================================
 * Parser Directives
 * ============================================================================ */

/* Enable GLR parsing for handling ambiguous grammars */
%glr-parser

/* Enable source location tracking (line/column numbers) */
%locations

/* Enable detailed error messages */
%define parse.error detailed

/* Enable LAC (Lookahead Correction) for better error recovery */
%define parse.lac full

/* ============================================================================
 * Semantic Value Union
 * ============================================================================ */

%union {
    double    dval;       /* Floating-point values */
    int       ival;       /* Integer values */
    char     *sval;       /* String values (dynamically allocated) */
    double   *aval;       /* Array pointer */
    struct {              /* For range expressions */
        double start;
        double end;
        double step;
    } range;
}

/* ============================================================================
 * Token Declarations with Types
 * ============================================================================ */

/* Literals */
%token <dval> NUMBER       "number"
%token <ival> INTEGER      "integer"
%token <sval> STRING       "string literal"
%token <sval> IDENTIFIER   "identifier"

/* Keywords */
%token IF       "if"
%token ELSE     "else"
%token WHILE    "while"
%token FOR      "for"
%token IN       "in"
%token RETURN   "return"
%token PRINT    "print"
%token TRUE     "true"
%token FALSE    "false"
%token NULL_TOK "null"

/* Operators (sorted by precedence, lowest first) */
%token QUESTION "?"
%token COLON    ":"
%token OR       "||"
%token AND      "&&"
%token EQ       "=="
%token NE       "!="
%token LT       "<"
%token GT       ">"
%token LE       "<="
%token GE       ">="
%token PLUS     "+"
%token MINUS    "-"
%token TIMES    "*"
%token DIVIDE   "/"
%token MOD      "%"
%token POWER    "^"
%token NOT      "!"
%token UMINUS   "unary minus"

/* Delimiters */
%token LPAREN   "("
%token RPAREN   ")"
%token LBRACKET "["
%token RBRACKET "]"
%token LBRACE   "{"
%token RBRACE   "}"
%token SEMICOLON ";"
%token COMMA    ","
%token ASSIGN   "="
%token DOTDOT   ".."
%token NEWLINE  "newline"

/* ============================================================================
 * Nonterminal Type Declarations
 * ============================================================================ */

%type <dval>  expr term factor primary literal
%type <dval>  comparison logical ternary
%type <ival>  bool_literal
%type <sval>  identifier
%type <aval>  array_literal array_elements
%type <range> range_expr

/* ============================================================================
 * Operator Precedence and Associativity
 * (listed from lowest to highest precedence)
 * ============================================================================ */

/* Ternary operator - lowest precedence, right associative */
%right QUESTION COLON

/* Logical OR */
%left OR

/* Logical AND */
%left AND

/* Equality operators */
%left EQ NE

/* Relational operators */
%left LT GT LE GE

/* Additive operators */
%left PLUS MINUS

/* Multiplicative operators */
%left TIMES DIVIDE MOD

/* Power operator - right associative */
%right POWER

/* Unary operators - highest precedence */
%right NOT UMINUS

/* ============================================================================
 * Destructor Declarations (for memory management)
 * ============================================================================ */

/* Free string values when popped from stack without being used */
%destructor { free($$); printf("Freed string: %s\n", $$); } <sval>

/* Free arrays */
%destructor { free($$); printf("Freed array\n"); } <aval>

/* ============================================================================
 * Grammar Rules
 * ============================================================================ */

%%

/* Entry point */
program:
    /* empty */
    | program statement
    ;

statement:
    expr NEWLINE                    { printf("= %g\n", $1); }
    | expr SEMICOLON                { printf("= %g\n", $1); }
    | IDENTIFIER ASSIGN expr NEWLINE { set_var($1, $3); free($1); }
    | IDENTIFIER ASSIGN expr SEMICOLON { set_var($1, $3); free($1); }
    | PRINT LPAREN expr RPAREN SEMICOLON { printf("%g\n", $3); }
    | NEWLINE                       { /* empty line */ }
    | error NEWLINE                 { yyerrok; printf("Syntax error - continuing...\n"); }
    ;

/* Top-level expression: ternary conditional */
expr:
    ternary                         { $$ = $1; }
    ;

/* Ternary conditional: a ? b : c */
ternary:
    logical                         { $$ = $1; }
    | logical QUESTION expr COLON ternary { $$ = $1 ? $3 : $5; }
    ;

/* Logical operators */
logical:
    comparison                      { $$ = $1; }
    | logical OR logical            { $$ = ($1 != 0.0) || ($3 != 0.0); }
    | logical AND logical           { $$ = ($1 != 0.0) && ($3 != 0.0); }
    | NOT logical                   { $$ = ($2 == 0.0) ? 1.0 : 0.0; }
    ;

/* Comparison operators */
comparison:
    term                            { $$ = $1; }
    | comparison LT comparison      { $$ = ($1 < $3) ? 1.0 : 0.0; }
    | comparison GT comparison      { $$ = ($1 > $3) ? 1.0 : 0.0; }
    | comparison LE comparison      { $$ = ($1 <= $3) ? 1.0 : 0.0; }
    | comparison GE comparison      { $$ = ($1 >= $3) ? 1.0 : 0.0; }
    | comparison EQ comparison      { $$ = ($1 == $3) ? 1.0 : 0.0; }
    | comparison NE comparison      { $$ = ($1 != $3) ? 1.0 : 0.0; }
    ;

/* Additive operators */
term:
    factor                          { $$ = $1; }
    | term PLUS factor              { $$ = $1 + $3; }
    | term MINUS factor             { $$ = $1 - $3; }
    ;

/* Multiplicative operators */
factor:
    primary                         { $$ = $1; }
    | factor TIMES primary          { $$ = $1 * $3; }
    | factor DIVIDE primary         { 
        if ($3 == 0.0) {
            yyerror("Division by zero");
            $$ = 0.0;
        } else {
            $$ = $1 / $3;
        }
    }
    | factor MOD primary            { $$ = fmod($1, $3); }
    | factor POWER primary          { $$ = pow($1, $3); }
    ;

/* Primary expressions */
primary:
    literal                         { $$ = $1; }
    | IDENTIFIER                    { $$ = get_var($1); free($1); }
    | LPAREN expr RPAREN            { $$ = $2; }
    | MINUS primary %prec UMINUS    { $$ = -$2; }
    | function_call                 { $$ = $<dval>1; }
    | array_access                  { $$ = $<dval>1; }
    ;

/* Literals */
literal:
    NUMBER                          { $$ = $1; }
    | INTEGER                       { $$ = (double)$1; }
    | bool_literal                  { $$ = (double)$1; }
    ;

bool_literal:
    TRUE                            { $$ = 1; }
    | FALSE                         { $$ = 0; }
    ;

/* Function calls */
function_call:
    IDENTIFIER LPAREN RPAREN        { 
        $<dval>$ = call_func($1, 0.0); 
        free($1); 
    }
    | IDENTIFIER LPAREN expr RPAREN { 
        $<dval>$ = call_func($1, $3); 
        free($1); 
    }
    | IDENTIFIER LPAREN argument_list RPAREN {
        /* Multi-argument function - for demo, use first arg */
        $<dval>$ = call_func($1, $<dval>3);
        free($1);
    }
    ;

argument_list:
    expr                            { $<dval>$ = $1; }
    | argument_list COMMA expr      { $<dval>$ = $<dval>1; /* Keep first */ }
    ;

/* Array access */
array_access:
    IDENTIFIER LBRACKET expr RBRACKET {
        /* Array indexing - simplified demo */
        $<dval>$ = get_var($1) + $3;
        free($1);
    }
    ;

/* Array literals [1, 2, 3] */
array_literal:
    LBRACKET array_elements RBRACKET { $$ = $2; }
    | LBRACKET RBRACKET             { $$ = array_new(0); }
    ;

array_elements:
    expr                            { 
        $$ = array_new(1);
        array_set($$, 0, $1);
    }
    | array_elements COMMA expr     {
        $$ = $1;
        /* In real impl, would resize and append */
    }
    ;

/* Range expression for loops: 1..10 or 1..10..2 (start..end..step) */
range_expr:
    expr DOTDOT expr                { 
        $$.start = $1; 
        $$.end = $3; 
        $$.step = 1.0; 
    }
    | expr DOTDOT expr DOTDOT expr  { 
        $$.start = $1; 
        $$.end = $3; 
        $$.step = $5; 
    }
    ;

/* Optional: identifier helper */
identifier:
    IDENTIFIER                      { $$ = $1; }
    ;

%%

/* ============================================================================
 * Epilogue: C code
 * ============================================================================ */

/* Variable operations */
double get_var(const char *name) {
    for (int i = 0; i < var_count; i++) {
        if (strcmp(variables[i].name, name) == 0) {
            return variables[i].value;
        }
    }
    printf("Warning: undefined variable '%s', using 0\n", name);
    return 0.0;
}

void set_var(const char *name, double value) {
    for (int i = 0; i < var_count; i++) {
        if (strcmp(variables[i].name, name) == 0) {
            variables[i].value = value;
            return;
        }
    }
    if (var_count < MAX_VARS) {
        variables[var_count].name = strdup(name);
        variables[var_count].value = value;
        var_count++;
    } else {
        printf("Error: too many variables\n");
    }
}

/* Built-in functions */
double call_func(const char *name, double arg) {
    if (strcmp(name, "sin") == 0) return sin(arg);
    if (strcmp(name, "cos") == 0) return cos(arg);
    if (strcmp(name, "tan") == 0) return tan(arg);
    if (strcmp(name, "sqrt") == 0) return sqrt(arg);
    if (strcmp(name, "abs") == 0) return fabs(arg);
    if (strcmp(name, "floor") == 0) return floor(arg);
    if (strcmp(name, "ceil") == 0) return ceil(arg);
    if (strcmp(name, "round") == 0) return round(arg);
    if (strcmp(name, "log") == 0) return log(arg);
    if (strcmp(name, "log10") == 0) return log10(arg);
    if (strcmp(name, "exp") == 0) return exp(arg);
    if (strcmp(name, "pi") == 0) return 3.14159265358979323846;
    if (strcmp(name, "e") == 0) return 2.71828182845904523536;
    printf("Warning: unknown function '%s'\n", name);
    return 0.0;
}

/* Array operations */
double *array_new(int size) {
    return (double *)calloc(size + 1, sizeof(double));
}

double array_get(double *arr, int index) {
    return arr[index];
}

void array_set(double *arr, int index, double value) {
    arr[index] = value;
}

void yyerror(const char *s) {
    fprintf(stderr, "Error at line %d, column %d: %s\n", 
            yylloc.first_line, yylloc.first_column, s);
}

int main(void) {
    printf("OpenLexer Expression Calculator\n");
    printf("Features: GLR, locations, typed values, error recovery\n");
    printf("Enter expressions (Ctrl+D to exit):\n\n");
    return yyparse();
}
