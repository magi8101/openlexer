# Lexer File Format (.l)

The lexer specification file has three sections separated by `%%`:

```
DEFINITIONS
%%
RULES
%%
USER CODE
```

## Definitions Section

The definitions section contains:

### Prologue Code

Code enclosed in `%{` and `%}` is copied directly to the output:

```lex
%{
#include <stdio.h>
int line_count = 0;
%}
```

### Named Patterns

Named patterns can be referenced in rules using `{name}`:

```lex
DIGIT       [0-9]
ALPHA       [a-zA-Z]
ALNUM       [a-zA-Z0-9]
ID          {ALPHA}{ALNUM}*
```

### Start Condition Declarations

Declare exclusive (`%x`) or inclusive (`%s`) start conditions:

```lex
%x COMMENT
%s STRING
```

## Rules Section

Each rule has a pattern and an action:

```lex
%%

{ID}            { return IDENTIFIER; }
{DIGIT}+        { return NUMBER; }
"/*"            { BEGIN(COMMENT); }
<COMMENT>"*/"   { BEGIN(INITIAL); }
<COMMENT>.      { /* skip */ }
[ \t\n]+        { /* skip whitespace */ }

%%
```

### Rule Syntax

```
[<start_condition>]pattern    { action }
```

- Patterns match from left to right
- Longer matches take priority
- Earlier rules break ties
- Actions are code blocks that can return tokens

### Special Variables

- `yytext`: The matched text (string)
- `yyleng`: Length of matched text
- `yylineno`: Current line number (if enabled)

## User Code Section

The third section is copied verbatim to the end of the output file:

```lex
%%

int main() {
    while (yylex() != 0) {
        printf("Token: %s\n", yytext);
    }
    return 0;
}
```

## Complete Example

```lex
%{
/* Token definitions */
#define NUMBER 1
#define PLUS 2
#define MINUS 3
%}

DIGIT   [0-9]

%%

{DIGIT}+    { return NUMBER; }
"+"         { return PLUS; }
"-"         { return MINUS; }
[ \t\n]+    { /* skip */ }
.           { fprintf(stderr, "Unknown: %s\n", yytext); }

%%
```
