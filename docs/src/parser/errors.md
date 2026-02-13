# Error Handling

## The error Token

The special token `error` represents a syntax error:

```yacc
stmt: expr SEMICOLON
    | error SEMICOLON   { yyerrok; }
    ;
```

When an error occurs, the parser pops states until it can shift `error`, then continues parsing.

## yyerror Function

The `yyerror` function is called on syntax errors:

### C

```c
void yyerror(const char *msg) {
    fprintf(stderr, "line %d: %s\n", yylineno, msg);
}
```

### Python

```python
def yyerror(self, msg):
    print(f"line {self.yylineno}: {msg}", file=sys.stderr)
```

### Java

```java
void yyerror(String msg) {
    System.err.println("line " + yylineno + ": " + msg);
}
```

## Error Recovery

### yyerrok

Clears the error state, allowing normal parsing to resume:

```yacc
stmt: error SEMICOLON   { yyerrok; printf("Recovered at ;\n"); }
    ;
```

### yyclearin

Discards the current lookahead token:

```yacc
stmt: error SEMICOLON   { yyerrok; yyclearin; }
    ;
```

### YYERROR

Force an error from within an action:

```yacc
expr: expr DIVIDE expr {
        if ($3 == 0) {
            yyerror("division by zero");
            YYERROR;
        }
        $$ = $1 / $3;
    }
    ;
```

### YYABORT

Abort parsing immediately:

```yacc
stmt: QUIT { YYABORT; }
    ;
```

### YYACCEPT

Accept the input immediately:

```yacc
program: statement_list END { YYACCEPT; }
    ;
```

## Error Recovery Strategies

### Statement-Level Recovery

Recover at statement boundaries:

```yacc
program: stmt_list
    ;

stmt_list: stmt_list stmt
    | stmt
    ;

stmt: valid_statement SEMICOLON
    | error SEMICOLON { yyerrok; }
    ;
```

### Block-Level Recovery

Recover at block boundaries:

```yacc
block: LBRACE stmt_list RBRACE
    | LBRACE error RBRACE { yyerrok; }
    ;
```

### Parenthesis Recovery

```yacc
expr: LPAREN expr RPAREN
    | LPAREN error RPAREN { yyerrok; $$ = 0; }
    | NUMBER
    ;
```

## Error Messages

Provide context in error messages:

```c
void yyerror(const char *msg) {
    fprintf(stderr, "Error at line %d near '%s': %s\n",
            yylineno, yytext, msg);
}
```
