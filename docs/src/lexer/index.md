# Lexer Generator

The lexer generator converts regular expression patterns into a deterministic finite automaton (DFA) that tokenizes input text.

## How It Works

1. **Parsing**: The `.l` file is parsed to extract patterns, actions, and declarations.
2. **NFA Construction**: Each regex pattern is converted to an NFA using Thompson's construction.
3. **NFA Combination**: All pattern NFAs are combined with epsilon transitions.
4. **DFA Construction**: The combined NFA is converted to a DFA using subset construction.
5. **DFA Minimization**: The DFA is minimized to reduce state count.
6. **Code Generation**: The DFA transition table is emitted in the target language.

## Input Format

Lexer specifications use the Flex `.l` file format:

```
%{
/* C/Python/Java prologue code */
%}

/* Definitions */
DIGIT   [0-9]
LETTER  [a-zA-Z]

%%

/* Rules */
{DIGIT}+        { return NUMBER; }
{LETTER}+       { return IDENTIFIER; }

%%

/* Epilogue code */
```

## Output

The generated lexer provides:

- A `Lexer` class (or struct in C)
- A `next_token()` function that returns the next token
- Token type constants
- Access to matched text via `yytext`
- **Test driver** (optional) for immediate testing

## Test Driver

When enabled, the generated lexer includes a test driver that lets you verify tokenization:

```bash
# Python
python lexer.py "3 + 4 * 2"

# C
./lexer "hello world"

# Java  
java Lexer "test input"
```

See [Test Drivers](../reference/test-drivers.md) for details.

## Using with Parser

The lexer's output feeds into the parser. Ensure token names match between your `.l` and `.y` files:

```lex
[0-9]+   { return NUMBER; }  /* Must match %token NUMBER in parser */
```

See [Combining Lexer and Parser](../reference/integration.md) for complete integration examples.

## Chapters

- [Lexer File Format](./file-format.md) - Complete `.l` file syntax
- [Regular Expressions](./regex.md) - Supported regex syntax
- [Actions and Return Values](./actions.md) - Writing token actions
- [Start Conditions](./start-conditions.md) - Conditional lexing
