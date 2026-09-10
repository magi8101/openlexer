# Requirements Document

## Introduction

This requirements document specifies the implementation of a proper Flex/Bison workflow for OpenLexer. The feature enforces the traditional separation of concerns where lexer specifications (.l files) and parser specifications (.y files) are provided independently by the user, rather than auto-generating lexer specifications from parser token declarations. The system will remove auto-generation capabilities, improve workflow enforcement with warning messages, add an interface contract display panel, and enhance the Try tab to support flexible testing scenarios.

## Glossary

- **OpenLexer_System**: The complete lexer and parser generator application, including CLI and GUI components
- **GUI_Application**: The graphical user interface component of OpenLexer, providing interactive lexer and parser editing
- **CLI_Tool**: The command-line interface component of OpenLexer for batch code generation
- **Grammar**: The parsed representation of a .y (Bison-format) parser specification file
- **Lexer_Spec**: The user-provided .l (Flex-format) lexer specification file content
- **Parser_Spec**: The user-provided .y (Bison-format) parser specification file content
- **Token_Enum**: The enumeration of token types referenced in the parser specification
- **YYSTYPE**: The semantic value union type used for parser stack values
- **yylex_Contract**: The lexer function signature that the parser expects to call
- **Interface_Panel**: A dedicated GUI panel displaying the integration contract between lexer and parser
- **Try_Tab**: The GUI testing interface where users can test their lexer and parser with input strings
- **Auto_Generation**: The deprecated automatic creation of lexer specifications from parser token declarations

## Requirements

### Requirement 1: Remove Auto-Lexer Generation

**User Story:** As a developer using OpenLexer, I want the system to stop automatically generating lexer specifications from parser tokens, so that I maintain full control over my lexer implementation following traditional Flex/Bison workflow practices.

#### Acceptance Criteria

1. THE Grammar SHALL NOT provide a generate_lexer_spec() method
2. THE CLI_Tool SHALL NOT automatically create lexer specification files when only a parser specification is provided
3. THE GUI_Application SHALL NOT automatically generate lexer code when processing a parser specification
4. WHEN a parser specification is processed, THE OpenLexer_System SHALL NOT store or display auto-generated lexer specifications
5. THE OpenLexer_System SHALL NOT write lexer_spec.l files to the output directory during parser-only code generation

### Requirement 2: Workflow Enforcement with Warnings

**User Story:** As a user learning proper compiler construction workflow, I want to receive helpful warning messages when I attempt operations that require both lexer and parser specifications, so that I understand the dependencies between components without being blocked from exploring.

#### Acceptance Criteria

1. WHEN a user attempts to generate parser code without providing a lexer specification, THE OpenLexer_System SHALL display a warning message stating "Warning: Parser generation without lexer specification. You will need to provide a compatible lexer implementation."
2. WHEN the warning is displayed, THE OpenLexer_System SHALL continue with parser code generation
3. THE GUI_Application SHALL display workflow warnings in a visible notification area with warning-level styling
4. THE CLI_Tool SHALL print workflow warnings to standard error with a "Warning:" prefix
5. THE OpenLexer_System SHALL NOT block or prevent code generation when workflow warnings are triggered

### Requirement 3: Interface Contract Display Panel

**User Story:** As a developer integrating a lexer with a parser, I want to see the exact interface contract that my lexer must implement, so that I can ensure compatibility between my independently-developed lexer and parser components.

#### Acceptance Criteria

1. THE GUI_Application SHALL provide an Interface_Panel that displays parser integration requirements
2. WHEN a Parser_Spec is successfully parsed, THE Interface_Panel SHALL display the Token_Enum with all token names and their numeric values
3. WHEN a Parser_Spec is successfully parsed, THE Interface_Panel SHALL display the YYSTYPE definition including all union fields and their types
4. WHEN a Parser_Spec is successfully parsed, THE Interface_Panel SHALL display the yylex_Contract function signature showing the expected return type and parameters
5. THE Interface_Panel SHALL be accessible from the GUI_Application main interface without requiring code generation
6. THE Interface_Panel SHALL update its display whenever the Parser_Spec content changes and is successfully re-parsed
7. WHEN a Parser_Spec contains syntax errors, THE Interface_Panel SHALL display an error message stating "Cannot display interface: parser specification contains errors"
8. THE Interface_Panel SHALL present information in the target language format corresponding to the currently selected output language

