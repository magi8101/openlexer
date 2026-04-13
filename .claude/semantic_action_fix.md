# Parser Code Generation Issues & Solutions

## Research Findings

### 1. Error Recovery Mechanisms (From POSIX yacc spec)
**Current Issues:**
- `yyerrok` is output as-is → syntax error in Python/Java
- `yyerror()` calls not translated 
- `YYERROR` macro not recognized

**Solutions:**
- **Python**: `yyerrok` → no-op (parser state handled differently)
- **Java**: `yyerrok` → reset `yyerrstatus` field
- **C**: Keep as-is (native macro)
- Translate `yyerror("msg")` → language-specific print/log

### 2. Multi-line C Code Block Parsing
**Current Issues:**
- Any action with `{` or `}` is replaced with `pass`
- No proper brace matching
- Comments & strings confuse parser

**Solution:**
Implement proper C tokenizer to:
1. Skip over string literals ("..." and '...')
2. Skip over comments (/* */ and //)
3. Count braces with depth
4. Extract complete block

### 3. Printf → Print Conversion
**Current Issues:**
- `printf("= %g\n", result)` → `print("= {}\n".format(result))`
- `\n` becomes literal, not actual newline
- Escape sequences not handled

**Format Specifiers (Standard):**
```
%d, %i → integer
%g, %f, %e → float/double
%s → string
%x, %o → hex/octal
%c → char
%% → literal %
```

**Python Conversion:**
```c
printf("%d %g\n", x, y)  
→ print("{}  {}{}".format(x, y))  OR
→ print(f"{x} {y}")
```

**Java Conversion:**
```c
printf("%d %g\n", x, y)
→ System.out.printf("%d %g%n", x, y)  // Direct
```

### 4. Semantic Value Stack Handling
**Pattern from Bison:**
- Stack stores: `[state, semantic_value, location]`
- `$$` = result of reduction (top of value stack)
- `$1`, `$2`, etc = operands (stack offset by RHS length)
- `@$` = location of result
- `@1`, `@2`, etc = operand locations

**Multi-Language Implementation:**
- **C**: Direct array/struct access `yystack[yytop - offset]`
- **Java**: Stack/ArrayList with peek/pop
- **Python**: List with negative indexing `stack[-n]`

### 5. Union Type Mapping
**Current Issues:**
- Type tags for `$<type>N` syntax not properly resolved
- Cross-language type conversion incomplete

**Required Mapping:**
```
C Type              → Java Type      → Python Type
int                  → int            → int
double               → double         → float
char*                → String         → str
struct foo*          → Foo            → object
void*                → Object         → object
```

## Implementation Priorities

### High Priority (Blocking)
1. **Parse multi-line C blocks properly**
   - Implement C-aware tokenizer
   - Handle nested structures
   - Extract full action code

2. **Translate error recovery**
   - Map yyerrok/yyclearin to each language
   - Handle yyerror() calls

3. **Fix printf conversion**
   - Proper format specifier handling
   - Escape sequence preservation
   - Language-specific output functions

### Medium Priority  
4. **Improve type substitution**
   - Handle `$<type>$` and `$<type>N` syntax
   - Map types correctly across languages
   - Track type context

5. **Semantic value references**
   - Consistent access patterns
   - Proper offset calculation
   - Location tracking (@$ syntax)

### Lower Priority
6. **Performance optimization**
   - Cache parsed action blocks
   - Pre-compute type mappings
   - Minimize runtime lookups

## Technical Approach

### Phase 1: C Code Tokenizer
Create `struct Token` parser that handles:
- String literals (escape sequences)
- Comments (single & multi-line)
- Braces (nesting depth)
- Operators (handle $ grammar constructs)

### Phase 2: Parser-Aware Substitution
Parse action as token stream:
1. Identify `$` positions
2. Match to numeric/name references
3. Look up types from grammar
4. Generate language-specific code

### Phase 3: Language-Specific Codegen
For each language backend:
- `substitute_vars_<lang>()`
- `translate_printf_<lang>()`
- `translate_error_recovery_<lang>()`
- `translate_function_call_<lang>()` (pow, fmod, etc.)

### Phase 4: Testing
Create test grammars:
- Simple arithmetic (passing)
- With error recovery (failing)
- Multi-line actions (failing)  
- Complex type substitution (failing)

## Files to Modify

1. **src/parsegen/codegen.rs** - main generation logic
2. **NEW: src/parsegen/semantic_action.rs** - action parsing & translation
3. **NEW: src/parsegen/c_tokenizer.rs** - C code aware tokenization

## References
- POSIX yacc specification (error recovery)
- GNU Bison manual (semantic actions)
- Java/Python/C documentation (printf equivalents)
