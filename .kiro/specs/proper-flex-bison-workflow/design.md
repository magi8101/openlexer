# Design Document

## Introduction

This design document specifies the architecture and implementation approach for enforcing proper Flex/Bison workflow in OpenLexer. The changes remove automatic lexer generation capabilities, introduce workflow guidance through warnings, add an interface contract display panel in the GUI, enhance the Try tab for flexible testing, and maintain proper code separation in output files.

The design follows WASM-compatible patterns for the GUI (egui framework) and ensures clean separation between lexer and parser generation pipelines.

## System Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      OpenLexer System                        │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐                    ┌──────────────┐       │
│  │   CLI Tool   │                    │  GUI (egui)  │       │
│  └──────┬───────┘                    └──────┬───────┘       │
│         │                                    │               │
│         │         ┌──────────────────────────┤               │
│         │         │                          │               │
│         ▼         ▼                          ▼               │
│    ┌────────────────────┐         ┌──────────────────┐      │
│    │  Parser Pipeline   │         │  Lexer Pipeline  │      │
│    │                    │         │                  │      │
│    │ - Grammar Parser   │         │ - Spec Parser    │      │
│    │ - Table Builder    │         │ - NFA Builder    │      │
│    │ - Code Generator   │         │ - DFA Builder    │      │
│    └────────────────────┘         │ - Code Generator │      │
│                                    └──────────────────┘      │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

### Component Overview


1. **Grammar Module** (`src/parsegen/grammar.rs`)
   - Parses Bison-format grammar files
   - Currently contains `generate_lexer_spec()` method (to be removed)
   - Stores token declarations, union types, rules, and precedence

2. **Parser Code Generator** (`src/parsegen/codegen.rs`)
   - Generates parser code for C, Java, Python
   - Currently auto-generates lexer alongside parser (to be modified)
   - Produces table-driven LALR/GLR parsers

3. **Lexer Module** (`src/lexgen/`)
   - Parses Flex-format lexer specifications
   - Builds NFA and DFA from patterns
   - Generates lexer code independently

4. **GUI Application** (`src/gui.rs`)
   - egui-based interface (~3000 lines)
   - Tabbed interface: Lexer, Parser, Try, Help
   - Currently displays auto-generated lexer spec (to be replaced with Interface Panel)

5. **CLI Tool** (`src/main.rs`)
   - Provides `gen-lexer` and `gen-parser` subcommands
   - Currently writes `lexer_spec.l` file during parser generation (to be removed)

## Detailed Design

### 1. Remove Auto-Lexer Generation

#### 1.1 Grammar Struct Modification


**Current State:**
```rust
impl Grammar {
    pub fn generate_lexer_spec(&self) -> String {
        // ~200 lines of lexer generation logic
        // Maps tokens to patterns, handles literals, etc.
    }
}
```

**Target State:**
```rust
impl Grammar {
    // Method removed entirely
    // Token literals and token_literals HashMap preserved for other uses
}
```

**Implementation Steps:**
1. Delete `generate_lexer_spec()` method from `Grammar` impl
2. Preserve `token_literals: HashMap<String, String>` field (used for textbook notation)
3. Update method visibility for token access methods (make public if needed by Interface Panel)

**Call Sites to Remove:**
- `src/gui.rs`: Line ~580 in `generate_parser()` method
- `src/main.rs`: Line ~210 in `GenParser` command handler

#### 1.2 CLI Modification

**Current Behavior:**
```rust
Commands::GenParser { parser, lang, output } => {
    // ... generate parser code ...
    
    // Generate simple lexer spec from token declarations

    let lexer_spec = grammar.generate_lexer_spec();
    let lexer_spec_path = output.join("lexer_spec.l");
    fs::write(&lexer_spec_path, lexer_spec)?;
    println!("Generated lexer spec: {}", lexer_spec_path.display());
}
```

**Target Behavior:**
```rust
Commands::GenParser { parser, lang, output } => {
    // ... generate parser code ...
    
    // NO lexer spec generation
    // Add warning to stderr
    eprintln!("Warning: Parser generation without lexer specification. You will need to provide a compatible lexer implementation.");
}
```

#### 1.3 GUI Modification

**Current State:**
```rust
fn generate_parser(&mut self) {
    // ...
    match parsegen::codegen::generate_parser(&table, &grammar, target) {
        Ok(code) => {
            self.parser_output = code;
            self.auto_lexer_spec = grammar.generate_lexer_spec();
            
            // Also auto-generate lexer code from the spec
            match lexgen::parse_lexer_spec(&self.auto_lexer_spec) {
                // ... populate self.lexer_output ...
            }
        }
    }
}
```

**Target State:**

```rust
fn generate_parser(&mut self) {
    // ...
    match parsegen::codegen::generate_parser(&table, &grammar, target) {
        Ok(code) => {
            self.parser_output = code;
            // Remove all auto-generation logic
            // Add warning to logs
            self.log(LogLevel::Warning, 
                "Parser generation without lexer specification. You will need to provide a compatible lexer implementation.");
        }
    }
}
```

