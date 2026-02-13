# Test Drivers

OpenLexer generates **test drivers** - standalone runnable code that lets you verify your lexer or parser works correctly without writing any integration code.

## What Are Test Drivers?

A test driver is a `main()` function (or equivalent) appended to your generated code that:

1. Accepts input from command-line arguments or stdin
2. Runs the lexer/parser on that input
3. Prints detailed output showing tokens or parse results
4. Helps you debug patterns and grammar rules

## Enabling Test Drivers

### Command Line

Test drivers are **included by default**. To exclude them:

```bash
# Include test driver (default)
openlexer --lexer rules.l --lang python -o lexer.py

# Exclude test driver
openlexer --lexer rules.l --lang python -o lexer.py --no-test

# Generate ONLY the test driver
openlexer --test-driver --lang python -o test_driver.py
```

### GUI

In the GUI, check the **"Test Driver"** option in the Lexer or Parser options panel before generating.

## Using Test Drivers

### Python

```bash
# Test with command-line expressions
python lexer.py "3 + 4 * 2"
python lexer.py "hello" "world" "123"

# Interactive testing from Python
from lexer import test, test_all

test('3 + 4 * 2')
test_all('1+2', '(3*4)', 'x = 10')
```

Output:
```
Input: '3 + 4 * 2'
  NUMBER       | '3'             | pos=0 line=1 col=1
  PLUS         | '+'             | pos=2 line=1 col=3
  NUMBER       | '4'             | pos=4 line=1 col=5
  TIMES        | '*'             | pos=6 line=1 col=7
  NUMBER       | '2'             | pos=8 line=1 col=9
```

### C

```bash
# Compile with test driver
gcc -o lexer lexer.c

# Test expressions
./lexer "3 + 4 * 2"
./lexer "hello" "123"

# Compile WITHOUT test driver (for library use)
gcc -DLEXER_NO_MAIN -c lexer.c -o lexer.o
```

Output:
```
Input: "3 + 4 * 2"
  type=1   | "3"
  type=2   | "+"
  type=1   | "4"
  type=3   | "*"
  type=1   | "2"
  type=0   | ""
```

### Java

```bash
# Compile
javac Lexer.java

# Test expressions
java Lexer "3 + 4 * 2"
java Lexer "hello" "123"
```

## Test Driver API

### Python Test Driver Functions

| Function | Description |
|----------|-------------|
| `test(expr)` | Tokenize one expression, print tokens, return list |
| `test_all(*exprs)` | Test multiple expressions, return list of results |

### C Test Driver

The C test driver is wrapped in `#ifndef LEXER_NO_MAIN` so you can disable it:

```c
// To compile as a library without main():
#define LEXER_NO_MAIN
#include "lexer.c"

// Your code that uses Lexer API
```

### Java Test Driver

The Java test driver adds a `main` method to the `Lexer` class:

```java
public static void main(String[] args) {
    // Tests each argument
}
```

## Parser Test Drivers

Parser test drivers integrate both lexer and parser:

```bash
# Python
python parser.py "3 + 4 * 2"

# C
./parser "3 + 4 * 2"

# Java
java Parser "3 + 4 * 2"
```

Parser test drivers show:
- Tokenization phase output
- Parsing phase with shift/reduce actions
- Final parse result or error messages

## Customizing Test Behavior

### Python

Modify the test driver in the generated file:

```python
if __name__ == '__main__':
    # Add your custom test cases
    test('my custom expression')
    
    # Or read from file
    with open('input.txt') as f:
        test(f.read())
```

### C

Override the default expressions in the `else` branch of `main()`:

```c
} else {
    // Your custom default tests
    test("custom expression 1");
    test("custom expression 2");
}
```

## Best Practices

1. **Use test drivers during development** - They're the fastest way to verify patterns
2. **Remove for production** - Use `--no-test` when building release versions
3. **Keep test expressions** - Save useful test cases in a script or file
4. **Check edge cases** - Test empty input, long input, Unicode, special characters

## Example Workflows

### Python Workflow

```bash
# 1. Generate lexer with test driver
openlexer --lexer calc.l --lang python -o lexer.py

# 2. Test your patterns
python lexer.py "3+4" "(10-2)/4" "x=y*z"

# 3. Fix any issues, regenerate, retest
openlexer --lexer calc.l --lang python -o lexer.py
python lexer.py "3+4"

# 4. Generate production version without test driver
openlexer --lexer calc.l --lang python -o lexer.py --no-test
```

### C Workflow

```bash
# 1. Generate lexer with test driver
openlexer --lexer calc.l --lang c -o lexer.c

# 2. Compile the lexer
gcc -o lexer lexer.c -Wall

# 3. Test your patterns
./lexer "3+4"
./lexer "(10-2)/4"
./lexer "x=y*z"

# 4. Fix any issues, regenerate, recompile, retest
openlexer --lexer calc.l --lang c -o lexer.c
gcc -o lexer lexer.c -Wall
./lexer "3+4"

# 5. Generate production version without test driver
openlexer --lexer calc.l --lang c -o lexer.c --no-test

# 6. Compile as library (for use in your project)
gcc -DLEXER_NO_MAIN -c lexer.c -o lexer.o
```

**Complete C Example Program:**

