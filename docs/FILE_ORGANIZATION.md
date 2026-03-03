# File Organization Guide for Generated Code

## Overview

OpenLexer generates lexer and parser code in C, Java, and Python. This guide explains how to properly organize and use the generated files.

## Java File Organization

### The "One Public Class Per File" Rule

**Java requires that each .java file contains only ONE public class, and the filename must match that class name.**

### Generating Lexer Only

```bash
openlexer gen-lexer --lexer grammar.l -L java -o output/
```

**Output:** `output/Lexer.java`
- Contains: `public class Lexer`
- Includes: Token class, lexer logic, test driver

**Compilation & Usage:**
```bash
javac Lexer.java
java Lexer "3 + 4 * 2"
```

### Generating Parser Only

```bash
openlexer gen-parser --parser grammar.y -L java -o output/
```

**Output:** `output/Parser.java`
- Contains: `public class Parser`
- Includes: Inline lexer, parser logic, test driver
- Can run standalone or detect external Lexer.class

**Compilation & Usage:**
```bash
javac Parser.java
java Parser "3 + 4 * 2"
```

### Generating Both Lexer and Parser

**Method 1: Separate Files (Recommended)**

```bash
openlexer gen-lexer --lexer grammar.l -L java -o output/
openlexer gen-parser --parser grammar.y -L java -o output/
```

**Output:**
- `output/Lexer.java` - public class Lexer
- `output/Parser.java` - public class Parser

**Compilation & Usage:**
```bash
# Compile both
javac Lexer.java Parser.java

# Run parser (automatically detects and uses Lexer.class)
java Parser "3 + 4 * 2"
# Output: [Using external Lexer.class]
```

**Method 2: Combined File (Advanced)**

If you need both in one file for deployment, make one class non-public:

```java
// File: Lexer.java
public class Lexer {
    // ... lexer code ...
}

class Parser {  // Note: not public!
    // ... parser code ...
}
```

Compile and run:
```bash
javac Lexer.java
java Lexer  # or java Parser if Parser is public
```

## Token Interface

### Lexer Token Format

All Java lexers return a `Token` object with consistent structure:

```java
public static class Token {
    public final int type;      // Token type constant
    public final String text;   // Lexeme text
    public final int pos;       // Position in input
    
    public Token(int type, String text, int pos) {
        this.type = type;
        this.text = text;
        this.pos = pos;
    }
}
```

### Parser-Lexer Integration

The Parser automatically detects and uses an external Lexer if available:

1. **With external Lexer:**
   - Parser calls `Lexer.nextToken()` via reflection
   - Displays: `[Using external Lexer.class]`

2. **Without external Lexer:**
   - Parser uses its inline lexer
   - Displays: `[Using inline lexer]`

## C File Organization

### Generating Lexer

```bash
openlexer gen-lexer --lexer grammar.l -L c -o output/
```

**Output:** `output/lexer.c`
- Includes headers, lexer logic, test driver

**Compilation & Usage:**
```bash
gcc -o lexer lexer.c
./lexer "3 + 4 * 2"
```

### Generating Parser

```bash
openlexer gen-parser --parser grammar.y -L c -o output/
```

**Output:** `output/parser.c`
- Includes inline lexer, parser logic, test driver

**Compilation & Usage:**
```bash
gcc -o parser parser.c
./parser "3 + 4 * 2"
```

### Combining Lexer and Parser

```bash
# Generate both
openlexer gen-lexer --lexer grammar.l -L c -o output/
openlexer gen-parser --parser grammar.y -L c -o output/

# Method 1: Compile separately and link
gcc -c lexer.c -o lexer.o
gcc -c parser.c -o parser.o
gcc lexer.o parser.o -o myapp

# Method 2: Combine into one compilation unit
cat lexer.c parser.c > combined.c
gcc -o myapp combined.c
```

**Suppressing Test Drivers:**

Use preprocessor flags to disable test/main functions:

```bash
# Lexer without test driver
gcc -DLEXER_NO_MAIN -DLEXER_NO_TEST -c lexer.c

# Parser without test driver
gcc -DPARSER_NO_MAIN -c parser.c
```

## Python File Organization

### Generating Lexer

```bash
openlexer gen-lexer --lexer grammar.l -L python -o output/
```

**Output:** `output/lexer.py`
- Lexer class, test functions

**Usage:**
```bash
python lexer.py "3 + 4 * 2"

# Or import in your code:
from lexer import Lexer
lex = Lexer("3 + 4 * 2")
for token in lex.tokenize():
    print(token)
```