**Field Removal:**
- Remove `auto_lexer_spec: String` field from `OpenLexerApp` struct

### 2. Workflow Warning System

#### 2.1 Warning Structure

```rust
#[derive(Clone)]
struct WorkflowWarning {
    message: String,
    context: WarningContext,
}

enum WarningContext {
    ParserWithoutLexer,
    LexerWithoutParser,  // Future use
}

impl WorkflowWarning {
    fn parser_without_lexer() -> Self {
        Self {
            message: "Warning: Parser generation without lexer specification. \
                     You will need to provide a compatible lexer implementation.".to_string(),
            context: WarningContext::ParserWithoutLexer,
        }
    }
}
```



#### 2.2 CLI Warning Implementation

```rust
// In src/main.rs
Commands::GenParser { parser, lang, output } => {
    // ... existing generation logic ...
    
    // Emit warning to stderr
    eprintln!("Warning: Parser generation without lexer specification. You will need to provide a compatible lexer implementation.");
    
    // Continue with generation (non-blocking)
}
```

#### 2.3 GUI Warning Implementation

**Approach:** Use existing logging system with visual notification

```rust
// In generate_parser() method
self.log(LogLevel::Warning, 
    "Parser generation without lexer specification. You will need to provide a compatible lexer implementation.");

// LogLevel::Warning already has warning-level styling:
impl LogLevel {
    fn color(&self) -> egui::Color32 {
        match self {
            LogLevel::Warning => egui::Color32::from_rgb(255, 200, 50),
            // ...
        }
    }
}
```

**UI Display:** Warnings appear in the existing log panel with yellow/orange styling.

### 3. Interface Contract Display Panel

#### 3.1 Data Structure

```rust
#[derive(Clone, Debug)]
struct InterfaceContract {
    tokens: Vec<TokenInfo>,
    yystype: Option<YYSTypeInfo>,
    yylex_signature: String,
    target_language: TargetLanguage,
}

#[derive(Clone, Debug)]
struct TokenInfo {
    name: String,
    numeric_value: i32,
}

#[derive(Clone, Debug)]
struct YYSTypeInfo {
    union_fields: Vec<UnionFieldInfo>,
    raw_body: Option<String>,
}

#[derive(Clone, Debug)]

struct UnionFieldInfo {
    type_name: String,
    field_name: String,
}

impl InterfaceContract {
    fn from_grammar(grammar: &Grammar, lang: TargetLanguage) -> Result<Self> {
        // Extract token enumeration
        let tokens = grammar.tokens.iter().enumerate()
            .map(|(idx, name)| TokenInfo {
                name: name.clone(),
                numeric_value: (idx + 256) as i32,  // Token offset
            })
            .collect();
        
        // Extract YYSTYPE information
        let yystype = if grammar.has_union() {
            Some(YYSTypeInfo {
                union_fields: grammar.union_fields.iter()
                    .map(|f| UnionFieldInfo {
                        type_name: f.c_type.clone(),
                        field_name: f.name.clone(),
                    })
                    .collect(),
                raw_body: grammar.raw_union_body.clone(),
            })
        } else {
            None
        };
        
        // Generate yylex signature based on target language
        let yylex_signature = Self::generate_yylex_signature(lang);
        
        Ok(Self {
            tokens,
            yystype,
            yylex_signature,
            target_language: lang,
        })
    }
    
    fn generate_yylex_signature(lang: TargetLanguage) -> String {
        match lang {
            TargetLanguage::C => "int yylex(void)".to_string(),
            TargetLanguage::Java => "int yylex()".to_string(),
            TargetLanguage::Python => "def yylex() -> int:".to_string(),
        }
    }
}
```

#### 3.2 GUI Integration



**Add Sub-Tab to Parser Tab:**

```rust
#[derive(Default, PartialEq, Clone, Copy)]
enum ParserSubTab {
    #[default]
    Code,
    LalrTable,
    Glr,
    Debug,
    Tree,
    Interface,  // NEW
}
```

**App State Addition:**

```rust
struct OpenLexerApp {
    // ... existing fields ...
    
    // NEW: Interface contract caching
    cached_interface_contract: Option<InterfaceContract>,
    interface_error: Option<String>,
}
```

**Generation Hook:**

```rust
fn generate_parser(&mut self) {
    // ... existing generation logic ...
    
    match parsegen::parse_grammar(&self.parser_input) {
        Ok(grammar) => {
            // ... build table and generate code ...
            
            // NEW: Build interface contract
            match InterfaceContract::from_grammar(&grammar, target) {
                Ok(contract) => {
                    self.cached_interface_contract = Some(contract);
                    self.interface_error = None;
                }
                Err(e) => {
                    self.interface_error = Some(format!("Failed to extract interface: {}", e));
                }
            }
        }
        Err(e) => {
            self.interface_error = Some("Cannot display interface: parser specification contains errors".to_string());
        }
    }
}
```

**Rendering Logic:**

