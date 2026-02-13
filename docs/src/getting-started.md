# Getting Started

This guide will help you install OpenLexer and generate your first lexer and parser.

## What You'll Learn

1. How to install OpenLexer
2. How to write a lexer specification (`.l` file)
3. How to write a grammar specification (`.y` file)
4. How to generate and use the output code

## Prerequisites

- Basic understanding of regular expressions
- Familiarity with context-free grammars (helpful but not required)
- A compiler for your target language (gcc, javac, or python)

## Overview

OpenLexer follows a two-stage process similar to Flex/Bison:

```
┌─────────────┐      ┌─────────────┐
│   calc.l    │──────│  Lexer      │──────▶ lexer.c/java/py
│ (patterns)  │      │  Generator  │
└─────────────┘      └─────────────┘

┌─────────────┐      ┌─────────────┐
│   calc.y    │──────│  Parser     │──────▶ parser.c/java/py
│ (grammar)   │      │  Generator  │
└─────────────┘      └─────────────┘
```

The **lexer** converts input text into tokens using regular expressions.
The **parser** recognizes the structure of those tokens using grammar rules.

## Next Steps

- [Installation](./getting-started/installation.md) - Install OpenLexer on your system
- [Quick Start](./getting-started/quickstart.md) - Build a calculator in 5 minutes
