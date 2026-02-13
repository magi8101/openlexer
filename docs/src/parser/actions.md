# Semantic Actions

Semantic actions are code blocks attached to grammar rules that execute when the rule is reduced.

## Basic Syntax

```yacc
expr: expr PLUS expr    { $$ = $1 + $3; }
    | NUMBER            { $$ = $1; }
    ;
```

## Value Stack References

- `$$`: The result value (left-hand side)
- `$1`, `$2`, `$3`, ...: Values of right-hand side symbols

```yacc
/* Rule: factor -> LPAREN expr RPAREN */
factor: LPAREN expr RPAREN  { $$ = $2; }
       /* $1=LPAREN, $2=expr, $3=RPAREN */
    ;
```

## Typed Actions

With `%union`, values have specific types:

```yacc
%union {
    int ival;
    double dval;
    struct node *ast;
}

%type <dval> expr term factor

%%

expr: expr PLUS term    { $$ = $1 + $3; }
    ;
```

## Multi-Statement Actions

Braces contain the full action:

```yacc
stmt: PRINT expr SEMICOLON {
        printf("Value: %g\n", $2);
        $$ = $2;
    }
    ;
```

## Mid-Rule Actions

Actions can appear in the middle of a rule:

```yacc
compound: LBRACE { enter_scope(); } stmt_list RBRACE { leave_scope(); }
    ;
```

Mid-rule actions count as symbols:

```yacc
rule: A { mid_action(); } B C
    /* $1=A, $2=mid_action result, $3=B, $4=C */
    ;
```

## AST Construction

Building abstract syntax trees:

```yacc
%union {
    struct node *ast;
}

%type <ast> expr term factor

%%

expr: expr PLUS term {
        $$ = make_binary_node(OP_ADD, $1, $3);
    }
    | term {
        $$ = $1;
    }
    ;
```

## Language-Specific Actions

### C

```yacc
expr: expr PLUS expr    { $$ = $1 + $3; }
    ;
```

### Python

```yacc
expr: expr PLUS expr    { self.yyval = self.yyvs[-3] + self.yyvs[-1] }
    ;
```

### Java

```yacc
expr: expr PLUS expr    { yyval = yyvs[yyvsp-2] + yyvs[yyvsp]; }
    ;
```

## Empty Actions

Rules without explicit actions get a default that copies `$1` to `$$`:

```yacc
term: factor    /* implicit: { $$ = $1; } */
    ;
```