```rust
fn render_interface_panel(&mut self, ui: &mut egui::Ui) {
    if let Some(ref error) = self.interface_error {

        ui.colored_label(egui::Color32::from_rgb(255, 100, 100), error);
        return;
    }
    
    if let Some(ref contract) = self.cached_interface_contract {
        ui.heading("Lexer-Parser Interface Contract");
        ui.add_space(10.0);
        
        // Token Enum Section
        ui.group(|ui| {
            ui.label(egui::RichText::new("Token Enumeration").strong());
            ui.separator();
            
            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                for token in &contract.tokens {
                    ui.horizontal(|ui| {
                        ui.label(format!("{:<20}", token.name));
                        ui.label(format!("= {}", token.numeric_value));
                    });
                }
            });
        });
        
        ui.add_space(10.0);
        
        // YYSTYPE Section
        if let Some(ref yystype) = contract.yystype {
            ui.group(|ui| {
                ui.label(egui::RichText::new("YYSTYPE Definition").strong());
                ui.separator();
                
                if let Some(ref raw) = yystype.raw_body {
                    ui.label(format!("typedef union YYSTYPE {{\n{}\n}} YYSTYPE;", raw));
                } else {
                    for field in &yystype.union_fields {
                        ui.label(format!("  {} {};", field.type_name, field.field_name));
                    }
                }
            });
        } else {
            ui.group(|ui| {
                ui.label(egui::RichText::new("YYSTYPE Definition").strong());
                ui.separator();
                ui.label("typedef int YYSTYPE;  /* Default */");
            });
        }
        
        ui.add_space(10.0);
        
        // yylex Signature
        ui.group(|ui| {

            ui.label(egui::RichText::new("yylex() Function Signature").strong());
            ui.separator();
            ui.label(&contract.yylex_signature);
            ui.label("/* Lexer must set global yylval before returning token type */");
        });
    } else {
        ui.label("Generate a parser to view the interface contract.");
    }
}
```

#### 3.3 Reactivity

**Update Trigger:** Interface panel updates whenever `cached_grammar` is updated, which happens:
- On successful grammar parsing in `generate_parser()`
- When parser input changes and is re-parsed (if auto-parse is implemented)

**Manual Refresh:** Users can trigger refresh by clicking "Generate" button in Parser tab.

### 4. Try Tab Enhancements

#### 4.1 Current Try Tab Structure

```rust
#[derive(Default, PartialEq, Clone, Copy)]
enum TryInclude {
    #[default]
    LexerOnly,
    ParserOnly,
    Both,
}
```

**Current Behavior:**
- `LexerOnly`: Uses explicit lexer, falls back to auto-generated if missing
- `ParserOnly`: Not properly implemented
- `Both`: Combines lexer and parser

#### 4.2 Target Behavior

**LexerOnly Mode:**
```rust
fn run_try_lexer_only(&mut self) {
    if self.lexer_output.is_empty() {
        self.log(LogLevel::Warning, 
            "No lexer generated. Go to Lexer tab and click Generate first.");
        return;
    }
    
    // Execute lexer on try_code input
    let tokens = self.execute_lexer(&self.lexer_output, &self.run_input);
    self.run_output = self.format_token_output(tokens);
}
```



**ParserOnly Mode:**
```rust
fn run_try_parser_only(&mut self) {
    if self.parser_output.is_empty() {
        self.log(LogLevel::Warning, 
            "No parser generated. Go to Parser tab and click Generate first.");
        return;
    }
    
    // Display message - cannot test parser without lexer
    self.run_output = "Parser testing requires lexer specification. \
                       Please provide a lexer or test with manual token input.".to_string();
}
```

**Both Mode:**
```rust
fn run_try_both(&mut self) {
    if self.lexer_output.is_empty() {
        self.log(LogLevel::Warning, 
            "No lexer generated. Go to Lexer tab and Generate.");
        return;
    }
    
    if self.parser_output.is_empty() {
        self.log(LogLevel::Warning, 
            "No parser generated. Go to Parser tab and Generate.");
        return;
    }
    
    // Execute full pipeline: lex then parse
    let combined_code = self.combine_lexer_parser_code();
    self.run_generated_code(&combined_code, None);
}
```

#### 4.3 Output Display Enhancements

**Separate Sections:**

```rust
fn format_combined_output(&self, lexer_output: String, parser_output: String) -> String {
    format!(
        "=== LEXER OUTPUT ===\n{}\n\n=== PARSER OUTPUT ===\n{}\n",
        lexer_output,
        parser_output
    )
}
```

**Visual Rendering:**

```rust
fn render_try_output(&mut self, ui: &mut egui::Ui) {
    if self.try_include == TryInclude::Both {
        // Split output into sections
        if let Some((lexer_part, parser_part)) = self.split_output(&self.run_output) {
            ui.group(|ui| {
                ui.label(egui::RichText::new("Lexer Output").strong());
                ui.code(lexer_part);
            });
            
            ui.add_space(10.0);
            
            ui.group(|ui| {

                ui.label(egui::RichText::new("Parser Output").strong());
                ui.code(parser_part);
            });
        } else {
            ui.code(&self.run_output);
        }
    } else {
        ui.code(&self.run_output);
    }
}
```

