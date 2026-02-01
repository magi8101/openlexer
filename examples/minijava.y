%{
/* MiniJava Parser - Bison Grammar
 * Based on MiniJava Language Specification from Book
 * 
 * Goal ::= MainClass ( TypeDeclaration )* <EOF>
 */

#include <stdio.h>
#include <stdlib.h>

void yyerror(const char *s);
int yylex(void);

%}

/* Token declarations */
%token CLASS PUBLIC STATIC VOID MAIN STRING EXTENDS RETURN
%token INT BOOLEAN IF ELSE WHILE
%token SYSTEM_OUT_PRINTLN THIS NEW LENGTH TRUE FALSE
%token INTEGER_LITERAL IDENTIFIER
%token LBRACE RBRACE LPAREN RPAREN LBRACKET RBRACKET
%token SEMICOLON COMMA DOT ASSIGN
%token AND LT PLUS MINUS TIMES NOT

%%

/* Goal ::= MainClass ( TypeDeclaration )* <EOF> */
Goal:
    MainClass TypeDeclarationList
    { printf("Parsed Goal\n"); }
    ;

TypeDeclarationList:
    /* empty - ( TypeDeclaration )* allows zero */
    | TypeDeclarationList TypeDeclaration
    ;

/* MainClass ::= "class" Identifier "{" "public" "static" "void" "main" "(" "String" "[" "]" Identifier ")" "{" PrintStatement "}" "}" */
MainClass:
    CLASS Identifier LBRACE PUBLIC STATIC VOID MAIN LPAREN STRING LBRACKET RBRACKET Identifier RPAREN LBRACE PrintStatement RBRACE RBRACE
    { printf("Parsed MainClass\n"); }
    ;

/* TypeDeclaration ::= ClassDeclaration | ClassExtendsDeclaration */
TypeDeclaration:
    ClassDeclaration
    | ClassExtendsDeclaration
    ;

/* ClassDeclaration ::= "class" Identifier "{" ( VarDeclaration )* ( MethodDeclaration )* "}" */
ClassDeclaration:
    CLASS Identifier LBRACE VarDeclarationList MethodDeclarationList RBRACE
    { printf("Parsed ClassDeclaration\n"); }
    ;

/* ClassExtendsDeclaration ::= "class" Identifier "extends" Identifier "{" ( VarDeclaration )* ( MethodDeclaration )* "}" */
ClassExtendsDeclaration:
    CLASS Identifier EXTENDS Identifier LBRACE VarDeclarationList MethodDeclarationList RBRACE
    { printf("Parsed ClassExtendsDeclaration\n"); }
    ;

VarDeclarationList:
    /* empty - ( VarDeclaration )* */
    | VarDeclarationList VarDeclaration
    ;

MethodDeclarationList:
    /* empty - ( MethodDeclaration )* */
    | MethodDeclarationList MethodDeclaration
    ;

/* VarDeclaration ::= Type Identifier ";" */
VarDeclaration:
    Type Identifier SEMICOLON
    { printf("Parsed VarDeclaration\n"); }
    ;

/* MethodDeclaration ::= "public" Type Identifier "(" ( FormalParameterList )? ")" "{" ( VarDeclaration )* ( Statement )* "return" Expression ";" "}" */
MethodDeclaration:
    PUBLIC Type Identifier LPAREN FormalParameterListOpt RPAREN LBRACE VarDeclarationList StatementList RETURN Expression SEMICOLON RBRACE
    { printf("Parsed MethodDeclaration\n"); }
    ;

FormalParameterListOpt:
    /* empty - ( FormalParameterList )? */
    | FormalParameterList
    ;

/* FormalParameterList ::= FormalParameter ( FormalParameterRest )* */
FormalParameterList:
    FormalParameter FormalParameterRestList
    ;

FormalParameterRestList:
    /* empty */
    | FormalParameterRestList FormalParameterRest
    ;

/* FormalParameter ::= Type Identifier */
FormalParameter:
    Type Identifier
    ;

/* FormalParameterRest ::= "," FormalParameter */
FormalParameterRest:
    COMMA FormalParameter
    ;

/* Type ::= ArrayType | BooleanType | IntegerType | Identifier */
Type:
    ArrayType
    | BooleanType
    | IntegerType
    | Identifier
    ;

/* ArrayType ::= "int" "[" "]" */
ArrayType:
    INT LBRACKET RBRACKET
    ;

/* BooleanType ::= "boolean" */
BooleanType:
    BOOLEAN
    ;

/* IntegerType ::= "int" */
IntegerType:
    INT
    ;

StatementList:
    /* empty - ( Statement )* */
    | StatementList Statement
    ;

/* Statement ::= Block | AssignmentStatement | ArrayAssignmentStatement | IfStatement | WhileStatement | PrintStatement */
Statement:
    Block
    | AssignmentStatement
    | ArrayAssignmentStatement
    | IfStatement
    | WhileStatement
    | PrintStatement
    ;

/* Block ::= "{" ( Statement )* "}" */
Block:
    LBRACE StatementList RBRACE
    ;

