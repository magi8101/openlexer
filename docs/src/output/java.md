# Java Output

## Generated Files

- `Lexer.java` - Lexer class with DFA tables
- `Parser.java` - Parser class with LALR tables

## Lexer Interface

```java
public class Lexer {
    // Token constants
    public static final int EOF = 0;
    public static final int NUMBER = 1;
    public static final int PLUS = 2;
    // ...
    
    // Constructor
    public Lexer(String input);
    
    // Get next token
    public int nextToken();
    
    // Get matched text
    public String getText();
    
    // Get semantic value
    public Object getValue();
}
```

## Parser Interface

```java
public class Parser {
    // Constructor
    public Parser(Lexer lexer);
    
    // Parse input, returns result
    public Object parse() throws ParseException;
}
```

## Integration Example

```java
import java.io.*;

public class Main {
    public static void main(String[] args) {
        if (args.length < 1) {
            System.err.println("Usage: java Main <expression>");
            System.exit(1);
        }
        
        try {
            Lexer lexer = new Lexer(args[0]);
            Parser parser = new Parser(lexer);
            Object result = parser.parse();
            System.out.println("Result: " + result);
        } catch (ParseException e) {
            System.err.println("Parse error: " + e.getMessage());
            System.exit(1);
        }
    }
}
```

## Compilation and Execution

```bash
javac Lexer.java Parser.java Main.java
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
