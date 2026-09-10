# Implementation Plan: Proper Flex/Bison Workflow

## Overview

This implementation enforces proper Flex/Bison workflow in OpenLexer by removing automatic lexer generation, adding interface contract display, enhancing Try tab testing modes, and implementing workflow warnings. The changes prioritize breaking changes first (removal of auto-generation) before adding enhancements (interface panel and testing improvements).

## Tasks

### Phase 1: Remove Auto-Generation (Breaking Changes)

- [ ] 1. Remove Grammar::generate_lexer_spec() method and update call sites
  - [x] 1.1 Delete generate_lexer_spec() method from Grammar implementation
    - Remove the ~200 line method from `src/parsegen/grammar.rs`
    - Preserve `token_literals: HashMap<String, String>` field for other uses
    - Update any public token access methods if needed by interface panel
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_
  
  - [x] 1.2 Remove auto-generation call site in CLI (src/main.rs)
    - Locate the `GenParser` command handler around line 210
    - Remove lexer_spec.l file writing logic
    - Remove the call to `grammar.generate_lexer_spec()`
    - Add warning emission to stderr: "Warning: Parser generation without lexer specification. You will need to provide a compatible lexer implementation."
    - _Requirements: 1.1, 1.2, 2.1, 2.2, 2.4_
  
  - [x] 1.3 Remove auto-generation call site in GUI (src/gui.rs)
    - Locate `generate_parser()` method around line 580
    - Remove `auto_lexer_spec` field from `OpenLexerApp` struct
    - Remove the call to `grammar.generate_lexer_spec()`
    - Remove the auto-lexer code generation logic
    - Add warning log entry with LogLevel::Warning
    - _Requirements: 1.1, 1.3, 2.1, 2.3, 2.5_
  
  - [ ]* 1.4 Write integration tests for auto-generation removal
    - Test that CLI no longer creates lexer_spec.l files
    - Test that warnings appear in both CLI stderr and GUI logs
    - Verify parser code generation still works without lexer
    - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2_

- [ ] 2. Checkpoint - Verify breaking changes complete
  - Ensure all tests pass, ask the user if questions arise.

### Phase 2: Interface Contract Display Panel

- [ ] 3. Create InterfaceContract extraction module
  - [ ] 3.1 Create new file src/parsegen/interface_contract.rs
    - Define `InterfaceContract` struct with tokens, yystype, yylex_signature, target_language fields
    - Define `TokenInfo` struct with name and numeric_value fields
    - Define `YYSTypeInfo` struct with union_fields and raw_body fields
    - Define `UnionFieldInfo` struct with type_name and field_name
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.8_
  
  - [ ] 3.2 Implement InterfaceContract::from_grammar() method
    - Extract token enumeration from Grammar with numeric values (idx + 256)
    - Extract YYSTYPE union fields if grammar.has_union() returns true
    - Generate language-specific yylex signature based on target language
    - Return Result<InterfaceContract> with proper error handling
    - _Requirements: 3.2, 3.3, 3.4, 3.8_
  
  - [ ] 3.3 Implement InterfaceContract helper methods
    - Implement `format_for_display()` method for formatted output
    - Implement `get_token()` method for token lookup by name
    - Implement `uses_typed_values()` method to check for union types
    - Add module to lib.rs exports
    - _Requirements: 3.1, 3.5_
  
  - [ ]* 3.4 Write unit tests for InterfaceContract extraction
    - Test token extraction with various grammar inputs
    - Test YYSTYPE extraction with union and non-union grammars
    - Test yylex signature generation for C, Java, Python
    - Test error handling for malformed grammars
    - _Requirements: 3.2, 3.3, 3.4, 3.8_