### Requirement 4: Flexible Try Tab Testing

**User Story:** As a developer testing my lexer and parser independently, I want the Try tab to allow me to test my lexer alone, my parser alone, or both together, so that I can debug components in isolation or verify their integration.

#### Acceptance Criteria

1. WHEN only a Lexer_Spec is provided, THE Try_Tab SHALL execute lexical analysis on the test input and display token output
2. WHEN only a Parser_Spec is provided, THE Try_Tab SHALL display a message stating "Parser testing requires lexer specification. Please provide a lexer or test with manual token input."
3. WHEN both Lexer_Spec and Parser_Spec are provided, THE Try_Tab SHALL execute full lexical and syntactic analysis on the test input
4. THE Try_Tab SHALL display lexer output showing token sequence with token names and values
5. THE Try_Tab SHALL display parser output showing parse tree structure or semantic action results
6. WHEN lexical analysis fails, THE Try_Tab SHALL display the lexer error message and SHALL NOT attempt parsing
7. WHEN parsing fails, THE Try_Tab SHALL display the parser error message including the problematic token and parser state
8. THE Try_Tab SHALL provide a clear visual separation between lexer output and parser output
9. THE Try_Tab SHALL allow users to clear test input and output with a reset button

### Requirement 5: Code Organization Preservation

**User Story:** As a developer generating production compiler code, I want the system to maintain proper separation between lexer and parser output files, so that I can integrate them into my build system following standard practices.

#### Acceptance Criteria

1. WHEN generating code for C language, THE OpenLexer_System SHALL produce lexer.c and parser.c as separate files
2. WHEN generating code for Java language, THE OpenLexer_System SHALL produce Lexer.java and Parser.java as separate files
3. WHEN generating code for Python language, THE OpenLexer_System SHALL produce lexer.py and parser.py as separate files
4. THE OpenLexer_System SHALL NOT embed lexer implementation code inside parser output files when both specifications are provided
5. THE OpenLexer_System SHALL include appropriate import or include statements in parser files to reference the external lexer module
6. WHEN only a parser specification is provided, THE OpenLexer_System SHALL generate parser code with placeholder comments indicating where lexer integration is required

### Requirement 6: Documentation Updates

**User Story:** As a new user learning OpenLexer, I want the documentation to accurately reflect the proper workflow requiring separate lexer and parser specifications, so that I understand how to use the tool correctly from the start.

#### Acceptance Criteria

1. THE OpenLexer_System SHALL provide documentation stating that lexer and parser specifications must be created separately
2. THE OpenLexer_System SHALL provide examples showing independent creation of .l and .y files
3. THE OpenLexer_System SHALL document the interface contract components: Token_Enum, YYSTYPE, and yylex_Contract
4. THE OpenLexer_System SHALL explain the workflow sequence: create lexer specification, create parser specification, verify interface contract, generate code
5. THE OpenLexer_System SHALL document how to use the Interface_Panel to verify component compatibility
6. THE OpenLexer_System SHALL document all Try_Tab testing modes: lexer-only, parser-only, and integrated testing
7. THE OpenLexer_System SHALL update README.md to remove references to automatic lexer generation

### Requirement 7: Backward Compatibility Considerations

**User Story:** As an existing OpenLexer user, I want to understand what changes affect my current workflow, so that I can migrate my projects to the new approach smoothly.

#### Acceptance Criteria

1. THE OpenLexer_System SHALL provide a migration guide document explaining the removal of auto-generation features
2. THE OpenLexer_System SHALL document how to convert projects that relied on auto-generated lexer specifications
3. WHEN processing projects created with older OpenLexer versions, THE OpenLexer_System SHALL display a warning if lexer_spec.l files are found in output directories
4. THE OpenLexer_System SHALL provide example lexer specifications corresponding to common parser token declarations
5. THE OpenLexer_System SHALL document the benefits of the new workflow approach in terms of control and flexibility
