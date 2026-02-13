# MiniJava Example

A subset of Java for teaching compilers. This example shows a more complex grammar.

## Lexer Specification (minijava.l)

```lex
%{
/* MiniJava Lexer */
%}

%x COMMENT

%%

"class"         { return CLASS; }
"public"        { return PUBLIC; }
"static"        { return STATIC; }
"void"          { return VOID; }
"main"          { return MAIN; }
"String"        { return STRING; }
"extends"       { return EXTENDS; }
"return"        { return RETURN; }
"int"           { return INT; }
"boolean"       { return BOOLEAN; }
"if"            { return IF; }
"else"          { return ELSE; }
"while"         { return WHILE; }
"System.out.println" { return PRINTLN; }
"length"        { return LENGTH; }
"true"          { return TRUE; }
"false"         { return FALSE; }
"this"          { return THIS; }
"new"           { return NEW; }

[a-zA-Z_][a-zA-Z0-9_]*  { return IDENTIFIER; }
[0-9]+                   { return INTEGER_LITERAL; }

"{"             { return LBRACE; }
"}"             { return RBRACE; }
"["             { return LBRACKET; }
"]"             { return RBRACKET; }
"("             { return LPAREN; }
")"             { return RPAREN; }
";"             { return SEMICOLON; }
","             { return COMMA; }
"."             { return DOT; }
"="             { return ASSIGN; }
"&&"            { return AND; }
"<"             { return LT; }
"+"             { return PLUS; }
"-"             { return MINUS; }
"*"             { return TIMES; }
"!"             { return NOT; }

"//".*          { /* skip single-line comment */ }
"/*"            { BEGIN(COMMENT); }
<COMMENT>"*/"   { BEGIN(INITIAL); }
<COMMENT>.      { /* skip */ }
<COMMENT>\n     { /* skip */ }

[ \t\n\r]+      { /* skip whitespace */ }
.               { return ERROR; }

%%
```

## Grammar Specification (minijava.y)

```yacc
%{
/* MiniJava Parser */
%}

%token CLASS PUBLIC STATIC VOID MAIN STRING EXTENDS RETURN
%token INT BOOLEAN IF ELSE WHILE PRINTLN LENGTH
%token TRUE FALSE THIS NEW
%token IDENTIFIER INTEGER_LITERAL
%token LBRACE RBRACE LBRACKET RBRACKET LPAREN RPAREN
%token SEMICOLON COMMA DOT ASSIGN
%token AND LT PLUS MINUS TIMES NOT
%token ERROR

%left AND
%left LT
%left PLUS MINUS
%left TIMES
%right NOT
%left DOT LBRACKET

%%

Program:
    MainClass ClassDeclarationList
    ;

MainClass:
    CLASS IDENTIFIER LBRACE
        PUBLIC STATIC VOID MAIN LPAREN STRING LBRACKET RBRACKET IDENTIFIER RPAREN
        LBRACE Statement RBRACE
    RBRACE
    ;

ClassDeclarationList:
    /* empty */
    | ClassDeclarationList ClassDeclaration
    ;

ClassDeclaration:
    CLASS IDENTIFIER LBRACE VarDeclarationList MethodDeclarationList RBRACE
    | CLASS IDENTIFIER EXTENDS IDENTIFIER LBRACE VarDeclarationList MethodDeclarationList RBRACE
    ;

VarDeclarationList:
    /* empty */
    | VarDeclarationList VarDeclaration
    ;

VarDeclaration:
    Type IDENTIFIER SEMICOLON
    ;

MethodDeclarationList:
    /* empty */
    | MethodDeclarationList MethodDeclaration
    ;

MethodDeclaration:
    PUBLIC Type IDENTIFIER LPAREN FormalParameterList RPAREN
        LBRACE VarDeclarationList StatementList RETURN Expression SEMICOLON RBRACE
    ;

FormalParameterList:
    /* empty */
    | FormalParameter FormalParameterRest
    ;

FormalParameter:
    Type IDENTIFIER
    ;

FormalParameterRest:
    /* empty */
    | FormalParameterRest COMMA FormalParameter
    ;

Type:
    INT LBRACKET RBRACKET
    | BOOLEAN
    | INT
    | IDENTIFIER
    ;

StatementList:
    /* empty */
    | StatementList Statement
    ;

Statement:
    LBRACE StatementList RBRACE
    | IF LPAREN Expression RPAREN Statement ELSE Statement
    | WHILE LPAREN Expression RPAREN Statement
    | PRINTLN LPAREN Expression RPAREN SEMICOLON
    | IDENTIFIER ASSIGN Expression SEMICOLON
    | IDENTIFIER LBRACKET Expression RBRACKET ASSIGN Expression SEMICOLON
    ;

Expression:
    Expression AND Expression
    | Expression LT Expression
    | Expression PLUS Expression
    | Expression MINUS Expression
    | Expression TIMES Expression
    | Expression LBRACKET Expression RBRACKET
    | Expression DOT LENGTH
    | Expression DOT IDENTIFIER LPAREN ExpressionList RPAREN
    | INTEGER_LITERAL
    | TRUE
    | FALSE
    | IDENTIFIER
    | THIS
    | NEW INT LBRACKET Expression RBRACKET
    | NEW IDENTIFIER LPAREN RPAREN
    | NOT Expression
    | LPAREN Expression RPAREN
    ;

ExpressionList:
    /* empty */
    | Expression ExpressionRest
    ;

ExpressionRest:
    /* empty */
    | ExpressionRest COMMA Expression
    ;

%%
```

## Generate Code

```bash
openlexer gen-lexer --lexer minijava.l --lang java --output ./
openlexer gen-parser --parser minijava.y --lang java --output ./
```

## Sample MiniJava Program

```java
class Factorial {
    public static void main(String[] args) {
        System.out.println(new Fac().compute(10));
    }
}

class Fac {
    public int compute(int num) {
        int result;
        if (num < 1)
            result = 1;
        else
            result = num * this.compute(num - 1);
        return result;
    }
}
```

## Limitations

MiniJava is a teaching language. It does not support:
- Interfaces
- Overloading
- Static fields
- Access modifiers other than `public`
- Floating point numbers
- Strings (except in `main` signature)