- [ ] 4. Integrate Interface Panel into GUI
  - [ ] 4.1 Add ParserSubTab::Interface enum variant
    - Update `ParserSubTab` enum in src/gui.rs to include Interface variant
    - Update tab button rendering to display "Interface" tab
    - Set up enum ordering and default behavior
    - _Requirements: 3.5, 3.8_
  
  - [ ] 4.2 Add interface contract state fields to OpenLexerApp
    - Add `cached_interface_contract: Option<InterfaceContract>` field
    - Add `interface_error: Option<String>` field
    - Initialize fields in `OpenLexerApp::new()` method
    - _Requirements: 3.1, 3.6, 3.7_
  
  - [ ] 4.3 Hook interface contract extraction into generate_parser()
    - After successful grammar parsing, call `InterfaceContract::from_grammar()`
    - Cache result in `cached_interface_contract` field
    - Handle errors and store in `interface_error` field
    - Clear interface_error on success, clear cached_contract on grammar error
    - _Requirements: 3.6, 3.7, 3.8_
  
  - [ ] 4.4 Implement render_interface_panel() method
    - Create method that takes `&mut egui::Ui` parameter
    - Display error message with red styling if interface_error is Some
    - Display "Generate a parser to view the interface contract" if no contract cached
    - Render Token Enumeration section with scrollable list (max height 200px)
    - Render YYSTYPE Definition section with union fields or "typedef int YYSTYPE" default
    - Render yylex Function Signature section with language-specific signature
    - Use egui::RichText::new().strong() for section headers
    - _Requirements: 3.2, 3.3, 3.4, 3.5, 3.7, 3.8_
  
  - [ ] 4.5 Wire interface panel rendering into parser tab
    - Add ParserSubTab::Interface match arm in main UI rendering
    - Call `render_interface_panel()` when Interface tab is selected
    - Ensure proper layout with spacing and grouping
    - Test tab switching and contract display updates
    - _Requirements: 3.5, 3.6, 3.8_
  
  - [ ]* 4.6 Write GUI integration tests for interface panel
    - Test interface panel displays correctly after parser generation
    - Test error message display when grammar has syntax errors
    - Test interface updates when grammar changes
    - Test display for all three target languages (C, Java, Python)
    - _Requirements: 3.6, 3.7, 3.8_

- [ ] 5. Checkpoint - Verify interface panel working
  - Ensure all tests pass, ask the user if questions arise.

### Phase 3: Try Tab Testing Enhancements

- [ ] 6. Refactor Try tab execution into mode-specific methods
  - [ ] 6.1 Implement run_try_lexer_only() method
    - Check if lexer_output is empty, log warning and return if so
    - Execute lexer on run_input test string
    - Format token output with token names and values
    - Store result in run_output field
    - _Requirements: 4.1, 4.4, 4.9_
  
  - [ ] 6.2 Implement run_try_parser_only() method
    - Check if parser_output is empty, log warning and return if so
    - Display informative message: "Parser testing requires lexer specification. Please provide a lexer or test with manual token input."
    - Store message in run_output field
    - _Requirements: 4.2, 4.9_
  
  - [ ] 6.3 Implement run_try_both() method
    - Check both lexer_output and parser_output are non-empty
    - Execute lexer first on run_input
    - Handle lexer failure: display error, do not attempt parsing (Requirement 4.6)
    - If lexer succeeds, execute parser with token stream
    - Handle parser failure: extract problematic token and parser state, display detailed error (Requirement 4.7)
    - Format combined output with separate lexer and parser sections
    - _Requirements: 4.3, 4.4, 4.5, 4.6, 4.7, 4.8_
  
  - [ ] 6.4 Update main Try tab run button handler
    - Replace existing `run_try_code()` with mode dispatch logic
    - Call appropriate method based on `try_include` enum value
    - Remove old auto-generated lexer fallback logic
    - _Requirements: 4.1, 4.2, 4.3_

