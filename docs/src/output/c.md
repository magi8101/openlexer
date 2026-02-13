# C Output

## Generated Files

- `lexer.c` - Lexer implementation with DFA tables
- `parser.c` - Parser implementation with LALR tables

## Lexer Interface

```c
/* Initialize lexer with input */
void lex_init(const char *input);

/* Get next token, returns token type */
int yylex(void);

/* Current matched text */
extern char *yytext;

/* Length of matched text */
extern int yyleng;

/* Current line number (if enabled) */
extern int yylineno;
```

## Parser Interface

```c
/* Parse input, returns 0 on success */
int yyparse(void);

/* Error reporting callback */
void yyerror(const char *msg);

/* Semantic value passed from lexer */
extern YYSTYPE yylval;
```

## Integration Example

```c
#include <stdio.h>
#include "lexer.c"
#include "parser.c"

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "Usage: %s <input>\n", argv[0]);
        return 1;
    }
    
    lex_init(argv[1]);
    
    if (yyparse() == 0) {
        printf("Parse successful\n");
        return 0;
    } else {
        printf("Parse failed\n");
        return 1;
    }
}
```

## Compilation

```bash
gcc -o myparser main.c -Wall
```

Or compile separately:

```bash
gcc -c lexer.c -o lexer.o
gcc -c parser.c -o parser.o
gcc -c main.c -o main.o
gcc -o myparser lexer.o parser.o main.o
```

## Memory Management

The C lexer uses a static buffer for `yytext`. For dynamic allocation:

```c
char *token_copy = strdup(yytext);
/* ... use token_copy ... */
free(token_copy);
```

## YYSTYPE Union

When using `%union` in the grammar:

```c
typedef union YYSTYPE {
    int ival;
    double dval;
    char *str;
} YYSTYPE;

extern YYSTYPE yylval;
```

Set `yylval` in lexer actions:

```c
[0-9]+  { yylval.ival = atoi(yytext); return NUMBER; }
```

Access in parser actions:

```c
expr: expr PLUS expr  { $$ = $1 + $3; }
```

## Platform Compatibility

The generated C code is C99 compliant and works on:
- GCC (Linux, macOS, MinGW)
- Clang (Linux, macOS)
- MSVC (Windows)
