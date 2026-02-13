# Start Conditions

Start conditions allow the lexer to switch between different sets of rules. This is useful for handling comments, strings, and other context-dependent lexing.

## Declaring Start Conditions

Use `%x` for exclusive conditions or `%s` for inclusive conditions:

```lex
%x COMMENT
%x STRING
%s SPECIAL
```

- **Exclusive (`%x`)**: Only rules with this condition are active.
- **Inclusive (`%s`)**: Rules with this condition AND rules without conditions are active.

## Using Start Conditions

### Applying to Rules

Prefix a rule with the condition name in angle brackets:

```lex
<COMMENT>.      { /* inside comment */ }
<STRING>[^"]+   { /* inside string */ }
```

### Multiple Conditions

Specify multiple conditions separated by commas:

```lex
<COMMENT,STRING>.   { /* in comment or string */ }
```

### Initial Condition

Rules without a condition apply in the `INITIAL` state:

```lex
[a-z]+      { return WORD; }   /* applies in INITIAL */
```

Or explicitly:

```lex
<INITIAL>[a-z]+     { return WORD; }
```

## Switching Conditions

Use `BEGIN(condition)` to switch:

```lex
"/*"            { BEGIN(COMMENT); }
<COMMENT>"*/"   { BEGIN(INITIAL); }
```

## Example: C-Style Comments

```lex
%x COMMENT

%%

"/*"            { BEGIN(COMMENT); }
<COMMENT>"*/"   { BEGIN(INITIAL); }
<COMMENT>\n     { yylineno++; }
<COMMENT>.      { /* skip comment content */ }

%%
```

## Example: String Literals

```lex
%x STRING

%%

\"              { BEGIN(STRING); string_buf_ptr = string_buf; }
<STRING>\"      { 
    BEGIN(INITIAL);
    *string_buf_ptr = '\0';
    yylval.str = strdup(string_buf);
    return STRING_LITERAL;
}
<STRING>\\n     { *string_buf_ptr++ = '\n'; }
<STRING>\\t     { *string_buf_ptr++ = '\t'; }
<STRING>\\\\    { *string_buf_ptr++ = '\\'; }
<STRING>\\.     { *string_buf_ptr++ = yytext[1]; }
<STRING>[^\\\"]+ {
    char *p = yytext;
    while (*p) *string_buf_ptr++ = *p++;
}

%%
```

## Example: Nested Comments

For languages with nested comments, use a counter:

```lex
%x COMMENT

%{
int comment_depth = 0;
%}

%%

"(*"            { comment_depth++; BEGIN(COMMENT); }
<COMMENT>"(*"   { comment_depth++; }
<COMMENT>"*)"   { 
    if (--comment_depth == 0) BEGIN(INITIAL);
}
<COMMENT>.      { /* skip */ }
<COMMENT>\n     { yylineno++; }

%%
```
