/* ============================================================================
 * OpenLexer Feature Showcase Lexer
 * 
 * Matching lexer for showcase.y grammar.
 * Demonstrates:
 * - Multiple token types (integers, floats, strings, identifiers)
 * - Multi-character operators
 * - Whitespace handling
 * - Comment support (// and /* */)
 * - String literal parsing
 * - Unicode character classes (\p{...})
 * - Unicode escapes (\u{XXXX})
 * - Script-based matching (\p{Script=Greek})
 * ============================================================================ */

/* ============================================================================
 * Definitions Section
 * ============================================================================ */

DIGIT       [0-9]
LETTER      [a-zA-Z_]
HEXDIGIT    [0-9a-fA-F]
WHITESPACE  [ \t\r]+

/* Unicode character classes */
UNICODE_LETTER    \p{L}
UNICODE_UPPER     \p{Lu}
UNICODE_LOWER     \p{Ll}
UNICODE_DIGIT     \p{Nd}
UNICODE_SYMBOL    \p{S}
UNICODE_PUNCT     \p{P}

/* Script-specific patterns */
GREEK_LETTER      \p{Script=Greek}
CYRILLIC_LETTER   \p{Script=Cyrillic}
ARABIC_LETTER     \p{Script=Arabic}
HEBREW_LETTER     \p{Script=Hebrew}
HAN_CHAR          \p{Script=Han}
HIRAGANA          \p{Script=Hiragana}
KATAKANA          \p{Script=Katakana}
THAI_CHAR         \p{Script=Thai}
DEVANAGARI        \p{Script=Devanagari}

/* Unicode categories */
EMOJI             \p{So}
MATH_SYMBOL       \p{Sm}
CURRENCY          \p{Sc}

%%

/* ============================================================================
 * Rules Section
 * ============================================================================ */

/* === Whitespace (including Unicode spaces) === */
{WHITESPACE}           { /* skip ASCII whitespace */ }
\p{Zs}+                { /* skip Unicode space separators */ }

/* === Comments === */
"//"[^\n]*             { /* single-line comment - skip */ }

/* === Newlines === */
\n                     { return NEWLINE; }

/* === Unicode escape sequences in identifiers === */
/* Example: \u{03B1} for Greek alpha (α) */

/* === Floating-point numbers (must come before integers) === */
{DIGIT}+\.{DIGIT}*     { return NUMBER; }
\.{DIGIT}+             { return NUMBER; }
{DIGIT}+[eE][+-]?{DIGIT}+ { return NUMBER; }
{DIGIT}+\.{DIGIT}*[eE][+-]?{DIGIT}+ { return NUMBER; }

/* === Numbers with Unicode digits === */
\p{Nd}+                { return INTEGER; }

/* === Integers === */
{DIGIT}+               { return INTEGER; }
0x{HEXDIGIT}+          { return INTEGER; }
0b[01]+                { return INTEGER; }

/* === String literals (with Unicode content) === */
\"([^\"\n\\]|\\.|\u{0000}-\u{FFFF})*\"   { return STRING; }
\'([^\'\n\\]|\\.)*\'   { return STRING; }

/* === Keywords === */
if                     { return IF; }
else                   { return ELSE; }
while                  { return WHILE; }
for                    { return FOR; }
in                     { return IN; }
return                 { return RETURN; }
print                  { return PRINT; }
true                   { return TRUE; }
false                  { return FALSE; }
null                   { return NULL_TOK; }

/* === Greek keywords (demonstration of Unicode keywords) === */
/* αληθής = true, ψευδής = false in Greek */
\u{03B1}\u{03BB}\u{03B7}\u{03B8}\u{03AE}\u{03C2}  { return TRUE; }
\u{03C8}\u{03B5}\u{03C5}\u{03B4}\u{03AE}\u{03C2}  { return FALSE; }

