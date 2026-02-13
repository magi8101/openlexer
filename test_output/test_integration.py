"""Integration test for OpenLexer generated lexer and parser."""

import sys
# Add paths for the generated modules
sys.path.insert(0, 'calc_lexer.py')  # Directory containing lexer.py
sys.path.insert(0, 'parser.py')      # Directory containing parser.py

from lexer import Lexer, Token, TokenType
from parser import Parser

# Create adapter lexer that returns tokens with string type names
class LexerAdapter:
    def __init__(self, input_str: str):
        self.lexer = Lexer(input_str)
    
    def next_token(self):
        tok = self.lexer.next_token()
        # Convert enum to string for parser compatibility
        return AdaptedToken(tok)

class AdaptedToken:
    def __init__(self, tok):
        self.type = tok.type.name if tok.type != TokenType.EOF else '$'
        self.text = tok.text
        self.value = int(tok.text) if tok.type == TokenType.NUMBER else tok.text
        self.pos = tok.pos

def test_lexer():
    """Test lexer output."""
    print("=== Testing Lexer ===")
    lexer = Lexer("3 + 4 * 2")
    for token in lexer.tokenize():
        print(f"  {token.type.name:10s} | {token.text!r}")

def test_parser():
    """Test parser with lexer."""
    print("\n=== Testing Parser ===")
    adapter = LexerAdapter("5\n")
    parser = Parser(adapter)
    try:
        result = parser.parse()
        print(f"Parse result: {result}")
    except Exception as e:
        print(f"Parse error: {e}")

def test_expression(expr):
    """Test parsing an expression."""
    print(f"\n=== Parsing: {expr!r} ===")
    adapter = LexerAdapter(expr + "\n")
    parser = Parser(adapter)
    try:
        result = parser.parse()
        print(f"Result: {result}")
        return result
    except Exception as e:
        print(f"Error: {e}")
        return None

if __name__ == "__main__":
    test_lexer()
    test_parser()
    
    # Test simple expressions
    test_expression("5")
    test_expression("3 + 4")
    test_expression("3 + 4 * 2")
    test_expression("(3 + 4) * 2")
