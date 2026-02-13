# GLR Parsing

GLR (Generalized LR) parsing handles ambiguous and non-deterministic grammars that LALR(1) cannot process.

## When to Use GLR

Use GLR parsing when your grammar has:

- Inherent ambiguity that cannot be resolved with precedence
- Multiple valid parse trees for the same input
- Complex language constructs requiring more than one token of lookahead

## Enabling GLR Mode

Add `%glr-parser` to the declarations section:

```yacc
%glr-parser

%%

expr: expr MINUS expr
    | NUMBER
    ;

%%
```

## How GLR Works

### Graph-Structured Stack (GSS)

Instead of a single parse stack, GLR maintains a graph of stacks:

```
     [0] ─── expr ─── [3] ─── MINUS ─── [4] ─── expr ─── [7]
      │                                          │
      └─────── expr ─────────────────────────────┘
```

When a conflict occurs, GLR forks the stack and explores both paths.

### Shared Packed Parse Forest (SPPF)

Multiple parse trees are represented compactly:

```
         expr
        / | \
      expr - expr
       |     / \
       1   expr - expr
            |     |
            2     3
```

For `1 - 2 - 3`, the SPPF captures both:
- `(1 - 2) - 3` 
- `1 - (2 - 3)`

## Conflict Handling

In standard LALR(1), conflicts are resolved statically. In GLR:

- **Shift/Reduce**: Both shift and reduce occur on separate stack branches
- **Reduce/Reduce**: Both reductions occur

Invalid branches are pruned when they encounter errors.

## Performance

GLR parsing is O(n^3) in the worst case for highly ambiguous grammars. For mostly unambiguous grammars with occasional conflicts, it approaches O(n) LALR performance.

## Chapters

- [Ambiguous Grammars](./ambiguous.md) - Writing grammars with intentional ambiguity
- [GLR Algorithm](./algorithm.md) - Technical details of the implementation
