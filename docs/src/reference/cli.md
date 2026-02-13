# Command Line Interface

## Synopsis

```
openlexer <command> [options]
```

## Commands

### gen-lexer

Generate a lexer from a `.l` file.

```bash
openlexer gen-lexer --lexer <file.l> --lang <language> --output <dir>
```

**Options:**

| Option | Description |
|--------|-------------|
| `--lexer <file>` | Input lexer specification file |
| `--lang <lang>` | Target language: `c`, `java`, `python` |
| `--output <dir>` | Output directory |

**Example:**

```bash
openlexer gen-lexer --lexer calc.l --lang python --output ./build
```

### gen-parser

Generate a parser from a `.y` file.

```bash
openlexer gen-parser --parser <file.y> --lang <language> --output <dir>
```

**Options:**

| Option | Description |
|--------|-------------|
| `--parser <file>` | Input grammar specification file |
| `--lang <lang>` | Target language: `c`, `java`, `python` |
| `--output <dir>` | Output directory |

**Example:**

```bash
openlexer gen-parser --parser calc.y --lang c --output ./build
```

### help

Show help information.

```bash
openlexer help
openlexer --help
openlexer <command> --help
```

### version

Show version information.

```bash
openlexer --version
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |

## Output

The generator prints progress information to stdout:

```
Parsing lexer specification...
Building NFA (23 states)...
Converting to DFA (15 states)...
Minimizing DFA (12 states)...
Generating Python code...
Written: ./build/lexer.py
```

Errors are printed to stderr:

```
Error: Invalid regex at line 5: unclosed bracket
```

## Combining Lexer and Parser

Generate both in a single directory:

```bash
mkdir build
openlexer gen-lexer --lexer calc.l --lang c --output build
openlexer gen-parser --parser calc.y --lang c --output build
```

The generated files can then be compiled together.