- [ ] 7. Enhance Try tab output display
  - [ ] 7.1 Implement format_combined_output() helper method
    - Create method that takes lexer_output and parser_output strings
    - Format with clear section headers: "=== LEXER OUTPUT ===" and "=== PARSER OUTPUT ==="
    - Add spacing between sections
    - Return formatted string
    - _Requirements: 4.4, 4.5, 4.8_
  
  - [ ] 7.2 Implement render_try_output() method with visual separation
    - Check if try_include is Both mode
    - If Both mode, split output into lexer and parser sections
    - Render lexer section in ui.group() with strong text header "Lexer Output"
    - Render parser section in separate ui.group() with strong text header "Parser Output"
    - If not Both mode, render output directly with ui.code()
    - Add spacing (ui.add_space(10.0)) between sections
    - _Requirements: 4.4, 4.5, 4.8_
  
  - [ ] 7.3 Implement error extraction helper methods
    - Create `extract_error_token()` method to parse token from parser error messages
    - Create `extract_error_state()` method to parse state from parser error messages
    - Handle cases where token/state info is not available
    - Return descriptive strings for display
    - _Requirements: 4.7_
  
  - [ ]* 7.4 Write integration tests for Try tab modes
    - Test lexer-only mode with valid and invalid lexer specs
    - Test parser-only mode message display
    - Test both mode with lexer success and parser success
    - Test both mode with lexer failure (verify parsing not attempted)
    - Test both mode with parser failure (verify error details displayed)
    - _Requirements: 4.1, 4.2, 4.3, 4.6, 4.7_

- [ ] 8. Checkpoint - Verify Try tab enhancements working
  - Ensure all tests pass, ask the user if questions arise.

### Phase 4: Code Organization and Placeholders

- [ ] 9. Verify and enhance code separation
  - [ ] 9.1 Audit generated parser code for proper external declarations
    - Verify C parser includes `extern int yylex(void);` declaration
    - Verify Java parser code handles separate Lexer class properly
    - Verify Python parser includes commented import suggestion
    - Check all three languages maintain proper separation
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_
  
  - [ ] 9.2 Add placeholder comments for parser-only generation
    - Create `add_lexer_integration_comments()` helper function in codegen
    - Generate language-specific placeholder comments explaining lexer integration needed
    - Insert comments after header/import section in generated parser code
    - Include reference to Interface Contract panel
    - Apply comments when parser is generated without lexer
    - _Requirements: 5.6, 6.4_
  
  - [ ] 9.3 Verify import/include statements in all target languages
    - Check C parser has proper extern declarations
    - Verify Java parser can reference Lexer class from same package
    - Add commented import statement to Python parser: "# from lexer import Lexer, TokenType"
    - Test code generation in all three languages
    - _Requirements: 5.5, 6.4_
  
  - [ ]* 9.4 Write code generation verification tests
    - Test that lexer and parser generate separate files with correct names
    - Test that parser code includes proper external declarations
    - Test that placeholder comments appear in parser-only generation
    - Test file naming conventions for C, Java, Python
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

- [ ] 10. Checkpoint - Verify code organization correct
  - Ensure all tests pass, ask the user if questions arise.

### Phase 5: Documentation Updates

- [ ] 11. Update core documentation files
  - [ ] 11.1 Update README.md to remove auto-generation references
    - Remove any mentions of automatic lexer generation from parser
    - Add section explaining separate .l and .y file workflow
    - Update quickstart examples to show both files
    - Add link to Interface Panel documentation
    - _Requirements: 6.1, 6.2, 6.7_
  
  - [ ] 11.2 Create workflow guide document
    - Create new docs/src/workflow-guide.md file
    - Document the sequence: create .l file, create .y file, verify interface, generate code
    - Explain Interface Panel usage with screenshots/examples
    - Document all Try tab testing modes with examples
    - Add troubleshooting section for common integration issues
    - _Requirements: 6.1, 6.2, 6.4, 6.6_
  
  - [ ] 11.3 Document interface contract components
    - Add section to parser documentation explaining Token_Enum
    - Add section explaining YYSTYPE union types
    - Add section explaining yylex_Contract signature
    - Show examples for C, Java, and Python
    - _Requirements: 6.3, 6.4, 6.5_
  
  - [ ] 11.4 Create migration guide for existing users
    - Create docs/src/migration-guide.md file
    - Explain removal of auto-generation and rationale
    - Provide step-by-step migration instructions
    - Include example conversions from old to new workflow
    - Document benefits of new approach (control and flexibility)
    - _Requirements: 7.1, 7.2, 7.4, 7.5_
  
  - [ ] 11.5 Update SUMMARY.md to include new documentation
    - Add workflow-guide.md to table of contents
    - Add migration-guide.md to table of contents
    - Update existing parser docs links
    - Ensure logical organization of new content
    - _Requirements: 6.1, 6.4, 7.1_

