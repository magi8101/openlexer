# Actions and Return Values

Each lexer rule has an action block that executes when the pattern matches.

## Basic Actions

Return a token type:

```lex
[0-9]+      { return NUMBER; }
"+"         { return PLUS; }
```

Skip input without returning:

```lex
[ \t\n]+    { /* skip whitespace */ }
```

## Available Variables

### yytext

The matched text as a string:

```lex
[a-z]+      { printf("Matched: %s\n", yytext); return WORD; }
```

### yyleng

Length of the matched text:

```lex
[a-z]+      { printf("Length: %d\n", yyleng); return WORD; }
```

### yylval

Semantic value passed to the parser. Set in the action:

```lex
[0-9]+      { yylval.ival = atoi(yytext); return NUMBER; }
[0-9]+\.[0-9]+  { yylval.dval = atof(yytext); return FLOAT; }
```

## Multi-Statement Actions

Use braces for multiple statements:

```lex
[0-9]+      {
    int val = atoi(yytext);
    if (val > 1000) {
        fprintf(stderr, "Warning: large number\n");
    }
    yylval.ival = val;
    return NUMBER;
}
```

## Language-Specific Actions

### C Actions

```lex
[a-z]+      {
    yylval.str = strdup(yytext);
    return IDENTIFIER;
}
```

### Python Actions

```lex
[a-z]+      {
    self.yylval = self.yytext
    return self.IDENTIFIER
}
```

### Java Actions

```lex
[a-z]+      {
    yylval = yytext;
    return IDENTIFIER;
}
```

## Token Constants

Token constants should match those declared in the grammar file:

```lex
%{
/* From parser.h or defined here */
#define NUMBER 258
#define PLUS   259
#define MINUS  260
%}

%%

[0-9]+  { return NUMBER; }
"+"     { return PLUS; }
"-"     { return MINUS; }

%%
```

## Error Handling

The `.` pattern matches any single character. Use it as a catch-all:

```lex
.       {
    fprintf(stderr, "Unexpected character: %s\n", yytext);
    return ERROR;
}
```