#### 4.4 Error Handling

**Lexer Failure:**
```rust
fn execute_lexer(&mut self, lexer_code: &str, input: &str) -> Result<Vec<Token>, String> {
    match self.run_lexer_code(lexer_code, input) {
        Ok(tokens) => Ok(tokens),
        Err(e) => {
            self.log(LogLevel::Error, &format!("Lexer error: {}", e));
            Err(e)
        }
    }
}

// In Try Both mode:
fn run_try_both(&mut self) {
    match self.execute_lexer(&self.lexer_output, &self.run_input) {
        Ok(tokens) => {
            // Proceed to parsing
            self.execute_parser(tokens);
        }
        Err(lexer_error) => {
            // Stop here, don't attempt parsing
            self.run_output = format!("Lexer Error:\n{}\n\nParsing not attempted.", lexer_error);
        }
    }
}
```

**Parser Failure:**
```rust
fn execute_parser(&mut self, tokens: Vec<Token>) -> Result<ParseTree, String> {
    match self.run_parser_code(&self.parser_output, tokens) {
        Ok(tree) => Ok(tree),
        Err(e) => {
            let problematic_token = self.extract_error_token(&e);
            let parser_state = self.extract_error_state(&e);
            
            let error_msg = format!(
                "Parser error: {}\nProblematic token: {}\nParser state: {}",
                e, problematic_token, parser_state
            );
            
            self.log(LogLevel::Error, &error_msg);
            Err(error_msg)
        }
    }
}
```

### 5. Code Organization Preservation



#### 5.1 Current Code Generation Flow

**Parser Generation** (`src/parsegen/codegen.rs`):
- Currently includes external lexer declaration: `extern int yylex(void);`
- Does NOT embed lexer implementation
- Good separation already exists ✓

**Lexer Generation** (`src/lexgen/codegen.rs`):
- Generates standalone lexer
- Includes test driver with `main()`
- Independent from parser ✓

#### 5.2 File Naming Conventions

**Already Correct:**

| Language | Lexer Output  | Parser Output  |
|----------|---------------|----------------|
| C        | lexer.c       | parser.c       |
| Java     | Lexer.java    | Parser.java    |
| Python   | lexer.py      | parser.py      |

**CLI Implementation:**
```rust
// In src/main.rs - GenLexer command
let filename = match lang {
    Language::C => "lexer.c",
    Language::Java => "Lexer.java",
    Language::Python => "lexer.py",
};

// In src/main.rs - GenParser command
let filename = match lang {
    Language::C => "parser.c",
    Language::Java => "Parser.java",
    Language::Python => "parser.py",
};
```

**GUI Implementation:**
```rust
// In download functions
fn download_lexer(&self) {
    let filename = match self.language {
        TargetLanguage::C => "lexer.c",
        TargetLanguage::Java => "Lexer.java",
        TargetLanguage::Python => "lexer.py",
    };
    web_utils::download_file(filename, &self.lexer_output);
}

fn download_parser(&self) {
    let filename = match self.language {
        TargetLanguage::C => "parser.c",
        TargetLanguage::Java => "Parser.java",
        TargetLanguage::Python => "parser.py",
    };
    web_utils::download_file(filename, &self.parser_output);
}
```



#### 5.3 Import/Include Statements

**C Language:**
```c
/* In parser.c - already present */
extern int yylex(void);  /* External lexer function - user must provide */
extern char *yytext;
extern int yyleng;
```

**Java Language:**
```java
/* In Parser.java - need to add if missing */
// Import lexer from same package
// Note: Both classes can be in the same file for simple use
```

**Python Language:**
```python
# In parser.py - need to verify
# from lexer import Lexer  # When using separate files
# Note: Current implementation may need this added
```

**Implementation Check:**
```rust
fn generate_parser_with_imports(grammar: &Grammar, lang: TargetLanguage) -> Result<String> {
    let mut code = String::new();
    
    // Add language-specific imports at the top
    match lang {
        TargetLanguage::Python => {
            code.push_str("# To use with separate lexer:\n");
            code.push_str("# from lexer import Lexer, TokenType\n\n");
        }
        TargetLanguage::Java => {
            // No explicit import needed if in same package
        }
        TargetLanguage::C => {
            // extern declarations already present
        }
    }
    
    // ... rest of generation
    Ok(code)
}
```

#### 5.4 Parser-Only Generation Placeholders

**When no lexer is provided:**

```rust
fn add_lexer_integration_comments(code: &mut String, lang: TargetLanguage) {
    let comment = match lang {
        TargetLanguage::C => {
            "\n/* ===== LEXER INTEGRATION REQUIRED =====\n\
             * Implement or link the following functions:\n\
             *   - int yylex(void)      // Returns next token type\n\
             *   - char *yytext         // Current token text\n\
             *   - int yyleng           // Current token length\n\
             * See Interface Contract panel for token values.\n\
             * ======================================= */\n\n"
        }
        TargetLanguage::Java => {
            "\n/* ===== LEXER INTEGRATION REQUIRED =====\n\
             * Implement yylex() method or import Lexer class:\n\

             *   - int yylex()         // Returns next token type\n\
             * See Interface Contract panel for token values.\n\
             * ======================================= */\n\n"
        }
        TargetLanguage::Python => {
            "\n# ===== LEXER INTEGRATION REQUIRED =====\n\
             # Implement or import lexer:\n\
             #   from lexer import Lexer, TokenType\n\
             # See Interface Contract panel for token values.\n\
             # ========================================\n\n"
        }
    };
    
    // Insert after initial imports/includes
    code.insert_str(find_header_end(code), comment);
}
```

