# Regular Expressions

OpenLexer supports the following regular expression syntax, compatible with Flex.

## Character Classes

| Syntax | Description |
|--------|-------------|
| `.` | Any character except newline |
| `[abc]` | Character class: a, b, or c |
| `[a-z]` | Range: lowercase letters |
| `[^abc]` | Negated class: not a, b, or c |
| `[a-zA-Z0-9_]` | Combined ranges |

## Predefined Classes

| Syntax | Equivalent |
|--------|------------|
| `[:alpha:]` | `[a-zA-Z]` |
| `[:digit:]` | `[0-9]` |
| `[:alnum:]` | `[a-zA-Z0-9]` |
| `[:space:]` | `[ \t\n\r\f\v]` |
| `[:upper:]` | `[A-Z]` |
| `[:lower:]` | `[a-z]` |

Use inside character classes: `[[:alpha:]_]`

## Quantifiers

| Syntax | Description |
|--------|-------------|
| `*` | Zero or more |
| `+` | One or more |
| `?` | Zero or one |
| `{n}` | Exactly n |
| `{n,}` | n or more |
| `{n,m}` | Between n and m |

## Anchors

| Syntax | Description |
|--------|-------------|
| `^` | Start of line |
| `$` | End of line |

## Grouping and Alternation

| Syntax | Description |
|--------|-------------|
| `(ab)` | Group |
| `a\|b` | Alternation: a or b |

## Escape Sequences

| Syntax | Description |
|--------|-------------|
| `\n` | Newline |
| `\t` | Tab |
| `\r` | Carriage return |
| `\\` | Literal backslash |
| `\.` | Literal dot |
| `\*` | Literal asterisk |

## Literal Strings

Double-quoted strings match literally:

```lex
"while"     { return WHILE; }
"=="        { return EQ; }
"++"        { return INCREMENT; }
```

## Named Pattern References

Reference definitions with braces:

```lex
/* Definition */
DIGIT   [0-9]

%%

/* Rule using definition */
{DIGIT}+    { return NUMBER; }
```

## Examples

```lex
/* Integer literal */
[0-9]+                      { return INTEGER; }

/* Floating point */
[0-9]+\.[0-9]+              { return FLOAT; }

/* Identifier */
[a-zA-Z_][a-zA-Z0-9_]*      { return IDENTIFIER; }

/* C-style string */
\"([^"\\]|\\.)*\"           { return STRING; }

/* Single-line comment */
"//".*                      { /* skip */ }
```
