# Tokens and Precedence

## Token Declaration

All terminals must be declared:

```yacc
%token NUMBER IDENTIFIER STRING_LITERAL
%token PLUS MINUS TIMES DIVIDE
%token IF ELSE WHILE FOR
```

### Token Values

By default, tokens are assigned sequential integer values starting from 256. To specify values:

```yacc
%token NUMBER 258
%token PLUS 259
%token MINUS 260
```

### Literal Tokens

Single-character tokens can be used directly in rules:

```yacc
expr: expr '+' expr
    | expr '-' expr
    | NUMBER
    ;
```

The character's ASCII value is the token value.

## Precedence and Associativity

### Associativity

```yacc
%left PLUS MINUS      /* left-to-right */
%left TIMES DIVIDE    /* higher precedence than + - */
%right POWER          /* right-to-left */
%nonassoc LT GT EQ    /* a < b < c is an error */
```

### Precedence Levels

Declarations later in the file have higher precedence:

```yacc
%left OR              /* lowest */
%left AND
%left EQ NE
%left LT GT LE GE
%left PLUS MINUS
%left TIMES DIVIDE
%right POWER          /* highest binary */
%right UMINUS         /* unary minus */
```

### Context-Dependent Precedence

Use `%prec` to override a rule's precedence:

```yacc
expr: MINUS expr %prec UMINUS   { $$ = -$2; }
    ;
```

This uses `UMINUS` precedence instead of `MINUS`.

## Conflict Resolution

### Shift/Reduce Conflicts

When the parser can either shift or reduce:

1. If the lookahead token has higher precedence than the rule, shift.
2. If the rule has higher precedence, reduce.
3. If equal precedence, use associativity:
   - Left: reduce
   - Right: shift
   - Nonassoc: error

### Example: Dangling Else

```yacc
%nonassoc LOWER_THAN_ELSE
%nonassoc ELSE

stmt: IF expr stmt %prec LOWER_THAN_ELSE
    | IF expr stmt ELSE stmt
    ;
```

### Reduce/Reduce Conflicts

When two rules can reduce the same input, the first rule in the file wins. This usually indicates a grammar problem.

## Token Types

With `%union`, assign types to tokens:

```yacc
%union {
    int ival;
    double dval;
    char *str;
}

%token <ival> INTEGER
%token <dval> FLOAT
%token <str> IDENTIFIER STRING
```