```c
// main.c - Using the generated lexer in your own program
#define LEXER_NO_MAIN  // Exclude the built-in test driver
#include "lexer.c"

#include <stdio.h>
#include <stdlib.h>

void process_file(const char* filename) {
    FILE* f = fopen(filename, "r");
    if (!f) {
        fprintf(stderr, "Cannot open file: %s\n", filename);
        return;
    }
    
    // Read entire file
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    
    char* buffer = malloc(size + 1);
    fread(buffer, 1, size, f);
    buffer[size] = '\0';
    fclose(f);
    
    // Tokenize
    Lexer lexer;
    lexer_init(&lexer, buffer);
    
    Token token;
    int count = 0;
    
    printf("Tokens in %s:\n", filename);
    do {
        token = lexer_next(&lexer);
        printf("  [%3d] type=%-3d \"", count++, token.type);
        for (int i = 0; i < token.length; i++) {
            putchar(token.start[i]);
        }
        printf("\"\n");
    } while (token.type != TOKEN_EOF);
    
    printf("Total: %d tokens\n\n", count);
    free(buffer);
}

int main(int argc, char** argv) {
    if (argc < 2) {
        printf("Usage: %s <file1> [file2] ...\n", argv[0]);
        printf("       %s --expr \"expression\"\n", argv[0]);
        return 1;
    }
    
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--expr") == 0 && i + 1 < argc) {
            // Tokenize expression directly
            Lexer lexer;
            lexer_init(&lexer, argv[++i]);
            
            Token token;
            printf("Expression: \"%s\"\n", argv[i]);
            do {
                token = lexer_next(&lexer);
                printf("  type=%-3d \"", token.type);
                for (int j = 0; j < token.length; j++) {
                    putchar(token.start[j]);
                }
                printf("\"\n");
            } while (token.type != TOKEN_EOF);
        } else {
            process_file(argv[i]);
        }
    }
    
    return 0;
}
```

**Build and run:**

```bash
# Compile your program with the lexer
gcc -o myprogram main.c -Wall

# Process files
./myprogram source1.txt source2.txt

# Or tokenize an expression
./myprogram --expr "3 + 4 * 2"
```

### Java Workflow

```bash
# 1. Generate lexer with test driver
openlexer --lexer calc.l --lang java -o Lexer.java

# 2. Compile the lexer
javac Lexer.java

# 3. Test your patterns
java Lexer "3+4"
java Lexer "(10-2)/4"
java Lexer "x=y*z"

# 4. Fix any issues, regenerate, recompile, retest
openlexer --lexer calc.l --lang java -o Lexer.java
javac Lexer.java
java Lexer "3+4"

# 5. Generate production version without test driver
openlexer --lexer calc.l --lang java -o Lexer.java --no-test
```

**Complete Java Example Program:**

```java
// Calculator.java - Using the generated lexer in your own program
import java.io.*;
import java.nio.file.*;
import java.util.*;

public class Calculator {
    
    public static void main(String[] args) {
        if (args.length == 0) {
            System.out.println("Usage: java Calculator <expression>");
            System.out.println("       java Calculator --file <filename>");
            System.out.println("       java Calculator --interactive");
            return;
        }
        
        Calculator calc = new Calculator();
        
        if (args[0].equals("--file") && args.length > 1) {
            calc.processFile(args[1]);
        } else if (args[0].equals("--interactive")) {
            calc.interactiveMode();
        } else {
            // Process each argument as an expression
            for (String expr : args) {
                calc.tokenize(expr);
            }
        }
    }
    
    public void tokenize(String expression) {
        System.out.println("Input: \"" + expression + "\"");
        
        Lexer lexer = new Lexer(expression);
        List<Token> tokens = new ArrayList<>();
        
        try {
            for (Token token : lexer.tokenize()) {
                tokens.add(token);
                System.out.printf("  %-12s | %-15s | pos=%d%n",
                    token.type.name(),
                    "\"" + token.text + "\"",
                    token.pos);
            }
        } catch (LexerException e) {
            System.err.println("Lexer error: " + e.getMessage());
        }
        
        System.out.println("Total: " + tokens.size() + " tokens\n");
    }
    
    public void processFile(String filename) {
        try {
            String content = Files.readString(Path.of(filename));
            System.out.println("=== File: " + filename + " ===\n");
            tokenize(content);
        } catch (IOException e) {
            System.err.println("Cannot read file: " + filename);
            System.err.println(e.getMessage());
        }
    }
    
    public void interactiveMode() {
        Scanner scanner = new Scanner(System.in);
        System.out.println("OpenLexer Interactive Mode");
        System.out.println("Enter expressions to tokenize (empty line to quit):\n");
        
        while (true) {
            System.out.print("> ");
            String line = scanner.nextLine();
            
            if (line.isEmpty()) {
                System.out.println("Goodbye!");
                break;
            }
            
            tokenize(line);
        }
        
        scanner.close();
    }
}
```

**Build and run:**

```bash
# Compile your program (assumes Lexer.java is in same directory)
javac Calculator.java Lexer.java

# Tokenize expressions
java Calculator "3 + 4 * 2"
java Calculator "hello" "world"

# Process a file
java Calculator --file source.txt

# Interactive mode
java Calculator --interactive
> 3 + 4
  NUMBER       | "3"             | pos=0
  PLUS         | "+"             | pos=2
  NUMBER       | "4"             | pos=4
Total: 3 tokens

> quit
Goodbye!
```

## IDE Integration

### VS Code Tasks

Create `.vscode/tasks.json`:

```json
{
    "version": "2.0.0",
    "tasks": [
        {
            "label": "Generate & Test Lexer",
            "type": "shell",
            "command": "openlexer --lexer ${file} --lang python -o lexer.py && python lexer.py \"test\"",
            "group": "build"
        }
    ]
}
```

### Makefile

```makefile
LANG ?= c

lexer.$(LANG): rules.l
	openlexer --lexer $< --lang $(LANG) -o $@

test: lexer.$(LANG)
ifeq ($(LANG),c)
	gcc -o lexer $< && ./lexer "test expression"
else ifeq ($(LANG),java)
	javac $< && java Lexer "test expression"
else
	python $< "test expression"
endif

clean:
	rm -f lexer lexer.c lexer.py Lexer.java *.class
```