## Data Flow Diagrams

### Parser Generation Flow (Modified)

```
User Input (.y file)
    ↓
Grammar Parser
    ↓
Grammar AST ────────────────┐
    ↓                       ↓
LALR Table Builder    Interface Contract
    ↓                   Extractor
Code Generator              ↓
    ↓                   Token Enum
Parser Code             YYSTYPE Info
(with extern            yylex Signature
 declarations)              ↓
    ↓                   GUI Interface Panel
Output Files                (Display Only)
```

### Try Tab Execution Flow (New)

```
User selects mode:
├─ LexerOnly ─────────┐
│                     ↓
│              Lexer Code + Test Input
│                     ↓
│              Execute Lexer
│                     ↓
│              Display Tokens
│
├─ ParserOnly ────────┐
│                     ↓
│              Display Message:
│              "Requires lexer spec"
│
└─ Both ──────────────┐
                      ↓
            Lexer Code + Parser Code + Test Input
                      ↓
            Execute Lexer
                      ↓
            Success? ─────No───┐
                │              ↓
               Yes      Display Lexer Error
                │       Stop (no parsing)
                ↓
            Execute Parser
                │

                ↓
            Success? ─────No───┐
                │              ↓
               Yes      Display Parser Error
                │       (with token & state)
                ↓
            Display Results:
            [Lexer Output]
            [Parser Output]
```

## Component Interfaces

### InterfaceContract Module

```rust
// src/parsegen/interface_contract.rs (NEW FILE)

use crate::parsegen::grammar::{Grammar, UnionField};
use crate::lexgen::codegen::TargetLanguage;
use crate::error::Result;

pub struct InterfaceContract {
    pub tokens: Vec<TokenInfo>,
    pub yystype: Option<YYSTypeInfo>,
    pub yylex_signature: String,
    pub target_language: TargetLanguage,
}

pub struct TokenInfo {
    pub name: String,
    pub numeric_value: i32,
}

pub struct YYSTypeInfo {
    pub union_fields: Vec<UnionFieldInfo>,
    pub raw_body: Option<String>,
}

pub struct UnionFieldInfo {
    pub type_name: String,
    pub field_name: String,
}

impl InterfaceContract {
    /// Extract interface contract from a parsed grammar
    pub fn from_grammar(grammar: &Grammar, lang: TargetLanguage) -> Result<Self>;
    
    /// Generate formatted display string for a specific language
    pub fn format_for_display(&self) -> String;
    
    /// Get token by name
    pub fn get_token(&self, name: &str) -> Option<&TokenInfo>;
    
    /// Check if uses typed values
    pub fn uses_typed_values(&self) -> bool;
}
```

### WorkflowWarning Module

```rust
// src/workflow_warnings.rs (NEW FILE)

pub struct WorkflowWarning {
    message: String,
    context: WarningContext,
}

pub enum WarningContext {
    ParserWithoutLexer,
    LexerWithoutParser,
}

impl WorkflowWarning {

    pub fn parser_without_lexer() -> Self {
        Self {
            message: "Warning: Parser generation without lexer specification. \
                     You will need to provide a compatible lexer implementation.".to_string(),
            context: WarningContext::ParserWithoutLexer,
        }
    }
    
    pub fn emit_cli(&self) {
        eprintln!("{}", self.message);
    }
    
    pub fn to_log_entry(&self) -> LogEntry {
        // Convert to GUI log entry
    }
}
```

## Implementation Sequence

### Phase 1: Remove Auto-Generation (Breaking Changes)

1. **Remove `generate_lexer_spec()` method** from `Grammar` impl
2. **Update CLI** - remove lexer_spec.l file generation in `GenParser` command
3. **Update GUI** - remove `auto_lexer_spec` field and auto-generation logic
4. **Add warnings** - emit warning in both CLI and GUI when parser-only generation occurs
5. **Test**: Verify no lexer_spec.l files created, warnings appear

### Phase 2: Interface Panel (GUI Enhancement)

1. **Create InterfaceContract module** - new file with extraction logic
2. **Add ParserSubTab::Interface** enum variant
3. **Add state fields** to `OpenLexerApp`:
   - `cached_interface_contract: Option<InterfaceContract>`
   - `interface_error: Option<String>`
4. **Hook contract extraction** into `generate_parser()` method
5. **Implement rendering** - `render_interface_panel()` method
6. **Add tab button** in Parser tab UI
7. **Test**: Generate parser, switch to Interface tab, verify display

### Phase 3: Try Tab Enhancements

