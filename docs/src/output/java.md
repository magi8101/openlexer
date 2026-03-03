# Java Output

## File Organization

**Important**: Java requires exactly ONE public class per `.java` file, and the filename must match the public class name.

### Generated Files

- **Lexer Only**: Generates `Lexer.java` containing `public class Lexer`
- **Parser Only**: Generates `Parser.java` containing `public class Parser`
- **Both Together**: Generate as separate files and compile together

```bash
# Generate both
openlexer gen-lexer --lexer calc.l -L java -o output/
openlexer gen-parser --parser calc.y -L java -o output/

# Compile both
javac output/Lexer.java output/Parser.java

# Run (Parser auto-detects Lexer.class)
java -cp output Parser "3 + 4 * 2"
```

## Lexer Interface

```java
public class Lexer {
    // Token constants
    public static final int TOKEN_EOF = 0;
    public static final int TOKEN_ERROR = 1;
    public static final int TOKEN_NUMBER = 2;
    public static final int TOKEN_PLUS = 3;
    // ...
    
    // Token class for lexer-parser integration
    public static class Token {
        public final int type;      // Token type ID
        public final String text;   // Matched lexeme
        public final int pos;       // Position in input
        
        public Token(int type, String text, int pos) {
            this.type = type;
            this.text = text;
            this.pos = pos;
        }
    }
    
    // Constructor
    public Lexer(String input);
    
    // Get next token (returns Token object)
    public Token nextToken();
    
    // Get token name
    public static String tokenName(int type);
}
```

## Parser Interface

```java
public class Parser {
    // Parse input string (auto-detects external Lexer if available)
    public static int parse(String input);
    
    // Main method for testing
    public static void main(String[] args);
}
```

### Automatic Lexer Detection

The generated Parser automatically detects if a compiled `Lexer.class` is available:

1. **With external Lexer**: Uses reflection to call `Lexer.nextToken()`
   - Output: `[Using external Lexer.class]`
2. **Without external Lexer**: Falls back to inline lexer
   - Output: `[Using inline lexer]`

This allows the Parser to work standalone OR with a separate Lexer.

## Integration Example

### Method 1: Using Generated Test Drivers

```bash
# Lexer only
javac Lexer.java
java Lexer "3 + 4 * 2"

# Parser with external Lexer
javac Lexer.java Parser.java
java Parser "3 + 4 * 2"
# Output: [Using external Lexer.class]

# Parser standalone (no Lexer.class)
javac Parser.java
java Parser "3 + 4 * 2"
# Output: [Using inline lexer]
```

### Method 2: Custom Integration

```java
public class Main {
    public static void main(String[] args) {
        if (args.length < 1) {
            System.err.println("Usage: java Main <expression>");
            System.exit(1);
        }
        
        String input = args[0];
        
        // Tokenize with Lexer
        Lexer lexer = new Lexer(input);
        Lexer.Token token;
        
        System.out.println("Tokens:");
        while ((token = lexer.nextToken()).type != Lexer.TOKEN_EOF) {
            System.out.printf("  %s: \"%s\"\n", 
                Lexer.tokenName(token.type), token.text);
        }
        
        // Parse
        int result = Parser.parse(input);
        System.out.println("Result: " + result);
    }
}
```

## Compilation and Execution

```bash
# Compile all files
javac Lexer.java Parser.java Main.java

# Run
java Main "3 + 4 * 2"
```

## Semantic Values

The lexer's `getValue()` returns an `Object`. Cast as needed:

```java
public int nextToken() {
    // In NUMBER rule:
    this.value = Integer.parseInt(this.text);
    return NUMBER;
}
```

In parser actions, values are accessed through the semantic stack.

## Error Handling

```java
public class ParseException extends Exception {
    private int line;
    private int column;
    private String token;
    
    public ParseException(String message, int line, int column, String token) {
        super(message);
        this.line = line;
        this.column = column;
        this.token = token;
    }
    
    // Getters...
}
```

## Reading from Files

```java
String content = new String(Files.readAllBytes(Paths.get(filename)));
Lexer lexer = new Lexer(content);
```

## Reading from InputStream

```java
BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
StringBuilder sb = new StringBuilder();
String line;
while ((line = reader.readLine()) != null) {
    sb.append(line).append("\n");
}
Lexer lexer = new Lexer(sb.toString());
```

## Package Declaration

The generated code does not include a package declaration by default. Add one manually or modify the output if needed.

## Java Version Compatibility

The generated code is compatible with Java 8 and later. No external dependencies are required.