/* === Mathematical operators using Unicode symbols === */
\u{00D7}               { return TIMES; }    /* × multiplication sign */
\u{00F7}               { return DIVIDE; }   /* ÷ division sign */
\u{2212}               { return MINUS; }    /* − minus sign */
\u{2260}               { return NE; }       /* ≠ not equal */
\u{2264}               { return LE; }       /* ≤ less or equal */
\u{2265}               { return GE; }       /* ≥ greater or equal */
\u{2227}               { return AND; }      /* ∧ logical and */
\u{2228}               { return OR; }       /* ∨ logical or */
\u{00AC}               { return NOT; }      /* ¬ logical not */
\u{2192}               { return ASSIGN; }   /* → arrow (alternative assign) */

/* === Currency symbols === */
\p{Sc}+\p{Nd}+\.?\p{Nd}*  { return NUMBER; }  /* e.g., $100, €50.00, ¥1000 */

/* === Emoji support (as identifiers or operators) === */
\p{So}+                { return IDENTIFIER; }  /* Symbol, Other (includes emoji) */

/* === Multi-character operators === */
"||"                   { return OR; }
"&&"                   { return AND; }
"=="                   { return EQ; }
"!="                   { return NE; }
"<="                   { return LE; }
">="                   { return GE; }
".."                   { return DOTDOT; }

/* === Single-character operators === */
"+"                    { return PLUS; }
"-"                    { return MINUS; }
"*"                    { return TIMES; }
"/"                    { return DIVIDE; }
"%"                    { return MOD; }
"^"                    { return POWER; }
"<"                    { return LT; }
">"                    { return GT; }
"!"                    { return NOT; }
"?"                    { return QUESTION; }
":"                    { return COLON; }
"="                    { return ASSIGN; }

/* === Delimiters === */
"("                    { return LPAREN; }
")"                    { return RPAREN; }
"["                    { return LBRACKET; }
"]"                    { return RBRACKET; }
"{"                    { return LBRACE; }
"}"                    { return RBRACE; }
";"                    { return SEMICOLON; }
","                    { return COMMA; }

/* === Unicode identifiers === */
/* Identifiers can contain: */
/*   - ASCII letters and digits */
/*   - Unicode letters from any script */
/*   - Combining marks for diacritics */

/* Greek identifiers (e.g., αβγ, Δx, πr²) */
{GREEK_LETTER}({GREEK_LETTER}|{DIGIT})*  { return IDENTIFIER; }

/* Cyrillic identifiers (e.g., привет, счёт) */
{CYRILLIC_LETTER}({CYRILLIC_LETTER}|{DIGIT})*  { return IDENTIFIER; }

/* CJK identifiers (Chinese/Japanese/Korean) */
{HAN_CHAR}+            { return IDENTIFIER; }
({HIRAGANA}|{KATAKANA})+  { return IDENTIFIER; }

/* Arabic identifiers */
{ARABIC_LETTER}({ARABIC_LETTER}|{DIGIT})*  { return IDENTIFIER; }

/* Hebrew identifiers */
{HEBREW_LETTER}({HEBREW_LETTER}|{DIGIT})*  { return IDENTIFIER; }

/* Thai identifiers */
{THAI_CHAR}({THAI_CHAR}|{DIGIT})*  { return IDENTIFIER; }

/* Devanagari identifiers (Hindi, Sanskrit) */
{DEVANAGARI}({DEVANAGARI}|{DIGIT})*  { return IDENTIFIER; }

/* General Unicode identifier: letter followed by letters/digits/marks */
\p{L}(\p{L}|\p{Nd}|\p{Mn}|\p{Mc})*  { return IDENTIFIER; }

/* === ASCII identifiers (fallback, after keywords) === */
{LETTER}({LETTER}|{DIGIT})*  { return IDENTIFIER; }

/* === Error: unrecognized character === */
.                      { return ERROR; }

%%

/* ============================================================================
 * User Code Section
 * ============================================================================ */

