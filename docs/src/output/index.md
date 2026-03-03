# Output Languages

OpenLexer generates lexers and parsers in three target languages:

- [C](./c.md) - Standard C99
- [Java](./java.md) - Java 8+
- [Python](./python.md) - Python 3.6+

## Language Selection

Use the `--lang` option:

```bash
openlexer gen-lexer --lexer rules.l --lang c --output ./
openlexer gen-parser --parser grammar.y --lang java --output ./
```

## Output Files

| Language | Lexer Output | Parser Output |
|----------|--------------|---------------|
| C | `lexer.c` | `parser.c` |
| Java | `Lexer.java` | `Parser.java` |
| Python | `lexer.py` | `parser.py` |

## File Organization

Each language has specific file organization requirements and best practices:

- **Java**: Requires one public class per file; generated files are designed to work standalone or together
- **C**: Can be compiled separately or combined; use preprocessor flags to control test drivers
- **Python**: Generated files are importable modules

**📖 See [File Organization Guide](../FILE_ORGANIZATION.md) for detailed instructions on organizing generated code.**

## Common Interface

All generated lexers provide:
- Token type constants
- A method to get the next token
- Access to the matched text

All generated parsers provide:
- A parse method that drives the parsing loop
- Semantic value stack management
- Error reporting hooks

## Integration

The lexer and parser integrate through a common interface:

```
Input Text
    │
    ▼
┌─────────┐
│  Lexer  │ ──▶ Token Stream
└─────────┘
    │
    ▼
┌─────────┐
│ Parser  │ ──▶ Parse Result / AST
└─────────┘
```

Each language chapter describes the specific integration method.