1. **Refactor try execution** - split `run_try_code()` into mode-specific methods:
   - `run_try_lexer_only()`
   - `run_try_parser_only()`
   - `run_try_both()`
2. **Implement parser-only message** - show informative message
3. **Enhance error handling** - extract token and state from parser errors
4. **Improve output formatting** - separate lexer/parser sections
5. **Update UI rendering** - add visual separation with grouped sections
6. **Test**: Test all three modes with various inputs

### Phase 4: Code Organization Verification

1. **Audit generated code** - verify no embedded lexer in parser files
2. **Add placeholder comments** for parser-only generation
3. **Verify import statements** in Python and Java generators
4. **Test**: Generate code in all languages, verify separation

### Phase 5: Documentation Updates

1. **Update README.md** - remove auto-generation references
2. **Add workflow guide** - document proper Flex/Bison workflow
3. **Document Interface Panel** - add GUI feature documentation
4. **Create migration guide** - help existing users transition
5. **Update examples** - show separate .l and .y file workflow



## WASM Compatibility Considerations

### GUI Constraints

**egui Framework:** All UI code must work in both native and WASM contexts.

**No File System Access in WASM:**
- Cannot write files directly
- Use `web_utils::download_file()` for downloads (triggers browser download)
- No filesystem verification tests in WASM

**Async Constraints:**
- Code execution in WASM uses JavaScript interop
- `startCodeRun()`, `isRunDone()`, `getRunResult()` JS functions
- Interface Panel is purely display (no execution) - WASM compatible ✓

### Testing Strategy

**Native Tests:**
- File generation and organization
- CLI workflow warnings
- Full integration tests

**WASM Tests:**
- GUI rendering and interaction
- Interface Panel display
- Try tab execution (via JS interop)

## Error Handling

### Grammar Parsing Errors

```rust
match Grammar::parse(&input) {
    Ok(grammar) => {
        // Proceed with interface extraction
        match InterfaceContract::from_grammar(&grammar, lang) {
            Ok(contract) => { /* cache and display */ }
            Err(e) => {
                // Shouldn't happen if grammar parsed successfully
                self.interface_error = Some(format!("Internal error: {}", e));
            }
        }
    }
    Err(e) => {
        // Parser error - cannot show interface
        self.interface_error = Some(
            "Cannot display interface: parser specification contains errors".to_string()
        );
        self.parser_output.clear();
    }
}
```

### Try Tab Execution Errors

**Lexer Errors:**
- Display error message
- Include line/column if available
- Do not attempt parsing

**Parser Errors:**
- Display error message
- Extract and show problematic token
- Extract and show parser state
- Show lexer output separately (successful part)

### Backward Compatibility

**Breaking Changes:**
- No more auto-generated lexer_spec.l files
- Users relying on auto-generation must provide .l files

**Migration Path:**
1. System detects old lexer_spec.l files in output directory
2. Emit warning: "Found old lexer_spec.l file from previous OpenLexer version. Auto-generation has been removed. Please create explicit .l file."
3. Provide example lexer specs in documentation



## Security Considerations

**Code Execution in Try Tab:**
- Already sandboxed via WASM in browser
- Native execution uses subprocess
- No new security concerns introduced

**Input Validation:**
- Grammar and lexer specs already validated by parsers
- No SQL injection or XSS concerns (no database, no web rendering)

## Performance Considerations

**Interface Panel:**
- Contract extraction is O(n) in tokens/union fields
- Cached once per generation
- Minimal performance impact

**Try Tab:**
- Execution time unchanged (same code execution, just better organized)
- Separate lexer/parser output requires splitting (O(n) string operation, negligible)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_interface_contract_extraction() {
        let grammar_input = r#"
            %token NUMBER PLUS
            %union {
                int ival;
                double dval;
            }
            %%
            expr: NUMBER;
            %%
        "#;
        
        let grammar = Grammar::parse(grammar_input).unwrap();
        let contract = InterfaceContract::from_grammar(&grammar, TargetLanguage::C).unwrap();
        
        assert_eq!(contract.tokens.len(), 2);
        assert!(contract.yystype.is_some());
        assert_eq!(contract.yylex_signature, "int yylex(void)");
    }
    
    #[test]
    fn test_grammar_without_generate_lexer_spec() {
        let grammar = Grammar::new();
        // This should not compile if method exists
        // grammar.generate_lexer_spec();  // Compile error expected
    }
    
    #[test]
    fn test_workflow_warning() {
        let warning = WorkflowWarning::parser_without_lexer();
        assert!(warning.message.contains("Warning"));
        assert!(warning.message.contains("lexer specification"));
    }
}
```

### Integration Tests

```rust
#[test]
fn test_cli_parser_only_no_lexer_file() {
    // Run: openlexer gen-parser --parser test.y -L c -o output/
    // Verify: output/parser.c exists
    // Verify: output/lexer_spec.l does NOT exist
    // Verify: stderr contains warning message
}