/* AssignmentStatement ::= Identifier "=" Expression ";" */
AssignmentStatement:
    Identifier ASSIGN Expression SEMICOLON
    ;

/* ArrayAssignmentStatement ::= Identifier "[" Expression "]" "=" Expression ";" */
ArrayAssignmentStatement:
    Identifier LBRACKET Expression RBRACKET ASSIGN Expression SEMICOLON
    ;

/* IfStatement ::= "if" "(" Expression ")" Statement "else" Statement */
IfStatement:
    IF LPAREN Expression RPAREN Statement ELSE Statement
    ;

/* WhileStatement ::= "while" "(" Expression ")" Statement */
WhileStatement:
    WHILE LPAREN Expression RPAREN Statement
    ;

/* PrintStatement ::= "System.out.println" "(" Expression ")" ";" */
PrintStatement:
    SYSTEM_OUT_PRINTLN LPAREN Expression RPAREN SEMICOLON
    ;

/* Expression ::= AndExpression | CompareExpression | PlusExpression | MinusExpression | TimesExpression | ArrayLookup | ArrayLength | MessageSend | PrimaryExpression */
Expression:
    AndExpression
    | CompareExpression
    | PlusExpression
    | MinusExpression
    | TimesExpression
    | ArrayLookup
    | ArrayLength
    | MessageSend
    | PrimaryExpression
    ;

/* AndExpression ::= PrimaryExpression "&" PrimaryExpression */
AndExpression:
    PrimaryExpression AND PrimaryExpression
    ;

/* CompareExpression ::= PrimaryExpression "<" PrimaryExpression */
CompareExpression:
    PrimaryExpression LT PrimaryExpression
    ;

/* PlusExpression ::= PrimaryExpression "+" PrimaryExpression */
PlusExpression:
    PrimaryExpression PLUS PrimaryExpression
    ;

/* MinusExpression ::= PrimaryExpression "-" PrimaryExpression */
MinusExpression:
    PrimaryExpression MINUS PrimaryExpression
    ;

/* TimesExpression ::= PrimaryExpression "*" PrimaryExpression */
TimesExpression:
    PrimaryExpression TIMES PrimaryExpression
    ;

/* ArrayLookup ::= PrimaryExpression "[" PrimaryExpression "]" */
ArrayLookup:
    PrimaryExpression LBRACKET PrimaryExpression RBRACKET
    ;

/* ArrayLength ::= PrimaryExpression "." "length" */
ArrayLength:
    PrimaryExpression DOT LENGTH
    ;

/* MessageSend ::= PrimaryExpression "." Identifier "(" ( ExpressionList )? ")" */
MessageSend:
    PrimaryExpression DOT Identifier LPAREN ExpressionListOpt RPAREN
    ;

ExpressionListOpt:
    /* empty */
    | ExpressionList
    ;

/* ExpressionList ::= Expression ( ExpressionRest )* */
ExpressionList:
    Expression ExpressionRestList
    ;

ExpressionRestList:
    /* empty */
    | ExpressionRestList ExpressionRest
    ;

/* ExpressionRest ::= "," Expression */
ExpressionRest:
    COMMA Expression
    ;

/* PrimaryExpression ::= IntegerLiteral | TrueLiteral | FalseLiteral | Identifier | ThisExpression | ArrayAllocationExpression | AllocationExpression | NotExpression | BracketExpression */
PrimaryExpression:
    IntegerLiteral
    | TrueLiteral
    | FalseLiteral
    | Identifier
    | ThisExpression
    | ArrayAllocationExpression
    | AllocationExpression
    | NotExpression
    | BracketExpression
    ;

/* IntegerLiteral ::= <INTEGER_LITERAL> */
IntegerLiteral:
    INTEGER_LITERAL
    ;

/* TrueLiteral ::= "true" */
TrueLiteral:
    TRUE
    ;

/* FalseLiteral ::= "false" */
FalseLiteral:
    FALSE
    ;

/* Identifier ::= <IDENTIFIER> */
Identifier:
    IDENTIFIER
    ;

/* ThisExpression ::= "this" */
ThisExpression:
    THIS
    ;

/* ArrayAllocationExpression ::= "new" "int" "[" Expression "]" */
ArrayAllocationExpression:
    NEW INT LBRACKET Expression RBRACKET
    ;

/* AllocationExpression ::= "new" Identifier "(" ")" */
AllocationExpression:
    NEW Identifier LPAREN RPAREN
    ;

/* NotExpression ::= "!" PrimaryExpression */
NotExpression:
    NOT PrimaryExpression
    ;

/* BracketExpression ::= "(" Expression ")" */
BracketExpression:
    LPAREN Expression RPAREN
    ;

%%

void yyerror(const char *s) {
    fprintf(stderr, "Parse error: %s\n", s);
}

int main() {
    printf("MiniJava Parser\n");
    printf("Enter MiniJava code:\n\n");
    
    if (yyparse() == 0) {
        printf("\nParsing successful!\n");
    } else {
        printf("\nParsing failed!\n");
    }
    
    return 0;
}
