# Python Output

## Generated Files

- `lexer.py` - Lexer class with DFA tables
- `parser.py` - Parser class with LALR tables

## Lexer Interface

```python
class Lexer:
    # Token constants
    EOF = 0
    NUMBER = 1
    PLUS = 2
    # ...
    
    def __init__(self, input_string: str):
        """Initialize lexer with input string."""
        
    def next_token(self) -> int:
        """Return the next token type."""
        
    @property
    def text(self) -> str:
        """Return the matched text."""
        
    @property
    def value(self):
        """Return the semantic value."""
```

## Parser Interface

```python
class Parser:
    def __init__(self, lexer: Lexer):
        """Initialize parser with lexer."""
        
    def parse(self):
        """Parse input and return result."""
```

## Integration Example

```python
#!/usr/bin/env python3
import sys
from lexer import Lexer
from parser import Parser

def main():
    if len(sys.argv) < 2:
        print("Usage: python main.py <expression>", file=sys.stderr)
        sys.exit(1)
    
    lexer = Lexer(sys.argv[1])
    parser = Parser(lexer)
    
    try:
        result = parser.parse()
        print(f"Result: {result}")
    except Exception as e:
        print(f"Parse error: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
```

## Execution

```bash
python main.py "3 + 4 * 2"
```

## Semantic Values

Set in lexer actions:

```python
def next_token(self):
    # In NUMBER rule:
    self._value = int(self._text)
    return self.NUMBER
```

## Error Handling

```python
class ParseError(Exception):
    def __init__(self, message, line=None, column=None, token=None):
        super().__init__(message)
        self.line = line
        self.column = column
        self.token = token
```

## Reading from Files

```python
with open(filename, 'r') as f:
    content = f.read()

lexer = Lexer(content)
parser = Parser(lexer)
result = parser.parse()
```

## Reading from stdin

```python
import sys

content = sys.stdin.read()
lexer = Lexer(content)
parser = Parser(lexer)
result = parser.parse()
```

## Type Hints

The generated code includes type hints compatible with Python 3.6+:

```python
def next_token(self) -> int:
    ...

def parse(self) -> Any:
    ...
```

## Dependencies

No external dependencies. Uses only the Python standard library.

## Python Version Compatibility

- Python 3.6+
- Uses f-strings and type hints
- No walrus operator (3.8+) for broader compatibility
