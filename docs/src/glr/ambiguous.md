# Ambiguous Grammars

Ambiguous grammars have multiple valid parse trees for the same input.

## Expression Ambiguity

Without precedence or associativity, binary operators are ambiguous:

```yacc
%glr-parser

%%

expr: expr MINUS expr
    | expr PLUS expr
    | expr TIMES expr
    | NUMBER
    ;

%%
```

For `1 - 2 - 3`:

Parse tree 1 (left-associative):
```
    expr
   / | \
 expr - expr
 / \     |
expr -  3
 |    \
 1     2
```
Result: `(1 - 2) - 3 = -4`

Parse tree 2 (right-associative):
```
    expr
   / | \
expr  - expr
  |    / | \
  1  expr - expr
       |     |
       2     3
```
Result: `1 - (2 - 3) = 2`

## Dangling Else

The classic ambiguity in C-like languages:

```yacc
%glr-parser

%%

stmt: IF expr THEN stmt
    | IF expr THEN stmt ELSE stmt
    | other_stmt
    ;

%%
```

For `if a then if b then s1 else s2`:

- `if a then (if b then s1 else s2)` - else binds to inner if
- `if a then (if b then s1) else s2` - else binds to outer if

## Type or Expression

In C, `(x)(y)` could be:
- A cast of `y` to type `x`
- A function call of `x` with argument `y`

```yacc
%glr-parser

%%

expr: LPAREN type RPAREN expr
    | expr LPAREN expr RPAREN
    | IDENTIFIER
    ;

%%
```

## Resolving Ambiguity at Runtime

With GLR, you can defer disambiguation to semantic actions:

```yacc
%glr-parser

%%

primary: IDENTIFIER {
        if (is_type($1)) {
            $$ = make_type_node($1);
        } else {
            $$ = make_var_node($1);
        }
    }
    ;

%%
```

## Comparison with LALR

| Feature | LALR(1) | GLR |
|---------|---------|-----|
| Ambiguous grammars | Error | Supported |
| Multiple parse trees | No | Yes (SPPF) |
| Performance | O(n) | O(n) to O(n^3) |
| Conflict handling | Static | Dynamic |

## Best Practices

1. Use LALR with precedence when possible (more efficient)
2. Use GLR when the grammar is inherently ambiguous
3. Keep the ambiguous portion of the grammar small
4. Test with known ambiguous inputs