#[test]
fn test_separate_file_generation() {
    // Generate lexer and parser
    // Verify: lexer.c and parser.c are separate
    // Verify: parser.c contains "extern int yylex(void)"
    // Verify: parser.c does NOT contain lexer implementation
}
```



### GUI Tests

**Manual Testing Checklist:**
1. Generate parser without lexer → warning appears in logs
2. Switch to Interface tab → contract displays correctly
3. Change language → interface updates to show language-specific syntax
4. Try tab - LexerOnly mode → tokens display
5. Try tab - ParserOnly mode → message displays
6. Try tab - Both mode → lexer and parser output separate
7. Try tab - Both mode with lexer error → parser not attempted
8. Try tab - Both mode with parser error → error shows token and state

## Alternatives Considered

### Alternative 1: Keep Auto-Generation as Optional Feature

**Approach:** Add flag `--auto-lexer` to optionally generate lexer spec

**Pros:**
- Backward compatible
- Useful for quick prototyping

**Cons:**
- Confuses learning path (two workflows)
- Maintains code complexity
- Not consistent with Flex/Bison philosophy

**Decision:** Rejected. Clean break better for long-term maintainability.

### Alternative 2: Interface Panel as Separate Window

**Approach:** Interface contract in popup/modal instead of tab

**Pros:**
- Can view alongside code generation

**Cons:**
- More complex UI state management
- WASM popup handling complications
- Less discoverable

**Decision:** Rejected. Tab-based approach simpler and more consistent.

### Alternative 3: Try Tab with Manual Token Input Mode

**Approach:** Allow users to type token sequences manually for parser testing

**Pros:**
- Could test parser without lexer
- Educational value

**Cons:**
- Significant UI complexity (token sequence editor)
- Error-prone (wrong token values)
- Out of scope for current workflow

**Decision:** Deferred. Could be future enhancement.

## Structured Pseudocode Examples

### Interface Contract Extraction

```
FUNCTION extract_interface_contract(grammar, target_language):
    contract = new InterfaceContract
    
    // Extract tokens with numeric values
    FOR each (index, token) IN grammar.tokens:
        contract.tokens.add({
            name: token,
            numeric_value: index + 256  // Standard token offset
        })
    
    // Extract YYSTYPE information
    IF grammar.has_union():
        contract.yystype = new YYSTypeInfo
        
        IF grammar.raw_union_body exists:
            contract.yystype.raw_body = grammar.raw_union_body
        ELSE:
            FOR each field IN grammar.union_fields:
                contract.yystype.union_fields.add({
                    type_name: field.c_type,
                    field_name: field.name
                })
    ELSE:
        contract.yystype = None  // Default int type
    
    // Generate yylex signature for target language
    contract.yylex_signature = MATCH target_language:
        C      => "int yylex(void)"
        Java   => "int yylex()"
        Python => "def yylex() -> int:"
    
    contract.target_language = target_language
    RETURN contract
```



### Try Tab Mode Execution

```
FUNCTION execute_try_tab(mode, lexer_code, parser_code, test_input):
    MATCH mode:
        CASE LexerOnly:
            IF lexer_code is empty:
                RETURN error("No lexer generated")
            
            tokens = execute_lexer(lexer_code, test_input)
            RETURN format_token_display(tokens)
        
        CASE ParserOnly:
            RETURN message("Parser testing requires lexer specification")
        
        CASE Both:
            IF lexer_code is empty OR parser_code is empty:
                RETURN error("Both lexer and parser required")
            
            // Execute lexer first
            TRY:
                tokens = execute_lexer(lexer_code, test_input)
            CATCH lexer_error:
                RETURN format_error("Lexer", lexer_error)
                // Parser not attempted
            
            // Execute parser with tokens
            TRY:
                parse_result = execute_parser(parser_code, tokens)
                RETURN format_combined_output(tokens, parse_result)
            CATCH parser_error:
                token = extract_problematic_token(parser_error)
                state = extract_parser_state(parser_error)
                RETURN format_parser_error(parser_error, token, state, tokens)
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property Reflection

Before listing properties, I reviewed the prework analysis to eliminate redundancy:

**Identified Redundancies:**
1. Requirements 1.1-1.5 all test that auto-generation doesn't occur in different contexts. These can be consolidated into properties about the absence of generate_lexer_spec() method and lexer_spec.l files.

2. Requirements 2.1-2.5 all test warning behavior. These can be consolidated into properties about warning emission and non-blocking behavior.

3. Requirements 3.2-3.4 all test Interface Panel display of different contract components. These can be consolidated into a single comprehensive property about complete interface display.

4. Requirements 4.4 and 4.5 both test output display. These can be consolidated into a property about output format correctness.

5. Requirements 5.1-5.3 all test file separation for different languages. These can be consolidated into a single property parameterized by language.

**Remaining Properties:** After consolidation, we have focused properties that each provide unique validation value.



### Property 1: Interface Contract Completeness

*For any* valid parser specification with tokens and optional union types, the Interface Panel SHALL display all declared tokens with their numeric values, all union fields with their types, and the correct yylex signature for the selected target language.

**Validates: Requirements 3.2, 3.3, 3.4, 3.8**

