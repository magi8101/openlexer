# OpenLexer

OpenLexer is a lexer and parser generator written in Rust. It reads `.l` (Flex-compatible) and `.y` (Bison-compatible) specification files and generates lexers and parsers in C, Java, or Python.

## Components

- **Lexer Generator**: Converts regular expression patterns to DFA-based lexers using Thompson construction and subset construction algorithms.
- **Parser Generator**: Builds LALR(1) parsing tables from context-free grammars. Supports GLR parsing for ambiguous grammars.
- **Code Generation**: Outputs standalone lexer and parser code in C, Java, or Python.

## Supported Platforms

- Windows (x64)
- Linux (x64, ARM64)
- macOS (x64, ARM64)

## File Formats

OpenLexer uses the standard Flex/Bison file formats:

- `.l` files: Lexer specifications with regex patterns and actions
- `.y` files: Grammar specifications with production rules and semantic actions

## Basic Usage

```bash
# Generate a Python lexer from calc.l
openlexer gen-lexer --lexer calc.l --lang python --output ./

# Generate a Java parser from calc.y 
openlexer gen-parser --parser calc.y --lang java --output ./

# For Java: compile and run
javac Lexer.java Parser.java
java Parser "3 + 4 * 2"
```

## Key Features

- **Smart File Organization**: Generated code follows language-specific best practices
  - Java: One public class per file, automatic lexer detection
  - C: Flexible compilation with preprocessor controls
  - Python: Module-based imports
- **Complete Integration**: Lexer and parser work seamlessly together
- **Standalone or Combined**: Each component can work independently or together
- **Built-in Test Drivers**: Generated code includes test scaffolding

## Documentation Structure

- [Getting Started](./getting-started.md) - Installation and first steps
- [Lexer Generator](./lexer/index.md) - Writing lexer specifications
- [Parser Generator](./parser/index.md) - Writing grammar specifications
- [GLR Parsing](./glr/index.md) - Handling ambiguous grammars
- [Output Languages](./output/index.md) - Language-specific details
- [Examples](./examples/calculator.md) - Complete working examples

## License

MIT License