- [ ] 12. Add example files demonstrating proper workflow
  - [ ] 12.1 Create example lexer specification files
    - Add examples/basic_tokens.l demonstrating simple token patterns
    - Add examples/advanced_tokens.l demonstrating start conditions
    - Ensure examples match common parser token declarations
    - Add comments explaining each pattern
    - _Requirements: 6.2, 7.4_
  
  - [ ] 12.2 Create matching parser specification files
    - Add examples/basic_grammar.y matching basic_tokens.l
    - Add examples/advanced_grammar.y matching advanced_tokens.l
    - Demonstrate proper integration between lexer and parser
    - Add %union and %type declarations
    - _Requirements: 6.2, 7.4_
  
  - [ ] 12.3 Add integration testing examples to documentation
    - Show complete workflow from .l + .y files to generated code
    - Demonstrate using Interface Panel to verify compatibility
    - Show Try tab testing in all three modes
    - Include expected output examples
    - _Requirements: 6.2, 6.4, 6.6_

- [ ] 13. Final checkpoint and verification
  - [ ] 13.1 Run full test suite across all changes
    - Execute all unit tests
    - Execute all integration tests
    - Test GUI functionality manually in both native and WASM
    - Test CLI commands with various inputs
    - _Requirements: All_
  
  - [ ] 13.2 Verify documentation completeness
    - Check all requirements have corresponding documentation
    - Verify code examples compile and work correctly
    - Test links and cross-references
    - Review for clarity and completeness
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7_
  
  - [ ] 13.3 Final user acceptance verification
    - Verify no lexer_spec.l files are created during parser generation
    - Verify Interface Panel displays correctly for various grammars
    - Verify Try tab modes work as documented
    - Verify warning messages appear appropriately
    - Verify generated code maintains proper separation
    - _Requirements: All_

## Notes

- Tasks marked with `*` are optional test tasks and can be skipped for faster MVP
- Breaking changes (Phase 1) must be completed first before enhancements
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation at phase boundaries
- Interface Panel is purely display-based (no code execution) and WASM-compatible
- Try tab uses existing JavaScript interop for code execution in WASM
- All GUI changes must work in both native (desktop) and WASM (web browser) contexts
- Documentation updates are independent and can proceed in parallel with code changes

## Task Dependency Graph

```json
{
  "waves": [
    { "id": 0, "tasks": ["1.1"] },
    { "id": 1, "tasks": ["1.2", "1.3"] },
    { "id": 2, "tasks": ["1.4", "3.1"] },
    { "id": 3, "tasks": ["3.2", "4.1"] },
    { "id": 4, "tasks": ["3.3", "3.4", "4.2"] },
    { "id": 5, "tasks": ["4.3"] },
    { "id": 6, "tasks": ["4.4"] },
    { "id": 7, "tasks": ["4.5", "4.6", "6.1", "6.2"] },
    { "id": 8, "tasks": ["6.3"] },
    { "id": 9, "tasks": ["6.4", "7.1"] },
    { "id": 10, "tasks": ["7.2", "7.3", "7.4", "9.1"] },
    { "id": 11, "tasks": ["9.2", "9.3"] },
    { "id": 12, "tasks": ["9.4", "11.1", "12.1"] },
    { "id": 13, "tasks": ["11.2", "11.3", "12.2"] },
    { "id": 14, "tasks": ["11.4", "12.3"] },
    { "id": 15, "tasks": ["11.5", "13.1"] },
    { "id": 16, "tasks": ["13.2", "13.3"] }
  ]
}
```