**Test Strategy:** Generate random parser specifications with varying numbers of tokens (0-50), union fields (0-20), and target languages (C, Java, Python). Verify that the extracted InterfaceContract contains all tokens, all union fields, and language-appropriate yylex signature.

### Property 2: Interface Panel Reactivity

*For any* parser specification change that results in successful re-parsing, the Interface Panel SHALL update to reflect the new interface contract.

**Validates: Requirements 3.6**

**Test Strategy:** Generate an initial parser spec, extract its contract, then make random modifications (add tokens, change union), re-parse, and verify the contract updates to match the new specification.

### Property 3: Interface Error Display

*For any* parser specification containing syntax errors, the Interface Panel SHALL display an error message and SHALL NOT display partial or stale interface information.

**Validates: Requirements 3.7**

**Test Strategy:** Generate random syntactically invalid parser specifications (missing semicolons, unclosed braces, invalid tokens), attempt to parse, and verify the Interface Panel shows error message and no contract information.

### Property 4: Try Tab Lexer Execution

*For any* lexer specification and test input string, when Try Tab is in LexerOnly mode, the system SHALL execute lexical analysis and display the resulting token sequence with token names and values.

**Validates: Requirements 4.1, 4.4**

**Test Strategy:** Generate random lexer specifications with various patterns, generate random test inputs, execute in LexerOnly mode, and verify token output contains all expected tokens with names and values.

### Property 5: Try Tab Parser Isolation

*For any* execution in Try Tab Both mode where lexical analysis fails, the system SHALL NOT attempt parsing and SHALL display only the lexer error message.

**Validates: Requirements 4.6**

**Test Strategy:** Generate random lexer/parser pairs, generate test inputs that cause lexer failures (invalid characters, pattern mismatches), execute in Both mode, and verify parser is not invoked and only lexer error appears.

### Property 6: Try Tab Parser Error Information

*For any* execution in Try Tab Both mode where parsing fails, the error display SHALL include the problematic token and parser state information.

**Validates: Requirements 4.7**

**Test Strategy:** Generate random parser specifications, generate test inputs that cause parse failures (syntax errors, unexpected tokens), execute in Both mode, and verify error message contains token and state information.

### Property 7: File Separation by Language

*For any* valid lexer and parser specification pair and any target language (C, Java, Python), code generation SHALL produce exactly two separate output files with language-appropriate names (lexer.c/parser.c, Lexer.java/Parser.java, lexer.py/parser.py).

**Validates: Requirements 5.1, 5.2, 5.3**

**Test Strategy:** Generate random lexer/parser specification pairs, generate code for each target language, and verify exactly two files are produced with correct names for that language.

### Property 8: Parser Code Lexer Exclusion

*For any* combined lexer and parser code generation, the parser output file SHALL NOT contain lexer implementation code (pattern matching, DFA tables, or lexical analysis logic).

**Validates: Requirements 5.4**

**Test Strategy:** Generate random lexer/parser pairs across all languages, generate combined code, parse the parser output file, and verify it contains no lexer-specific keywords, data structures, or DFA transitions.

### Property 9: Parser Lexer Integration Statements

*For any* parser code generation across all target languages, the parser output SHALL include appropriate import, include, or extern declarations for referencing an external lexer module.

**Validates: Requirements 5.5**

**Test Strategy:** Generate random parser specifications for each language, generate parser code, and verify presence of language-specific integration statements (C: extern declarations, Java: import/class access, Python: import statement or comment).

### Property 10: Parser-Only Placeholder Comments

*For any* parser-only code generation (no lexer specification provided), the generated parser code SHALL include placeholder comments indicating where lexer integration is required.

**Validates: Requirements 5.6**

**Test Strategy:** Generate random parser-only specifications across all languages, generate code, and verify parser files contain placeholder comments mentioning lexer integration requirements.

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking change disrupts existing users | High | Medium | Provide clear migration guide, warning for old files |
| Interface Panel performance issues with large grammars | Low | Low | Contract extraction is O(n), caching minimizes regeneration |
| Try Tab mode confusion | Medium | Low | Clear UI labels, documentation, example workflows |
| WASM compatibility issues | Low | High | All changes use existing egui patterns, no new platform-specific code |
| Missing import statements cause compilation errors | Medium | Medium | Comprehensive testing across all languages, include examples |

## Future Enhancements

1. **Manual Token Input Mode** - Allow users to type token sequences for parser testing without lexer
2. **Interface Contract Export** - Download contract as .h header file or interface file
3. **Workflow Templates** - Provide starter .l and .y file templates
4. **Integration Testing Mode** - Automated compatibility checking between lexer and parser
5. **Contract Diffing** - Show what changed when parser spec is modified

## Conclusion

This design enforces proper Flex/Bison workflow by:
- Removing automatic lexer generation capabilities
- Providing clear workflow guidance through warnings
- Displaying the lexer-parser interface contract explicitly
- Supporting flexible testing scenarios in the Try tab
- Maintaining clean code separation in all output formats

The implementation preserves WASM compatibility, maintains performance, and provides a smooth migration path for existing users while establishing better separation of concerns and educational value.