### Generating Parser

```bash
openlexer gen-parser --parser grammar.y -L python -o output/
```

**Output:** `output/parser.py`
- Parser and inline Lexer class

**Usage:**
```bash
python parser.py "3 + 4 * 2"

# Or import:
from parser import parse
result = parse("3 + 4 * 2")
```

### Combining Both

```bash
# Generate both
openlexer gen-lexer --lexer grammar.l -L python -o output/
openlexer gen-parser --parser grammar.y -L python -o output/

# Import and use together:
from lexer import Lexer
from parser import Parser

lex = Lexer("3 + 4 * 2")
parser = Parser(lex)
result = parser.parse()
```

## Best Practices

### 1. **Separate Generation**
Generate lexer and parser in separate commands for better modularity.

### 2. **Version Control**
- Commit .l and .y grammar files
- Add generated  files to .gitignore or commit them depending on your workflow

### 3. **Build Scripts**
Create a Makefile or build script:

```makefile
# Makefile
all: lexer parser

lexer:
	openlexer gen-lexer --lexer grammar.l -L java -o build/

parser:
	openlexer gen-parser --parser grammar.y -L java -o build/
	cd build && javac Lexer.java Parser.java

clean:
	rm -rf build/

run:
	cd build && java Parser "3 + 4 * 2"
```

### 4. **Package Structure (Java)**
For larger projects, use Java packages:

```bash
mkdir -p src/com/example/parser
openlexer gen-lexer --lexer grammar.l -L java -o src/com/example/parser/
# Edit generated files to add: package com.example.parser;
```

### 5. **Testing**
Use the built-in test drivers during development:

```bash
# Test lexer
java Lexer "(10 + 20) * 3"

# Test parser (with or without external lexer)
java Parser "(10 + 20) * 3"
```

## Troubleshooting

### Java: "Public class X must be in a file named X.java"

**Solution:** Ensure file name matches the public class name, or make the class non-public.

### Java: "Class Lexer not found" when running Parser

**Causes:**
1. Lexer.java not compiled
2. Lexer.class not in same directory
3. CLASSPATH not set correctly

**Solution:**
```bash
# Compile both in same directory
javac Lexer.java Parser.java

# Or set CLASSPATH
javac -cp /path/to/classes Parser.java
java -cp /path/to/classes Parser
```

### Java: Multiple public classes error

**Solution:** Only ONE public class per .java file. Make others non-public:
```java
public class Lexer { ... }
class Parser { ... }  // No 'public'
```

### C: Undefined reference to lexer functions

**Solution:** Ensure proper linking:
```bash
gcc lexer.c parser.c -o myapp
# Or link separately
gcc -c lexer.c && gcc -c parser.c && gcc lexer.o parser.o -o myapp
```

## Advanced: Custom Integration

### Java: Manual Lexer-Parser Integration

```java
// Main.java
public class Main {
    public static void main(String[] args) {
        Lexer lexer = new Lexer("3 + 4 * 2");
        Parser parser = new Parser();
        
        // Custom token loop
        Lexer.Token tok;
        while ((tok = lexer.nextToken()).type != Lexer.TOKEN_EOF) {
            // Process tokenparser.consumeToken(tok);
        }
        
        int result = parser.getResult();
        System.out.println("Result: " + result);
    }
}
```

### Python: Custom Integration

```python
# main.py
from lexer import Lexer, TokenType
from parser import Parser

def custom_parse(input_str):
    lex = Lexer(input_str)
    tokens = list(lex.tokenize())
    
    # Filter or transform tokens
    filtered = [t for t in tokens if t.type != TokenType.WHITESPACE]
    
    # Parse
    parser = Parser()
    return parser.parse_tokens(filtered)

result = custom_parse("3 + 4 * 2")
print(f"Result: {result}")
```

## Summary

| Language | Lexer File   | Parser File    | Test Command                          |
|----------|-------------|----------------|---------------------------------------|
| Java     | Lexer.java  | Parser.java    | `javac *.java && java Parser "input"` |
| C        | lexer.c     | parser.c       | `gcc *.c -o app && ./app "input"`     |
| Python   | lexer.py    | parser.py      | `python parser.py "input"`            |

**Key Points:**
- Java: One public class per file
- Parser can detect and use external Lexer automatically
- Use preprocessor flags in C to disable test drivers
- Python files can be imported as modules
