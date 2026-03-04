import java.util.Stack;
import java.util.HashMap;
import java.util.ArrayList;

class Parser {
    private static class Action {
        char type; // 'S', 'R', 'A', 'E'
        int param;
        Action(char t, int p) { type = t; param = p; }
    }

    private static class StackEntry {
        int state;
        Object value;
        StackEntry(int s, Object v) {
            state = s;
            value = v;
        }
    }

    private HashMap<Integer, HashMap<String, Action>> actionTable = new HashMap<>();
    private HashMap<Integer, HashMap<String, Integer>> gotoTable = new HashMap<>();
    private int[][] rules; // [lhs_id, rhs_len]
    private int yynerrs = 0;

    private Lexer defaultLexer;

    public Parser() {
        initTables();
    }

    public Parser(Lexer lexer) {
        this();
        this.defaultLexer = lexer;
    }

    private void initTables() {
        actionTable.put(33, new HashMap<>());
        actionTable.get(33).put("RPAREN", new Action('R', 44));
        actionTable.get(33).put("RBRACKET", new Action('R', 44));
        actionTable.get(33).put("COMMA", new Action('R', 44));
        actionTable.get(33).put("SEMICOLON", new Action('R', 44));
        actionTable.put(142, new HashMap<>());
        actionTable.get(142).put("IDENTIFIER", new Action('R', 36));
        actionTable.get(142).put("ELSE", new Action('R', 36));
        actionTable.get(142).put("IF", new Action('R', 36));
        actionTable.get(142).put("SYSTEM_OUT_PRINTLN", new Action('R', 36));
        actionTable.get(142).put("LBRACE", new Action('R', 36));
        actionTable.get(142).put("WHILE", new Action('R', 36));
        actionTable.get(142).put("RETURN", new Action('R', 36));
        actionTable.get(142).put("RBRACE", new Action('R', 36));
        actionTable.put(59, new HashMap<>());
        actionTable.get(59).put("INTEGER_LITERAL", new Action('S', 43));
        actionTable.get(59).put("LPAREN", new Action('S', 22));
        actionTable.get(59).put("NEW", new Action('S', 45));
        actionTable.get(59).put("FALSE", new Action('S', 39));
        actionTable.get(59).put("THIS", new Action('S', 44));
        actionTable.get(59).put("NOT", new Action('S', 21));
        actionTable.get(59).put("TRUE", new Action('S', 38));
        actionTable.get(59).put("IDENTIFIER", new Action('S', 5));
        actionTable.put(61, new HashMap<>());
        actionTable.get(61).put("NOT", new Action('S', 21));
        actionTable.get(61).put("FALSE", new Action('S', 39));
        actionTable.get(61).put("NEW", new Action('S', 45));
        actionTable.get(61).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(61).put("LPAREN", new Action('S', 22));
        actionTable.get(61).put("TRUE", new Action('S', 38));
        actionTable.get(61).put("THIS", new Action('S', 44));
        actionTable.get(61).put("INTEGER_LITERAL", new Action('S', 43));
        actionTable.put(51, new HashMap<>());
        actionTable.get(51).put("THIS", new Action('S', 44));
        actionTable.get(51).put("LPAREN", new Action('S', 22));
        actionTable.get(51).put("NEW", new Action('S', 45));
        actionTable.get(51).put("NOT", new Action('S', 21));
        actionTable.get(51).put("INTEGER_LITERAL", new Action('S', 43));
        actionTable.get(51).put("FALSE", new Action('S', 39));
        actionTable.get(51).put("TRUE", new Action('S', 38));
        actionTable.get(51).put("IDENTIFIER", new Action('S', 5));
        actionTable.put(125, new HashMap<>());
        actionTable.get(125).put("SYSTEM_OUT_PRINTLN", new Action('S', 19));
        actionTable.get(125).put("WHILE", new Action('S', 128));
        actionTable.get(125).put("LBRACE", new Action('S', 132));
        actionTable.get(125).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(125).put("RETURN", new Action('S', 136));
        actionTable.get(125).put("IF", new Action('S', 131));
        actionTable.put(24, new HashMap<>());
        actionTable.get(24).put("MINUS", new Action('R', 67));
        actionTable.get(24).put("LT", new Action('R', 67));
        actionTable.get(24).put("COMMA", new Action('R', 67));
        actionTable.get(24).put("TIMES", new Action('R', 67));
        actionTable.get(24).put("DOT", new Action('R', 67));
        actionTable.get(24).put("RPAREN", new Action('R', 67));
        actionTable.get(24).put("RBRACKET", new Action('R', 67));
        actionTable.get(24).put("PLUS", new Action('R', 67));
        actionTable.get(24).put("SEMICOLON", new Action('R', 67));
        actionTable.get(24).put("AND", new Action('R', 67));
        actionTable.get(24).put("LBRACKET", new Action('R', 67));
        actionTable.put(122, new HashMap<>());
        actionTable.get(122).put("LBRACE", new Action('S', 123));
        actionTable.put(45, new HashMap<>());
        actionTable.get(45).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(45).put("INT", new Action('S', 47));
        actionTable.put(145, new HashMap<>());
        actionTable.get(145).put("LBRACE", new Action('S', 132));
        actionTable.get(145).put("WHILE", new Action('S', 128));
        actionTable.get(145).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(145).put("IF", new Action('S', 131));
        actionTable.get(145).put("SYSTEM_OUT_PRINTLN", new Action('S', 19));
        actionTable.put(96, new HashMap<>());
        actionTable.get(96).put("WHILE", new Action('R', 9));
        actionTable.get(96).put("SYSTEM_OUT_PRINTLN", new Action('R', 9));
        actionTable.get(96).put("RBRACE", new Action('R', 9));
        actionTable.get(96).put("PUBLIC", new Action('R', 9));
        actionTable.get(96).put("BOOLEAN", new Action('R', 9));
        actionTable.get(96).put("RETURN", new Action('R', 9));
        actionTable.get(96).put("IDENTIFIER", new Action('R', 9));
        actionTable.get(96).put("IF", new Action('R', 9));
        actionTable.get(96).put("INT", new Action('R', 9));
        actionTable.get(96).put("LBRACE", new Action('R', 9));
        actionTable.put(67, new HashMap<>());
        actionTable.get(67).put("RPAREN", new Action('R', 56));
        actionTable.get(67).put("RBRACKET", new Action('R', 56));
        actionTable.get(67).put("COMMA", new Action('R', 56));
        actionTable.get(67).put("SEMICOLON", new Action('R', 56));
        actionTable.put(15, new HashMap<>());
        actionTable.get(15).put("RPAREN", new Action('S', 16));
        actionTable.put(1, new HashMap<>());
        actionTable.get(1).put("$", new Action('R', 1));
        actionTable.get(1).put("CLASS", new Action('R', 1));
        actionTable.put(41, new HashMap<>());
        actionTable.get(41).put("PLUS", new Action('R', 65));
        actionTable.get(41).put("RPAREN", new Action('R', 65));
        actionTable.get(41).put("RBRACKET", new Action('R', 65));
        actionTable.get(41).put("AND", new Action('R', 65));
        actionTable.get(41).put("MINUS", new Action('R', 65));
        actionTable.get(41).put("DOT", new Action('R', 65));
        actionTable.get(41).put("COMMA", new Action('R', 65));
        actionTable.get(41).put("LBRACKET", new Action('R', 65));
        actionTable.get(41).put("LT", new Action('R', 65));
        actionTable.get(41).put("TIMES", new Action('R', 65));
        actionTable.get(41).put("SEMICOLON", new Action('R', 65));
        actionTable.put(4, new HashMap<>());
        actionTable.get(4).put("LBRACE", new Action('S', 6));
        actionTable.put(76, new HashMap<>());
        actionTable.get(76).put("COMMA", new Action('R', 63));
        actionTable.get(76).put("RPAREN", new Action('R', 63));
        actionTable.put(62, new HashMap<>());
        actionTable.get(62).put("THIS", new Action('S', 44));
        actionTable.get(62).put("TRUE", new Action('S', 38));
        actionTable.get(62).put("NOT", new Action('S', 21));
        actionTable.get(62).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(62).put("FALSE", new Action('S', 39));
        actionTable.get(62).put("NEW", new Action('S', 45));
        actionTable.get(62).put("LPAREN", new Action('S', 22));
        actionTable.get(62).put("INTEGER_LITERAL", new Action('S', 43));
        actionTable.put(139, new HashMap<>());
        actionTable.get(139).put("RBRACE", new Action('S', 140));
        actionTable.put(97, new HashMap<>());
        actionTable.get(97).put("IDENTIFIER", new Action('R', 22));
        actionTable.put(38, new HashMap<>());
        actionTable.get(38).put("LBRACKET", new Action('R', 75));
        actionTable.get(38).put("COMMA", new Action('R', 75));
        actionTable.get(38).put("SEMICOLON", new Action('R', 75));
        actionTable.get(38).put("DOT", new Action('R', 75));
        actionTable.get(38).put("MINUS", new Action('R', 75));
        actionTable.get(38).put("PLUS", new Action('R', 75));
        actionTable.get(38).put("RBRACKET", new Action('R', 75));
        actionTable.get(38).put("RPAREN", new Action('R', 75));
        actionTable.get(38).put("AND", new Action('R', 75));
        actionTable.get(38).put("TIMES", new Action('R', 75));
        actionTable.get(38).put("LT", new Action('R', 75));
        actionTable.put(93, new HashMap<>());
        actionTable.get(93).put("WHILE", new Action('R', 8));
        actionTable.get(93).put("BOOLEAN", new Action('R', 8));
        actionTable.get(93).put("INT", new Action('R', 8));
        actionTable.get(93).put("IDENTIFIER", new Action('R', 8));
        actionTable.get(93).put("LBRACE", new Action('R', 8));
        actionTable.get(93).put("RBRACE", new Action('R', 8));
        actionTable.get(93).put("RETURN", new Action('R', 8));
        actionTable.get(93).put("PUBLIC", new Action('R', 8));
        actionTable.get(93).put("IF", new Action('R', 8));
        actionTable.get(93).put("SYSTEM_OUT_PRINTLN", new Action('R', 8));
        actionTable.put(68, new HashMap<>());
        actionTable.get(68).put("LPAREN", new Action('S', 70));
        actionTable.put(35, new HashMap<>());
        actionTable.get(35).put("RPAREN", new Action('R', 66));
        actionTable.get(35).put("DOT", new Action('R', 66));
        actionTable.get(35).put("PLUS", new Action('R', 66));
        actionTable.get(35).put("AND", new Action('R', 66));
        actionTable.get(35).put("COMMA", new Action('R', 66));
        actionTable.get(35).put("LBRACKET", new Action('R', 66));
        actionTable.get(35).put("LT", new Action('R', 66));
        actionTable.get(35).put("SEMICOLON", new Action('R', 66));
        actionTable.get(35).put("MINUS", new Action('R', 66));
        actionTable.get(35).put("RBRACKET", new Action('R', 66));
        actionTable.get(35).put("TIMES", new Action('R', 66));
        actionTable.put(141, new HashMap<>());
        actionTable.get(141).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(141).put("LBRACE", new Action('S', 132));
        actionTable.get(141).put("WHILE", new Action('S', 128));
        actionTable.get(141).put("IF", new Action('S', 131));
        actionTable.get(141).put("RBRACE", new Action('S', 142));
        actionTable.get(141).put("SYSTEM_OUT_PRINTLN", new Action('S', 19));
        actionTable.put(78, new HashMap<>());
        actionTable.get(78).put("SEMICOLON", new Action('R', 58));
        actionTable.get(78).put("RPAREN", new Action('R', 58));
        actionTable.get(78).put("RBRACKET", new Action('R', 58));
        actionTable.get(78).put("COMMA", new Action('R', 58));
        actionTable.put(137, new HashMap<>());
        actionTable.get(137).put("SYSTEM_OUT_PRINTLN", new Action('R', 29));
        actionTable.get(137).put("IDENTIFIER", new Action('R', 29));
        actionTable.get(137).put("WHILE", new Action('R', 29));
        actionTable.get(137).put("LBRACE", new Action('R', 29));
        actionTable.get(137).put("RBRACE", new Action('R', 29));
        actionTable.get(137).put("IF", new Action('R', 29));
        actionTable.get(137).put("RETURN", new Action('R', 29));
        actionTable.put(103, new HashMap<>());
        actionTable.get(103).put("IDENTIFIER", new Action('R', 26));
        actionTable.put(47, new HashMap<>());
        actionTable.get(47).put("LBRACKET", new Action('S', 51));
        actionTable.put(130, new HashMap<>());
        actionTable.get(130).put("WHILE", new Action('R', 32));
        actionTable.get(130).put("IDENTIFIER", new Action('R', 32));
        actionTable.get(130).put("RBRACE", new Action('R', 32));
        actionTable.get(130).put("LBRACE", new Action('R', 32));
        actionTable.get(130).put("ELSE", new Action('R', 32));
        actionTable.get(130).put("SYSTEM_OUT_PRINTLN", new Action('R', 32));
        actionTable.get(130).put("RETURN", new Action('R', 32));
        actionTable.get(130).put("IF", new Action('R', 32));
        actionTable.put(87, new HashMap<>());
        actionTable.get(87).put("CLASS", new Action('R', 4));
        actionTable.get(87).put("$", new Action('R', 4));
        actionTable.put(58, new HashMap<>());
        actionTable.get(58).put("LENGTH", new Action('S', 69));
        actionTable.get(58).put("IDENTIFIER", new Action('S', 5));
        actionTable.put(20, new HashMap<>());
        actionTable.get(20).put("INTEGER_LITERAL", new Action('S', 43));
        actionTable.get(20).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(20).put("FALSE", new Action('S', 39));
        actionTable.get(20).put("NOT", new Action('S', 21));
        actionTable.get(20).put("THIS", new Action('S', 44));
        actionTable.get(20).put("NEW", new Action('S', 45));
        actionTable.get(20).put("TRUE", new Action('S', 38));
        actionTable.get(20).put("LPAREN", new Action('S', 22));
        actionTable.put(99, new HashMap<>());
        actionTable.get(99).put("IDENTIFIER", new Action('R', 24));
        actionTable.put(83, new HashMap<>());
        actionTable.get(83).put("COMMA", new Action('R', 81));
        actionTable.get(83).put("LT", new Action('R', 81));
        actionTable.get(83).put("RBRACKET", new Action('R', 81));
        actionTable.get(83).put("RPAREN", new Action('R', 81));
        actionTable.get(83).put("MINUS", new Action('R', 81));
        actionTable.get(83).put("TIMES", new Action('R', 81));
        actionTable.get(83).put("SEMICOLON", new Action('R', 81));
        actionTable.get(83).put("PLUS", new Action('R', 81));
        actionTable.get(83).put("DOT", new Action('R', 81));
        actionTable.get(83).put("LBRACKET", new Action('R', 81));
        actionTable.get(83).put("AND", new Action('R', 81));
        actionTable.put(104, new HashMap<>());
        actionTable.get(104).put("SEMICOLON", new Action('S', 105));
        actionTable.put(50, new HashMap<>());
        actionTable.get(50).put("DOT", new Action('R', 80));
        actionTable.get(50).put("COMMA", new Action('R', 80));
        actionTable.get(50).put("TIMES", new Action('R', 80));
        actionTable.get(50).put("AND", new Action('R', 80));
        actionTable.get(50).put("LBRACKET", new Action('R', 80));
        actionTable.get(50).put("MINUS", new Action('R', 80));
        actionTable.get(50).put("SEMICOLON", new Action('R', 80));
        actionTable.get(50).put("RBRACKET", new Action('R', 80));
        actionTable.get(50).put("RPAREN", new Action('R', 80));
        actionTable.get(50).put("PLUS", new Action('R', 80));
        actionTable.get(50).put("LT", new Action('R', 80));
        actionTable.put(3, new HashMap<>());
        actionTable.get(3).put("$", new Action('A', 0));
        actionTable.put(124, new HashMap<>());
        actionTable.get(124).put("BOOLEAN", new Action('S', 103));
        actionTable.get(124).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(124).put("RETURN", new Action('R', 28));
        actionTable.get(124).put("SYSTEM_OUT_PRINTLN", new Action('R', 28));
        actionTable.get(124).put("RBRACE", new Action('R', 28));
        actionTable.get(124).put("LBRACE", new Action('R', 28));
        actionTable.get(124).put("IF", new Action('R', 28));
        actionTable.get(124).put("INT", new Action('S', 98));
        actionTable.get(124).put("WHILE", new Action('R', 28));
        actionTable.put(18, new HashMap<>());
        actionTable.get(18).put("RBRACE", new Action('S', 84));
        actionTable.put(63, new HashMap<>());
        actionTable.get(63).put("SEMICOLON", new Action('R', 55));
        actionTable.get(63).put("RBRACKET", new Action('R', 55));
        actionTable.get(63).put("RPAREN", new Action('R', 55));
        actionTable.get(63).put("COMMA", new Action('R', 55));
        actionTable.put(74, new HashMap<>());
        actionTable.get(74).put("RPAREN", new Action('R', 61));
        actionTable.get(74).put("COMMA", new Action('S', 75));
        actionTable.put(70, new HashMap<>());
        actionTable.get(70).put("NEW", new Action('S', 45));
        actionTable.get(70).put("THIS", new Action('S', 44));
        actionTable.get(70).put("NOT", new Action('S', 21));
        actionTable.get(70).put("TRUE", new Action('S', 38));
        actionTable.get(70).put("FALSE", new Action('S', 39));
        actionTable.get(70).put("INTEGER_LITERAL", new Action('S', 43));
        actionTable.get(70).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(70).put("RPAREN", new Action('R', 59));
        actionTable.get(70).put("LPAREN", new Action('S', 22));
        actionTable.put(90, new HashMap<>());
        actionTable.get(90).put("CLASS", new Action('R', 5));
        actionTable.get(90).put("$", new Action('R', 5));
        actionTable.put(56, new HashMap<>());
        actionTable.get(56).put("NOT", new Action('S', 21));
        actionTable.get(56).put("NEW", new Action('S', 45));
        actionTable.get(56).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(56).put("TRUE", new Action('S', 38));
        actionTable.get(56).put("FALSE", new Action('S', 39));
        actionTable.get(56).put("INTEGER_LITERAL", new Action('S', 43));
        actionTable.get(56).put("THIS", new Action('S', 44));
        actionTable.get(56).put("LPAREN", new Action('S', 22));
        actionTable.put(16, new HashMap<>());
        actionTable.get(16).put("LBRACE", new Action('S', 17));
        actionTable.put(81, new HashMap<>());
        actionTable.get(81).put("RPAREN", new Action('S', 82));
        actionTable.put(94, new HashMap<>());
        actionTable.get(94).put("PUBLIC", new Action('R', 10));
        actionTable.get(94).put("BOOLEAN", new Action('S', 103));
        actionTable.get(94).put("INT", new Action('S', 98));
        actionTable.get(94).put("RBRACE", new Action('R', 10));
        actionTable.get(94).put("IDENTIFIER", new Action('S', 5));
        actionTable.put(95, new HashMap<>());
        actionTable.get(95).put("PUBLIC", new Action('S', 110));
        actionTable.get(95).put("RBRACE", new Action('S', 109));
        actionTable.put(106, new HashMap<>());
        actionTable.get(106).put("RBRACKET", new Action('S', 107));
        actionTable.put(101, new HashMap<>());
        actionTable.get(101).put("IDENTIFIER", new Action('S', 5));
        actionTable.put(113, new HashMap<>());
        actionTable.get(113).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(113).put("RPAREN", new Action('R', 14));
        actionTable.get(113).put("BOOLEAN", new Action('S', 103));
        actionTable.get(113).put("INT", new Action('S', 98));
        actionTable.put(6, new HashMap<>());
        actionTable.get(6).put("PUBLIC", new Action('S', 7));
        actionTable.put(71, new HashMap<>());
        actionTable.get(71).put("RPAREN", new Action('S', 78));
        actionTable.put(77, new HashMap<>());
        actionTable.get(77).put("RPAREN", new Action('R', 64));
        actionTable.get(77).put("COMMA", new Action('R', 64));
        actionTable.put(128, new HashMap<>());
        actionTable.get(128).put("LPAREN", new Action('S', 149));
        actionTable.put(158, new HashMap<>());
        actionTable.get(158).put("ASSIGN", new Action('S', 159));
        actionTable.put(17, new HashMap<>());
        actionTable.get(17).put("SYSTEM_OUT_PRINTLN", new Action('S', 19));
        actionTable.put(55, new HashMap<>());
        actionTable.get(55).put("WHILE", new Action('R', 41));
        actionTable.get(55).put("LBRACE", new Action('R', 41));
        actionTable.get(55).put("RETURN", new Action('R', 41));
        actionTable.get(55).put("ELSE", new Action('R', 41));
        actionTable.get(55).put("IF", new Action('R', 41));
        actionTable.get(55).put("IDENTIFIER", new Action('R', 41));
        actionTable.get(55).put("RBRACE", new Action('R', 41));
        actionTable.get(55).put("SYSTEM_OUT_PRINTLN", new Action('R', 41));
        actionTable.put(44, new HashMap<>());
        actionTable.get(44).put("SEMICOLON", new Action('R', 78));
        actionTable.get(44).put("DOT", new Action('R', 78));
        actionTable.get(44).put("RPAREN", new Action('R', 78));
        actionTable.get(44).put("LT", new Action('R', 78));
        actionTable.get(44).put("RBRACKET", new Action('R', 78));
        actionTable.get(44).put("COMMA", new Action('R', 78));
        actionTable.get(44).put("AND", new Action('R', 78));
        actionTable.get(44).put("PLUS", new Action('R', 78));
        actionTable.get(44).put("MINUS", new Action('R', 78));
        actionTable.get(44).put("TIMES", new Action('R', 78));
        actionTable.get(44).put("LBRACKET", new Action('R', 78));
        actionTable.put(114, new HashMap<>());
        actionTable.get(114).put("IDENTIFIER", new Action('S', 5));
        actionTable.put(140, new HashMap<>());
        actionTable.get(140).put("PUBLIC", new Action('R', 13));
        actionTable.get(140).put("RBRACE", new Action('R', 13));
        actionTable.put(144, new HashMap<>());
        actionTable.get(144).put("RPAREN", new Action('S', 145));
        actionTable.put(30, new HashMap<>());
        actionTable.get(30).put("RBRACKET", new Action('R', 50));
        actionTable.get(30).put("AND", new Action('S', 61));
        actionTable.get(30).put("DOT", new Action('S', 58));
        actionTable.get(30).put("COMMA", new Action('R', 50));
        actionTable.get(30).put("RPAREN", new Action('R', 50));
        actionTable.get(30).put("SEMICOLON", new Action('R', 50));
        actionTable.get(30).put("PLUS", new Action('S', 56));
        actionTable.get(30).put("MINUS", new Action('S', 60));
        actionTable.get(30).put("LBRACKET", new Action('S', 59));
        actionTable.get(30).put("LT", new Action('S', 57));
        actionTable.get(30).put("TIMES", new Action('S', 62));
        actionTable.put(159, new HashMap<>());
        actionTable.get(159).put("INTEGER_LITERAL", new Action('S', 43));
        actionTable.get(159).put("TRUE", new Action('S', 38));
        actionTable.get(159).put("FALSE", new Action('S', 39));
        actionTable.get(159).put("LPAREN", new Action('S', 22));
        actionTable.get(159).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(159).put("NOT", new Action('S', 21));
        actionTable.get(159).put("THIS", new Action('S', 44));
        actionTable.get(159).put("NEW", new Action('S', 45));
        actionTable.put(34, new HashMap<>());
        actionTable.get(34).put("COMMA", new Action('R', 45));
        actionTable.get(34).put("RBRACKET", new Action('R', 45));
        actionTable.get(34).put("SEMICOLON", new Action('R', 45));
        actionTable.get(34).put("RPAREN", new Action('R', 45));
        actionTable.put(49, new HashMap<>());
        actionTable.get(49).put("RPAREN", new Action('S', 50));
        actionTable.put(120, new HashMap<>());
        actionTable.get(120).put("BOOLEAN", new Action('S', 103));
        actionTable.get(120).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(120).put("INT", new Action('S', 98));
        actionTable.put(57, new HashMap<>());
        actionTable.get(57).put("FALSE", new Action('S', 39));
        actionTable.get(57).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(57).put("INTEGER_LITERAL", new Action('S', 43));
        actionTable.get(57).put("NOT", new Action('S', 21));
        actionTable.get(57).put("TRUE", new Action('S', 38));
        actionTable.get(57).put("THIS", new Action('S', 44));
        actionTable.get(57).put("NEW", new Action('S', 45));
        actionTable.get(57).put("LPAREN", new Action('S', 22));
        actionTable.put(46, new HashMap<>());
        actionTable.get(46).put("SEMICOLON", new Action('R', 48));
        actionTable.get(46).put("COMMA", new Action('R', 48));
        actionTable.get(46).put("RPAREN", new Action('R', 48));
        actionTable.get(46).put("RBRACKET", new Action('R', 48));
        actionTable.put(138, new HashMap<>());
        actionTable.get(138).put("SEMICOLON", new Action('S', 139));
        actionTable.put(66, new HashMap<>());
        actionTable.get(66).put("RBRACKET", new Action('S', 67));
        actionTable.put(14, new HashMap<>());
        actionTable.get(14).put("IDENTIFIER", new Action('S', 5));
        actionTable.put(166, new HashMap<>());
        actionTable.get(166).put("RBRACE", new Action('S', 167));
        actionTable.get(166).put("PUBLIC", new Action('S', 110));
        actionTable.put(22, new HashMap<>());
        actionTable.get(22).put("NEW", new Action('S', 45));
        actionTable.get(22).put("TRUE", new Action('S', 38));
        actionTable.get(22).put("INTEGER_LITERAL", new Action('S', 43));
        actionTable.get(22).put("LPAREN", new Action('S', 22));
        actionTable.get(22).put("FALSE", new Action('S', 39));
        actionTable.get(22).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(22).put("NOT", new Action('S', 21));
        actionTable.get(22).put("THIS", new Action('S', 44));
        actionTable.put(109, new HashMap<>());
        actionTable.get(109).put("$", new Action('R', 6));
        actionTable.get(109).put("CLASS", new Action('R', 6));
        actionTable.put(32, new HashMap<>());
        actionTable.get(32).put("RBRACKET", new Action('R', 47));
        actionTable.get(32).put("SEMICOLON", new Action('R', 47));
        actionTable.get(32).put("RPAREN", new Action('R', 47));
        actionTable.get(32).put("COMMA", new Action('R', 47));
        actionTable.put(36, new HashMap<>());
        actionTable.get(36).put("COMMA", new Action('R', 42));
        actionTable.get(36).put("RPAREN", new Action('R', 42));
        actionTable.get(36).put("SEMICOLON", new Action('R', 42));
        actionTable.get(36).put("RBRACKET", new Action('R', 42));
        actionTable.put(39, new HashMap<>());
        actionTable.get(39).put("MINUS", new Action('R', 76));
        actionTable.get(39).put("DOT", new Action('R', 76));
        actionTable.get(39).put("PLUS", new Action('R', 76));
        actionTable.get(39).put("LBRACKET", new Action('R', 76));
        actionTable.get(39).put("COMMA", new Action('R', 76));
        actionTable.get(39).put("RBRACKET", new Action('R', 76));
        actionTable.get(39).put("AND", new Action('R', 76));
        actionTable.get(39).put("SEMICOLON", new Action('R', 76));
        actionTable.get(39).put("LT", new Action('R', 76));
        actionTable.get(39).put("RPAREN", new Action('R', 76));
        actionTable.get(39).put("TIMES", new Action('R', 76));
        actionTable.put(85, new HashMap<>());
        actionTable.get(85).put("$", new Action('R', 3));
        actionTable.get(85).put("CLASS", new Action('R', 3));
        actionTable.put(8, new HashMap<>());
        actionTable.get(8).put("VOID", new Action('S', 9));
        actionTable.put(135, new HashMap<>());
        actionTable.get(135).put("IDENTIFIER", new Action('R', 31));
        actionTable.get(135).put("ELSE", new Action('R', 31));
        actionTable.get(135).put("SYSTEM_OUT_PRINTLN", new Action('R', 31));
        actionTable.get(135).put("RETURN", new Action('R', 31));
        actionTable.get(135).put("RBRACE", new Action('R', 31));
        actionTable.get(135).put("IF", new Action('R', 31));
        actionTable.get(135).put("WHILE", new Action('R', 31));
        actionTable.get(135).put("LBRACE", new Action('R', 31));
        actionTable.put(64, new HashMap<>());
        actionTable.get(64).put("COMMA", new Action('R', 51));
        actionTable.get(64).put("SEMICOLON", new Action('R', 51));
        actionTable.get(64).put("RPAREN", new Action('R', 51));
        actionTable.get(64).put("RBRACKET", new Action('R', 51));
        actionTable.put(91, new HashMap<>());
        actionTable.get(91).put("LBRACE", new Action('S', 93));
        actionTable.get(91).put("EXTENDS", new Action('S', 92));
        actionTable.put(5, new HashMap<>());
        actionTable.get(5).put("RBRACKET", new Action('R', 77));
        actionTable.get(5).put("RPAREN", new Action('R', 77));
        actionTable.get(5).put("LBRACKET", new Action('R', 77));
        actionTable.get(5).put("COMMA", new Action('R', 77));
        actionTable.get(5).put("LBRACE", new Action('R', 77));
        actionTable.get(5).put("TIMES", new Action('R', 77));
        actionTable.get(5).put("MINUS", new Action('R', 77));
        actionTable.get(5).put("LT", new Action('R', 77));
        actionTable.get(5).put("PLUS", new Action('R', 77));
        actionTable.get(5).put("SEMICOLON", new Action('R', 77));
        actionTable.get(5).put("EXTENDS", new Action('R', 77));
        actionTable.get(5).put("ASSIGN", new Action('R', 77));
        actionTable.get(5).put("DOT", new Action('R', 77));
        actionTable.get(5).put("IDENTIFIER", new Action('R', 77));
        actionTable.get(5).put("AND", new Action('R', 77));
        actionTable.get(5).put("LPAREN", new Action('R', 77));
        actionTable.put(105, new HashMap<>());
        actionTable.get(105).put("RBRACE", new Action('R', 12));
        actionTable.get(105).put("IDENTIFIER", new Action('R', 12));
        actionTable.get(105).put("PUBLIC", new Action('R', 12));
        actionTable.get(105).put("IF", new Action('R', 12));
        actionTable.get(105).put("SYSTEM_OUT_PRINTLN", new Action('R', 12));
        actionTable.get(105).put("BOOLEAN", new Action('R', 12));
        actionTable.get(105).put("RETURN", new Action('R', 12));
        actionTable.get(105).put("INT", new Action('R', 12));
        actionTable.get(105).put("LBRACE", new Action('R', 12));
        actionTable.get(105).put("WHILE", new Action('R', 12));
        actionTable.put(150, new HashMap<>());
        actionTable.get(150).put("RPAREN", new Action('S', 151));
        actionTable.put(157, new HashMap<>());
        actionTable.get(157).put("RBRACKET", new Action('S', 158));
        actionTable.put(2, new HashMap<>());
        actionTable.get(2).put("IDENTIFIER", new Action('S', 5));
        actionTable.put(9, new HashMap<>());
        actionTable.get(9).put("MAIN", new Action('S', 10));
        actionTable.put(12, new HashMap<>());
        actionTable.get(12).put("LBRACKET", new Action('S', 13));
        actionTable.put(116, new HashMap<>());
        actionTable.get(116).put("RPAREN", new Action('R', 15));
        actionTable.put(28, new HashMap<>());
        actionTable.get(28).put("PLUS", new Action('R', 69));
        actionTable.get(28).put("TIMES", new Action('R', 69));
        actionTable.get(28).put("MINUS", new Action('R', 69));
        actionTable.get(28).put("RPAREN", new Action('R', 69));
        actionTable.get(28).put("SEMICOLON", new Action('R', 69));
        actionTable.get(28).put("LBRACKET", new Action('R', 69));
        actionTable.get(28).put("LT", new Action('R', 69));
        actionTable.get(28).put("COMMA", new Action('R', 69));
        actionTable.get(28).put("RBRACKET", new Action('R', 69));
        actionTable.get(28).put("AND", new Action('R', 69));
        actionTable.get(28).put("DOT", new Action('R', 69));
        actionTable.put(151, new HashMap<>());
        actionTable.get(151).put("WHILE", new Action('S', 128));
        actionTable.get(151).put("IF", new Action('S', 131));
        actionTable.get(151).put("SYSTEM_OUT_PRINTLN", new Action('S', 19));
        actionTable.get(151).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(151).put("LBRACE", new Action('S', 132));
        actionTable.put(40, new HashMap<>());
        actionTable.get(40).put("LT", new Action('R', 72));
        actionTable.get(40).put("RPAREN", new Action('R', 72));
        actionTable.get(40).put("MINUS", new Action('R', 72));
        actionTable.get(40).put("RBRACKET", new Action('R', 72));
        actionTable.get(40).put("DOT", new Action('R', 72));
        actionTable.get(40).put("SEMICOLON", new Action('R', 72));
        actionTable.get(40).put("LBRACKET", new Action('R', 72));
        actionTable.get(40).put("TIMES", new Action('R', 72));
        actionTable.get(40).put("COMMA", new Action('R', 72));
        actionTable.get(40).put("AND", new Action('R', 72));
        actionTable.get(40).put("PLUS", new Action('R', 72));
        actionTable.put(75, new HashMap<>());
        actionTable.get(75).put("THIS", new Action('S', 44));
        actionTable.get(75).put("NEW", new Action('S', 45));
        actionTable.get(75).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(75).put("TRUE", new Action('S', 38));
        actionTable.get(75).put("LPAREN", new Action('S', 22));
        actionTable.get(75).put("NOT", new Action('S', 21));
        actionTable.get(75).put("INTEGER_LITERAL", new Action('S', 43));
        actionTable.get(75).put("FALSE", new Action('S', 39));
        actionTable.put(84, new HashMap<>());
        actionTable.get(84).put("RBRACE", new Action('S', 85));
        actionTable.put(88, new HashMap<>());
        actionTable.get(88).put("IDENTIFIER", new Action('S', 5));
        actionTable.put(146, new HashMap<>());
        actionTable.get(146).put("ELSE", new Action('S', 147));
        actionTable.put(72, new HashMap<>());
        actionTable.get(72).put("COMMA", new Action('R', 62));
        actionTable.get(72).put("RPAREN", new Action('R', 62));
        actionTable.put(133, new HashMap<>());
        actionTable.get(133).put("SYSTEM_OUT_PRINTLN", new Action('R', 35));
        actionTable.get(133).put("IDENTIFIER", new Action('R', 35));
        actionTable.get(133).put("ELSE", new Action('R', 35));
        actionTable.get(133).put("RETURN", new Action('R', 35));
        actionTable.get(133).put("WHILE", new Action('R', 35));
        actionTable.get(133).put("LBRACE", new Action('R', 35));
        actionTable.get(133).put("RBRACE", new Action('R', 35));
        actionTable.get(133).put("IF", new Action('R', 35));
        actionTable.put(136, new HashMap<>());
        actionTable.get(136).put("NEW", new Action('S', 45));
        actionTable.get(136).put("LPAREN", new Action('S', 22));
        actionTable.get(136).put("NOT", new Action('S', 21));
        actionTable.get(136).put("INTEGER_LITERAL", new Action('S', 43));
        actionTable.get(136).put("THIS", new Action('S', 44));
        actionTable.get(136).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(136).put("TRUE", new Action('S', 38));
        actionTable.get(136).put("FALSE", new Action('S', 39));
        actionTable.put(48, new HashMap<>());
        actionTable.get(48).put("LPAREN", new Action('S', 49));
        actionTable.put(111, new HashMap<>());
        actionTable.get(111).put("IDENTIFIER", new Action('S', 5));
        actionTable.put(155, new HashMap<>());
        actionTable.get(155).put("SEMICOLON", new Action('S', 156));
        actionTable.put(89, new HashMap<>());
        actionTable.get(89).put("CLASS", new Action('R', 2));
        actionTable.get(89).put("$", new Action('R', 2));
        actionTable.put(52, new HashMap<>());
        actionTable.get(52).put("RBRACKET", new Action('S', 53));
        actionTable.put(60, new HashMap<>());
        actionTable.get(60).put("LPAREN", new Action('S', 22));
        actionTable.get(60).put("THIS", new Action('S', 44));
        actionTable.get(60).put("TRUE", new Action('S', 38));
        actionTable.get(60).put("NOT", new Action('S', 21));
        actionTable.get(60).put("FALSE", new Action('S', 39));
        actionTable.get(60).put("INTEGER_LITERAL", new Action('S', 43));
        actionTable.get(60).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(60).put("NEW", new Action('S', 45));
        actionTable.put(126, new HashMap<>());
        actionTable.get(126).put("ASSIGN", new Action('S', 154));
        actionTable.get(126).put("LBRACKET", new Action('S', 153));
        actionTable.put(162, new HashMap<>());
        actionTable.get(162).put("COMMA", new Action('R', 19));
        actionTable.get(162).put("RPAREN", new Action('R', 19));
        actionTable.put(37, new HashMap<>());
        actionTable.get(37).put("SEMICOLON", new Action('R', 46));
        actionTable.get(37).put("RPAREN", new Action('R', 46));
        actionTable.get(37).put("COMMA", new Action('R', 46));
        actionTable.get(37).put("RBRACKET", new Action('R', 46));
        actionTable.put(110, new HashMap<>());
        actionTable.get(110).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(110).put("BOOLEAN", new Action('S', 103));
        actionTable.get(110).put("INT", new Action('S', 98));
        actionTable.put(163, new HashMap<>());
        actionTable.get(163).put("LBRACE", new Action('S', 164));
        actionTable.put(165, new HashMap<>());
        actionTable.get(165).put("PUBLIC", new Action('R', 10));
        actionTable.get(165).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(165).put("BOOLEAN", new Action('S', 103));
        actionTable.get(165).put("INT", new Action('S', 98));
        actionTable.get(165).put("RBRACE", new Action('R', 10));
        actionTable.put(86, new HashMap<>());
        actionTable.get(86).put("$", new Action('R', 0));
        actionTable.get(86).put("CLASS", new Action('S', 88));
        actionTable.put(31, new HashMap<>());
        actionTable.get(31).put("SEMICOLON", new Action('R', 43));
        actionTable.get(31).put("RBRACKET", new Action('R', 43));
        actionTable.get(31).put("COMMA", new Action('R', 43));
        actionTable.get(31).put("RPAREN", new Action('R', 43));
        actionTable.put(117, new HashMap<>());
        actionTable.get(117).put("COMMA", new Action('R', 17));
        actionTable.get(117).put("RPAREN", new Action('R', 17));
        actionTable.put(23, new HashMap<>());
        actionTable.get(23).put("COMMA", new Action('R', 70));
        actionTable.get(23).put("MINUS", new Action('R', 70));
        actionTable.get(23).put("LT", new Action('R', 70));
        actionTable.get(23).put("SEMICOLON", new Action('R', 70));
        actionTable.get(23).put("TIMES", new Action('R', 70));
        actionTable.get(23).put("PLUS", new Action('R', 70));
        actionTable.get(23).put("AND", new Action('R', 70));
        actionTable.get(23).put("LBRACKET", new Action('R', 70));
        actionTable.get(23).put("RPAREN", new Action('R', 70));
        actionTable.get(23).put("RBRACKET", new Action('R', 70));
        actionTable.get(23).put("DOT", new Action('R', 70));
        actionTable.put(0, new HashMap<>());
        actionTable.get(0).put("CLASS", new Action('S', 2));
        actionTable.put(100, new HashMap<>());
        actionTable.get(100).put("IDENTIFIER", new Action('R', 23));
        actionTable.put(21, new HashMap<>());
        actionTable.get(21).put("NEW", new Action('S', 45));
        actionTable.get(21).put("LPAREN", new Action('S', 22));
        actionTable.get(21).put("TRUE", new Action('S', 38));
        actionTable.get(21).put("THIS", new Action('S', 44));
        actionTable.get(21).put("INTEGER_LITERAL", new Action('S', 43));
        actionTable.get(21).put("FALSE", new Action('S', 39));
        actionTable.get(21).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(21).put("NOT", new Action('S', 21));
        actionTable.put(102, new HashMap<>());
        actionTable.get(102).put("IDENTIFIER", new Action('R', 21));
        actionTable.put(143, new HashMap<>());
        actionTable.get(143).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(143).put("INTEGER_LITERAL", new Action('S', 43));
        actionTable.get(143).put("TRUE", new Action('S', 38));
        actionTable.get(143).put("LPAREN", new Action('S', 22));
        actionTable.get(143).put("FALSE", new Action('S', 39));
        actionTable.get(143).put("NOT", new Action('S', 21));
        actionTable.get(143).put("NEW", new Action('S', 45));
        actionTable.get(143).put("THIS", new Action('S', 44));
        actionTable.put(27, new HashMap<>());
        actionTable.get(27).put("LT", new Action('R', 68));
        actionTable.get(27).put("MINUS", new Action('R', 68));
        actionTable.get(27).put("PLUS", new Action('R', 68));
        actionTable.get(27).put("COMMA", new Action('R', 68));
        actionTable.get(27).put("AND", new Action('R', 68));
        actionTable.get(27).put("RBRACKET", new Action('R', 68));
        actionTable.get(27).put("SEMICOLON", new Action('R', 68));
        actionTable.get(27).put("DOT", new Action('R', 68));
        actionTable.get(27).put("TIMES", new Action('R', 68));
        actionTable.get(27).put("LBRACKET", new Action('R', 68));
        actionTable.get(27).put("RPAREN", new Action('R', 68));
        actionTable.put(153, new HashMap<>());
        actionTable.get(153).put("TRUE", new Action('S', 38));
        actionTable.get(153).put("NOT", new Action('S', 21));
        actionTable.get(153).put("INTEGER_LITERAL", new Action('S', 43));
        actionTable.get(153).put("LPAREN", new Action('S', 22));
        actionTable.get(153).put("THIS", new Action('S', 44));
        actionTable.get(153).put("FALSE", new Action('S', 39));
        actionTable.get(153).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(153).put("NEW", new Action('S', 45));
        actionTable.put(156, new HashMap<>());
        actionTable.get(156).put("IDENTIFIER", new Action('R', 37));
        actionTable.get(156).put("SYSTEM_OUT_PRINTLN", new Action('R', 37));
        actionTable.get(156).put("RBRACE", new Action('R', 37));
        actionTable.get(156).put("WHILE", new Action('R', 37));
        actionTable.get(156).put("LBRACE", new Action('R', 37));
        actionTable.get(156).put("RETURN", new Action('R', 37));
        actionTable.get(156).put("IF", new Action('R', 37));
        actionTable.get(156).put("ELSE", new Action('R', 37));
        actionTable.put(160, new HashMap<>());
        actionTable.get(160).put("SEMICOLON", new Action('S', 161));
        actionTable.put(161, new HashMap<>());
        actionTable.get(161).put("ELSE", new Action('R', 38));
        actionTable.get(161).put("WHILE", new Action('R', 38));
        actionTable.get(161).put("SYSTEM_OUT_PRINTLN", new Action('R', 38));
        actionTable.get(161).put("RETURN", new Action('R', 38));
        actionTable.get(161).put("IDENTIFIER", new Action('R', 38));
        actionTable.get(161).put("RBRACE", new Action('R', 38));
        actionTable.get(161).put("LBRACE", new Action('R', 38));
        actionTable.get(161).put("IF", new Action('R', 38));
        actionTable.put(42, new HashMap<>());
        actionTable.get(42).put("RPAREN", new Action('S', 54));
        actionTable.put(167, new HashMap<>());
        actionTable.get(167).put("CLASS", new Action('R', 7));
        actionTable.get(167).put("$", new Action('R', 7));
        actionTable.put(79, new HashMap<>());
        actionTable.get(79).put("RBRACKET", new Action('R', 52));
        actionTable.get(79).put("RPAREN", new Action('R', 52));
        actionTable.get(79).put("SEMICOLON", new Action('R', 52));
        actionTable.get(79).put("COMMA", new Action('R', 52));
        actionTable.put(129, new HashMap<>());
        actionTable.get(129).put("LBRACE", new Action('R', 34));
        actionTable.get(129).put("ELSE", new Action('R', 34));
        actionTable.get(129).put("RBRACE", new Action('R', 34));
        actionTable.get(129).put("IDENTIFIER", new Action('R', 34));
        actionTable.get(129).put("WHILE", new Action('R', 34));
        actionTable.get(129).put("SYSTEM_OUT_PRINTLN", new Action('R', 34));
        actionTable.get(129).put("IF", new Action('R', 34));
        actionTable.get(129).put("RETURN", new Action('R', 34));
        actionTable.put(92, new HashMap<>());
        actionTable.get(92).put("IDENTIFIER", new Action('S', 5));
        actionTable.put(132, new HashMap<>());
        actionTable.get(132).put("RETURN", new Action('R', 28));
        actionTable.get(132).put("WHILE", new Action('R', 28));
        actionTable.get(132).put("LBRACE", new Action('R', 28));
        actionTable.get(132).put("IDENTIFIER", new Action('R', 28));
        actionTable.get(132).put("RBRACE", new Action('R', 28));
        actionTable.get(132).put("SYSTEM_OUT_PRINTLN", new Action('R', 28));
        actionTable.get(132).put("IF", new Action('R', 28));
        actionTable.put(123, new HashMap<>());
        actionTable.get(123).put("WHILE", new Action('R', 8));
        actionTable.get(123).put("RETURN", new Action('R', 8));
        actionTable.get(123).put("LBRACE", new Action('R', 8));
        actionTable.get(123).put("SYSTEM_OUT_PRINTLN", new Action('R', 8));
        actionTable.get(123).put("IDENTIFIER", new Action('R', 8));
        actionTable.get(123).put("RBRACE", new Action('R', 8));
        actionTable.get(123).put("BOOLEAN", new Action('R', 8));
        actionTable.get(123).put("IF", new Action('R', 8));
        actionTable.get(123).put("PUBLIC", new Action('R', 8));
        actionTable.get(123).put("INT", new Action('R', 8));
        actionTable.put(80, new HashMap<>());
        actionTable.get(80).put("RBRACKET", new Action('R', 53));
        actionTable.get(80).put("COMMA", new Action('R', 53));
        actionTable.get(80).put("SEMICOLON", new Action('R', 53));
        actionTable.get(80).put("RPAREN", new Action('R', 53));
        actionTable.put(54, new HashMap<>());
        actionTable.get(54).put("SEMICOLON", new Action('S', 55));
        actionTable.put(108, new HashMap<>());
        actionTable.get(108).put("RBRACE", new Action('R', 11));
        actionTable.get(108).put("PUBLIC", new Action('R', 11));
        actionTable.put(107, new HashMap<>());
        actionTable.get(107).put("IDENTIFIER", new Action('R', 25));
        actionTable.put(53, new HashMap<>());
        actionTable.get(53).put("LBRACKET", new Action('R', 79));
        actionTable.get(53).put("TIMES", new Action('R', 79));
        actionTable.get(53).put("MINUS", new Action('R', 79));
        actionTable.get(53).put("PLUS", new Action('R', 79));
        actionTable.get(53).put("DOT", new Action('R', 79));
        actionTable.get(53).put("COMMA", new Action('R', 79));
        actionTable.get(53).put("LT", new Action('R', 79));
        actionTable.get(53).put("SEMICOLON", new Action('R', 79));
        actionTable.get(53).put("AND", new Action('R', 79));
        actionTable.get(53).put("RPAREN", new Action('R', 79));
        actionTable.get(53).put("RBRACKET", new Action('R', 79));
        actionTable.put(118, new HashMap<>());
        actionTable.get(118).put("RPAREN", new Action('R', 16));
        actionTable.get(118).put("COMMA", new Action('S', 120));
        actionTable.put(26, new HashMap<>());
        actionTable.get(26).put("DOT", new Action('R', 71));
        actionTable.get(26).put("RPAREN", new Action('R', 71));
        actionTable.get(26).put("LT", new Action('R', 71));
        actionTable.get(26).put("LBRACKET", new Action('R', 71));
        actionTable.get(26).put("COMMA", new Action('R', 71));
        actionTable.get(26).put("AND", new Action('R', 71));
        actionTable.get(26).put("RBRACKET", new Action('R', 71));
        actionTable.get(26).put("TIMES", new Action('R', 71));
        actionTable.get(26).put("PLUS", new Action('R', 71));
        actionTable.get(26).put("SEMICOLON", new Action('R', 71));
        actionTable.get(26).put("MINUS", new Action('R', 71));
        actionTable.put(82, new HashMap<>());
        actionTable.get(82).put("LBRACKET", new Action('R', 82));
        actionTable.get(82).put("RBRACKET", new Action('R', 82));
        actionTable.get(82).put("DOT", new Action('R', 82));
        actionTable.get(82).put("AND", new Action('R', 82));
        actionTable.get(82).put("COMMA", new Action('R', 82));
        actionTable.get(82).put("SEMICOLON", new Action('R', 82));
        actionTable.get(82).put("LT", new Action('R', 82));
        actionTable.get(82).put("TIMES", new Action('R', 82));
        actionTable.get(82).put("RPAREN", new Action('R', 82));
        actionTable.get(82).put("MINUS", new Action('R', 82));
        actionTable.get(82).put("PLUS", new Action('R', 82));
        actionTable.put(112, new HashMap<>());
        actionTable.get(112).put("LPAREN", new Action('S', 113));
        actionTable.put(121, new HashMap<>());
        actionTable.get(121).put("RPAREN", new Action('R', 20));
        actionTable.get(121).put("COMMA", new Action('R', 20));
        actionTable.put(127, new HashMap<>());
        actionTable.get(127).put("IDENTIFIER", new Action('R', 33));
        actionTable.get(127).put("LBRACE", new Action('R', 33));
        actionTable.get(127).put("WHILE", new Action('R', 33));
        actionTable.get(127).put("RETURN", new Action('R', 33));
        actionTable.get(127).put("ELSE", new Action('R', 33));
        actionTable.get(127).put("SYSTEM_OUT_PRINTLN", new Action('R', 33));
        actionTable.get(127).put("IF", new Action('R', 33));
        actionTable.get(127).put("RBRACE", new Action('R', 33));
        actionTable.put(147, new HashMap<>());
        actionTable.get(147).put("SYSTEM_OUT_PRINTLN", new Action('S', 19));
        actionTable.get(147).put("LBRACE", new Action('S', 132));
        actionTable.get(147).put("WHILE", new Action('S', 128));
        actionTable.get(147).put("IF", new Action('S', 131));
        actionTable.get(147).put("IDENTIFIER", new Action('S', 5));
        actionTable.put(134, new HashMap<>());
        actionTable.get(134).put("SYSTEM_OUT_PRINTLN", new Action('R', 30));
        actionTable.get(134).put("LBRACE", new Action('R', 30));
        actionTable.get(134).put("WHILE", new Action('R', 30));
        actionTable.get(134).put("RETURN", new Action('R', 30));
        actionTable.get(134).put("ELSE", new Action('R', 30));
        actionTable.get(134).put("RBRACE", new Action('R', 30));
        actionTable.get(134).put("IF", new Action('R', 30));
        actionTable.get(134).put("IDENTIFIER", new Action('R', 30));
        actionTable.put(164, new HashMap<>());
        actionTable.get(164).put("RETURN", new Action('R', 8));
        actionTable.get(164).put("LBRACE", new Action('R', 8));
        actionTable.get(164).put("SYSTEM_OUT_PRINTLN", new Action('R', 8));
        actionTable.get(164).put("IF", new Action('R', 8));
        actionTable.get(164).put("RBRACE", new Action('R', 8));
        actionTable.get(164).put("PUBLIC", new Action('R', 8));
        actionTable.get(164).put("BOOLEAN", new Action('R', 8));
        actionTable.get(164).put("INT", new Action('R', 8));
        actionTable.get(164).put("IDENTIFIER", new Action('R', 8));
        actionTable.get(164).put("WHILE", new Action('R', 8));
        actionTable.put(13, new HashMap<>());
        actionTable.get(13).put("RBRACKET", new Action('S', 14));
        actionTable.put(98, new HashMap<>());
        actionTable.get(98).put("IDENTIFIER", new Action('R', 27));
        actionTable.get(98).put("LBRACKET", new Action('S', 106));
        actionTable.put(69, new HashMap<>());
        actionTable.get(69).put("RPAREN", new Action('R', 57));
        actionTable.get(69).put("RBRACKET", new Action('R', 57));
        actionTable.get(69).put("SEMICOLON", new Action('R', 57));
        actionTable.get(69).put("COMMA", new Action('R', 57));
        actionTable.put(29, new HashMap<>());
        actionTable.get(29).put("MINUS", new Action('R', 73));
        actionTable.get(29).put("PLUS", new Action('R', 73));
        actionTable.get(29).put("AND", new Action('R', 73));
        actionTable.get(29).put("RBRACKET", new Action('R', 73));
        actionTable.get(29).put("RPAREN", new Action('R', 73));
        actionTable.get(29).put("COMMA", new Action('R', 73));
        actionTable.get(29).put("LBRACKET", new Action('R', 73));
        actionTable.get(29).put("LT", new Action('R', 73));
        actionTable.get(29).put("DOT", new Action('R', 73));
        actionTable.get(29).put("TIMES", new Action('R', 73));
        actionTable.get(29).put("SEMICOLON", new Action('R', 73));
        actionTable.put(11, new HashMap<>());
        actionTable.get(11).put("STRING", new Action('S', 12));
        actionTable.put(65, new HashMap<>());
        actionTable.get(65).put("RPAREN", new Action('R', 54));
        actionTable.get(65).put("RBRACKET", new Action('R', 54));
        actionTable.get(65).put("COMMA", new Action('R', 54));
        actionTable.get(65).put("SEMICOLON", new Action('R', 54));
        actionTable.put(149, new HashMap<>());
        actionTable.get(149).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(149).put("TRUE", new Action('S', 38));
        actionTable.get(149).put("LPAREN", new Action('S', 22));
        actionTable.get(149).put("INTEGER_LITERAL", new Action('S', 43));
        actionTable.get(149).put("NEW", new Action('S', 45));
        actionTable.get(149).put("THIS", new Action('S', 44));
        actionTable.get(149).put("FALSE", new Action('S', 39));
        actionTable.get(149).put("NOT", new Action('S', 21));
        actionTable.put(73, new HashMap<>());
        actionTable.get(73).put("RPAREN", new Action('R', 60));
        actionTable.put(25, new HashMap<>());
        actionTable.get(25).put("RPAREN", new Action('R', 49));
        actionTable.get(25).put("SEMICOLON", new Action('R', 49));
        actionTable.get(25).put("RBRACKET", new Action('R', 49));
        actionTable.get(25).put("COMMA", new Action('R', 49));
        actionTable.put(148, new HashMap<>());
        actionTable.get(148).put("ELSE", new Action('R', 39));
        actionTable.get(148).put("LBRACE", new Action('R', 39));
        actionTable.get(148).put("SYSTEM_OUT_PRINTLN", new Action('R', 39));
        actionTable.get(148).put("WHILE", new Action('R', 39));
        actionTable.get(148).put("RETURN", new Action('R', 39));
        actionTable.get(148).put("IDENTIFIER", new Action('R', 39));
        actionTable.get(148).put("IF", new Action('R', 39));
        actionTable.get(148).put("RBRACE", new Action('R', 39));
        actionTable.put(154, new HashMap<>());
        actionTable.get(154).put("FALSE", new Action('S', 39));
        actionTable.get(154).put("INTEGER_LITERAL", new Action('S', 43));
        actionTable.get(154).put("NOT", new Action('S', 21));
        actionTable.get(154).put("IDENTIFIER", new Action('S', 5));
        actionTable.get(154).put("LPAREN", new Action('S', 22));
        actionTable.get(154).put("THIS", new Action('S', 44));
        actionTable.get(154).put("NEW", new Action('S', 45));
        actionTable.get(154).put("TRUE", new Action('S', 38));
        actionTable.put(115, new HashMap<>());
        actionTable.get(115).put("RPAREN", new Action('S', 122));
        actionTable.put(131, new HashMap<>());
        actionTable.get(131).put("LPAREN", new Action('S', 143));
        actionTable.put(43, new HashMap<>());
        actionTable.get(43).put("LT", new Action('R', 74));
        actionTable.get(43).put("LBRACKET", new Action('R', 74));
        actionTable.get(43).put("MINUS", new Action('R', 74));
        actionTable.get(43).put("PLUS", new Action('R', 74));
        actionTable.get(43).put("SEMICOLON", new Action('R', 74));
        actionTable.get(43).put("TIMES", new Action('R', 74));
        actionTable.get(43).put("COMMA", new Action('R', 74));
        actionTable.get(43).put("RPAREN", new Action('R', 74));
        actionTable.get(43).put("RBRACKET", new Action('R', 74));
        actionTable.get(43).put("AND", new Action('R', 74));
        actionTable.get(43).put("DOT", new Action('R', 74));
        actionTable.put(7, new HashMap<>());
        actionTable.get(7).put("STATIC", new Action('S', 8));
        actionTable.put(152, new HashMap<>());
        actionTable.get(152).put("LBRACE", new Action('R', 40));
        actionTable.get(152).put("ELSE", new Action('R', 40));
        actionTable.get(152).put("IDENTIFIER", new Action('R', 40));
        actionTable.get(152).put("RBRACE", new Action('R', 40));
        actionTable.get(152).put("SYSTEM_OUT_PRINTLN", new Action('R', 40));
        actionTable.get(152).put("WHILE", new Action('R', 40));
        actionTable.get(152).put("RETURN", new Action('R', 40));
        actionTable.get(152).put("IF", new Action('R', 40));
        actionTable.put(19, new HashMap<>());
        actionTable.get(19).put("LPAREN", new Action('S', 20));
        actionTable.put(119, new HashMap<>());
        actionTable.get(119).put("RPAREN", new Action('R', 18));
        actionTable.get(119).put("COMMA", new Action('R', 18));
        actionTable.put(10, new HashMap<>());
        actionTable.get(10).put("LPAREN", new Action('S', 11));
        gotoTable.put(111, new HashMap<>());
        gotoTable.get(111).put("Identifier", 112);
        gotoTable.put(118, new HashMap<>());
        gotoTable.get(118).put("FormalParameterRest", 119);
        gotoTable.put(1, new HashMap<>());
        gotoTable.get(1).put("TypeDeclarationList", 86);
        gotoTable.put(17, new HashMap<>());
        gotoTable.get(17).put("PrintStatement", 18);
        gotoTable.put(136, new HashMap<>());
        gotoTable.get(136).put("AllocationExpression", 26);
        gotoTable.get(136).put("NotExpression", 40);
        gotoTable.get(136).put("MinusExpression", 34);
        gotoTable.get(136).put("CompareExpression", 31);
        gotoTable.get(136).put("IntegerLiteral", 41);
        gotoTable.get(136).put("TimesExpression", 37);
        gotoTable.get(136).put("BracketExpression", 29);
        gotoTable.get(136).put("ThisExpression", 28);
        gotoTable.get(136).put("ArrayLength", 46);
        gotoTable.get(136).put("PlusExpression", 33);
        gotoTable.get(136).put("Expression", 138);
        gotoTable.get(136).put("AndExpression", 36);
        gotoTable.get(136).put("ArrayLookup", 32);
        gotoTable.get(136).put("MessageSend", 25);
        gotoTable.get(136).put("Identifier", 27);
        gotoTable.get(136).put("TrueLiteral", 35);
        gotoTable.get(136).put("ArrayAllocationExpression", 23);
        gotoTable.get(136).put("PrimaryExpression", 30);
        gotoTable.get(136).put("FalseLiteral", 24);
        gotoTable.put(145, new HashMap<>());
        gotoTable.get(145).put("WhileStatement", 129);
        gotoTable.get(145).put("Statement", 146);
        gotoTable.get(145).put("Identifier", 126);
        gotoTable.get(145).put("PrintStatement", 133);
        gotoTable.get(145).put("ArrayAssignmentStatement", 130);
        gotoTable.get(145).put("AssignmentStatement", 135);
        gotoTable.get(145).put("IfStatement", 127);
        gotoTable.get(145).put("Block", 134);
        gotoTable.put(56, new HashMap<>());
        gotoTable.get(56).put("IntegerLiteral", 41);
        gotoTable.get(56).put("ArrayAllocationExpression", 23);
        gotoTable.get(56).put("PrimaryExpression", 80);
        gotoTable.get(56).put("NotExpression", 40);
        gotoTable.get(56).put("FalseLiteral", 24);
        gotoTable.get(56).put("TrueLiteral", 35);
        gotoTable.get(56).put("ThisExpression", 28);
        gotoTable.get(56).put("Identifier", 27);
        gotoTable.get(56).put("AllocationExpression", 26);
        gotoTable.get(56).put("BracketExpression", 29);
        gotoTable.put(132, new HashMap<>());
        gotoTable.get(132).put("StatementList", 141);
        gotoTable.put(154, new HashMap<>());
        gotoTable.get(154).put("ArrayLength", 46);
        gotoTable.get(154).put("PrimaryExpression", 30);
        gotoTable.get(154).put("ArrayLookup", 32);
        gotoTable.get(154).put("ArrayAllocationExpression", 23);
        gotoTable.get(154).put("IntegerLiteral", 41);
        gotoTable.get(154).put("TrueLiteral", 35);
        gotoTable.get(154).put("ThisExpression", 28);
        gotoTable.get(154).put("BracketExpression", 29);
        gotoTable.get(154).put("MinusExpression", 34);
        gotoTable.get(154).put("AllocationExpression", 26);
        gotoTable.get(154).put("PlusExpression", 33);
        gotoTable.get(154).put("Identifier", 27);
        gotoTable.get(154).put("MessageSend", 25);
        gotoTable.get(154).put("NotExpression", 40);
        gotoTable.get(154).put("CompareExpression", 31);
        gotoTable.get(154).put("FalseLiteral", 24);
        gotoTable.get(154).put("AndExpression", 36);
        gotoTable.get(154).put("TimesExpression", 37);
        gotoTable.get(154).put("Expression", 155);
        gotoTable.put(164, new HashMap<>());
        gotoTable.get(164).put("VarDeclarationList", 165);
        gotoTable.put(60, new HashMap<>());
        gotoTable.get(60).put("AllocationExpression", 26);
        gotoTable.get(60).put("PrimaryExpression", 65);
        gotoTable.get(60).put("NotExpression", 40);
        gotoTable.get(60).put("Identifier", 27);
        gotoTable.get(60).put("TrueLiteral", 35);
        gotoTable.get(60).put("ArrayAllocationExpression", 23);
        gotoTable.get(60).put("IntegerLiteral", 41);
        gotoTable.get(60).put("FalseLiteral", 24);
        gotoTable.get(60).put("ThisExpression", 28);
        gotoTable.get(60).put("BracketExpression", 29);
        gotoTable.put(59, new HashMap<>());
        gotoTable.get(59).put("TrueLiteral", 35);
        gotoTable.get(59).put("NotExpression", 40);
        gotoTable.get(59).put("BracketExpression", 29);
        gotoTable.get(59).put("AllocationExpression", 26);
        gotoTable.get(59).put("IntegerLiteral", 41);
        gotoTable.get(59).put("PrimaryExpression", 66);
        gotoTable.get(59).put("ArrayAllocationExpression", 23);
        gotoTable.get(59).put("Identifier", 27);
        gotoTable.get(59).put("ThisExpression", 28);
        gotoTable.get(59).put("FalseLiteral", 24);
        gotoTable.put(93, new HashMap<>());
        gotoTable.get(93).put("VarDeclarationList", 94);
        gotoTable.put(0, new HashMap<>());
        gotoTable.get(0).put("MainClass", 1);
        gotoTable.get(0).put("Goal", 3);
        gotoTable.put(21, new HashMap<>());
        gotoTable.get(21).put("ThisExpression", 28);
        gotoTable.get(21).put("PrimaryExpression", 83);
        gotoTable.get(21).put("TrueLiteral", 35);
        gotoTable.get(21).put("Identifier", 27);
        gotoTable.get(21).put("IntegerLiteral", 41);
        gotoTable.get(21).put("AllocationExpression", 26);
        gotoTable.get(21).put("NotExpression", 40);
        gotoTable.get(21).put("ArrayAllocationExpression", 23);
        gotoTable.get(21).put("FalseLiteral", 24);
        gotoTable.get(21).put("BracketExpression", 29);
        gotoTable.put(51, new HashMap<>());
        gotoTable.get(51).put("ArrayLength", 46);
        gotoTable.get(51).put("AndExpression", 36);
        gotoTable.get(51).put("IntegerLiteral", 41);
        gotoTable.get(51).put("TrueLiteral", 35);
        gotoTable.get(51).put("ArrayAllocationExpression", 23);
        gotoTable.get(51).put("Identifier", 27);
        gotoTable.get(51).put("Expression", 52);
        gotoTable.get(51).put("AllocationExpression", 26);
        gotoTable.get(51).put("ArrayLookup", 32);
        gotoTable.get(51).put("MinusExpression", 34);
        gotoTable.get(51).put("ThisExpression", 28);
        gotoTable.get(51).put("PrimaryExpression", 30);
        gotoTable.get(51).put("NotExpression", 40);
        gotoTable.get(51).put("BracketExpression", 29);
        gotoTable.get(51).put("MessageSend", 25);
        gotoTable.get(51).put("CompareExpression", 31);
        gotoTable.get(51).put("TimesExpression", 37);
        gotoTable.get(51).put("FalseLiteral", 24);
        gotoTable.get(51).put("PlusExpression", 33);
        gotoTable.put(86, new HashMap<>());
        gotoTable.get(86).put("ClassDeclaration", 87);
        gotoTable.get(86).put("ClassExtendsDeclaration", 90);
        gotoTable.get(86).put("TypeDeclaration", 89);
        gotoTable.put(88, new HashMap<>());
        gotoTable.get(88).put("Identifier", 91);
        gotoTable.put(110, new HashMap<>());
        gotoTable.get(110).put("BooleanType", 97);
        gotoTable.get(110).put("IntegerType", 100);
        gotoTable.get(110).put("Identifier", 99);
        gotoTable.get(110).put("ArrayType", 102);
        gotoTable.get(110).put("Type", 111);
        gotoTable.put(125, new HashMap<>());
        gotoTable.get(125).put("Statement", 137);
        gotoTable.get(125).put("IfStatement", 127);
        gotoTable.get(125).put("ArrayAssignmentStatement", 130);
        gotoTable.get(125).put("WhileStatement", 129);
        gotoTable.get(125).put("Identifier", 126);
        gotoTable.get(125).put("AssignmentStatement", 135);
        gotoTable.get(125).put("PrintStatement", 133);
        gotoTable.get(125).put("Block", 134);
        gotoTable.put(74, new HashMap<>());
        gotoTable.get(74).put("ExpressionRest", 76);
        gotoTable.put(151, new HashMap<>());
        gotoTable.get(151).put("AssignmentStatement", 135);
        gotoTable.get(151).put("ArrayAssignmentStatement", 130);
        gotoTable.get(151).put("Block", 134);
        gotoTable.get(151).put("WhileStatement", 129);
        gotoTable.get(151).put("Identifier", 126);
        gotoTable.get(151).put("PrintStatement", 133);
        gotoTable.get(151).put("Statement", 152);
        gotoTable.get(151).put("IfStatement", 127);
        gotoTable.put(165, new HashMap<>());
        gotoTable.get(165).put("Identifier", 99);
        gotoTable.get(165).put("ArrayType", 102);
        gotoTable.get(165).put("Type", 101);
        gotoTable.get(165).put("MethodDeclarationList", 166);
        gotoTable.get(165).put("IntegerType", 100);
        gotoTable.get(165).put("BooleanType", 97);
        gotoTable.get(165).put("VarDeclaration", 96);
        gotoTable.put(166, new HashMap<>());
        gotoTable.get(166).put("MethodDeclaration", 108);
        gotoTable.put(61, new HashMap<>());
        gotoTable.get(61).put("ThisExpression", 28);
        gotoTable.get(61).put("BracketExpression", 29);
        gotoTable.get(61).put("AllocationExpression", 26);
        gotoTable.get(61).put("PrimaryExpression", 64);
        gotoTable.get(61).put("NotExpression", 40);
        gotoTable.get(61).put("TrueLiteral", 35);
        gotoTable.get(61).put("FalseLiteral", 24);
        gotoTable.get(61).put("IntegerLiteral", 41);
        gotoTable.get(61).put("ArrayAllocationExpression", 23);
        gotoTable.get(61).put("Identifier", 27);
        gotoTable.put(58, new HashMap<>());
        gotoTable.get(58).put("Identifier", 68);
        gotoTable.put(120, new HashMap<>());
        gotoTable.get(120).put("IntegerType", 100);
        gotoTable.get(120).put("BooleanType", 97);
        gotoTable.get(120).put("Type", 114);
        gotoTable.get(120).put("Identifier", 99);
        gotoTable.get(120).put("FormalParameter", 121);
        gotoTable.get(120).put("ArrayType", 102);
        gotoTable.put(113, new HashMap<>());
        gotoTable.get(113).put("BooleanType", 97);
        gotoTable.get(113).put("FormalParameterList", 116);
        gotoTable.get(113).put("IntegerType", 100);
        gotoTable.get(113).put("Type", 114);
        gotoTable.get(113).put("FormalParameterListOpt", 115);
        gotoTable.get(113).put("FormalParameter", 117);
        gotoTable.get(113).put("ArrayType", 102);
        gotoTable.get(113).put("Identifier", 99);
        gotoTable.put(57, new HashMap<>());
        gotoTable.get(57).put("IntegerLiteral", 41);
        gotoTable.get(57).put("FalseLiteral", 24);
        gotoTable.get(57).put("ThisExpression", 28);
        gotoTable.get(57).put("TrueLiteral", 35);
        gotoTable.get(57).put("BracketExpression", 29);
        gotoTable.get(57).put("AllocationExpression", 26);
        gotoTable.get(57).put("PrimaryExpression", 79);
        gotoTable.get(57).put("NotExpression", 40);
        gotoTable.get(57).put("ArrayAllocationExpression", 23);
        gotoTable.get(57).put("Identifier", 27);
        gotoTable.put(2, new HashMap<>());
        gotoTable.get(2).put("Identifier", 4);
        gotoTable.put(117, new HashMap<>());
        gotoTable.get(117).put("FormalParameterRestList", 118);
        gotoTable.put(14, new HashMap<>());
        gotoTable.get(14).put("Identifier", 15);
        gotoTable.put(95, new HashMap<>());
        gotoTable.get(95).put("MethodDeclaration", 108);
        gotoTable.put(143, new HashMap<>());
        gotoTable.get(143).put("ArrayLength", 46);
        gotoTable.get(143).put("AndExpression", 36);
        gotoTable.get(143).put("Identifier", 27);
        gotoTable.get(143).put("MinusExpression", 34);
        gotoTable.get(143).put("TrueLiteral", 35);
        gotoTable.get(143).put("ArrayAllocationExpression", 23);
        gotoTable.get(143).put("FalseLiteral", 24);
        gotoTable.get(143).put("Expression", 144);
        gotoTable.get(143).put("MessageSend", 25);
        gotoTable.get(143).put("CompareExpression", 31);
        gotoTable.get(143).put("ArrayLookup", 32);
        gotoTable.get(143).put("NotExpression", 40);
        gotoTable.get(143).put("PrimaryExpression", 30);
        gotoTable.get(143).put("AllocationExpression", 26);
        gotoTable.get(143).put("IntegerLiteral", 41);
        gotoTable.get(143).put("TimesExpression", 37);
        gotoTable.get(143).put("PlusExpression", 33);
        gotoTable.get(143).put("BracketExpression", 29);
        gotoTable.get(143).put("ThisExpression", 28);
        gotoTable.put(45, new HashMap<>());
        gotoTable.get(45).put("Identifier", 48);
        gotoTable.put(70, new HashMap<>());
        gotoTable.get(70).put("ArrayLookup", 32);
        gotoTable.get(70).put("FalseLiteral", 24);
        gotoTable.get(70).put("CompareExpression", 31);
        gotoTable.get(70).put("Identifier", 27);
        gotoTable.get(70).put("MinusExpression", 34);
        gotoTable.get(70).put("ThisExpression", 28);
        gotoTable.get(70).put("AllocationExpression", 26);
        gotoTable.get(70).put("PlusExpression", 33);
        gotoTable.get(70).put("AndExpression", 36);
        gotoTable.get(70).put("BracketExpression", 29);
        gotoTable.get(70).put("ExpressionList", 73);
        gotoTable.get(70).put("ArrayAllocationExpression", 23);
        gotoTable.get(70).put("IntegerLiteral", 41);
        gotoTable.get(70).put("MessageSend", 25);
        gotoTable.get(70).put("ExpressionListOpt", 71);
        gotoTable.get(70).put("ArrayLength", 46);
        gotoTable.get(70).put("NotExpression", 40);
        gotoTable.get(70).put("Expression", 72);
        gotoTable.get(70).put("TrueLiteral", 35);
        gotoTable.get(70).put("PrimaryExpression", 30);
        gotoTable.get(70).put("TimesExpression", 37);
        gotoTable.put(20, new HashMap<>());
        gotoTable.get(20).put("TrueLiteral", 35);
        gotoTable.get(20).put("NotExpression", 40);
        gotoTable.get(20).put("CompareExpression", 31);
        gotoTable.get(20).put("MinusExpression", 34);
        gotoTable.get(20).put("AllocationExpression", 26);
        gotoTable.get(20).put("BracketExpression", 29);
        gotoTable.get(20).put("ThisExpression", 28);
        gotoTable.get(20).put("TimesExpression", 37);
        gotoTable.get(20).put("ArrayLookup", 32);
        gotoTable.get(20).put("Expression", 42);
        gotoTable.get(20).put("Identifier", 27);
        gotoTable.get(20).put("FalseLiteral", 24);
        gotoTable.get(20).put("ArrayAllocationExpression", 23);
        gotoTable.get(20).put("PlusExpression", 33);
        gotoTable.get(20).put("ArrayLength", 46);
        gotoTable.get(20).put("MessageSend", 25);
        gotoTable.get(20).put("IntegerLiteral", 41);
        gotoTable.get(20).put("AndExpression", 36);
        gotoTable.get(20).put("PrimaryExpression", 30);
        gotoTable.put(92, new HashMap<>());
        gotoTable.get(92).put("Identifier", 163);
        gotoTable.put(101, new HashMap<>());
        gotoTable.get(101).put("Identifier", 104);
        gotoTable.put(114, new HashMap<>());
        gotoTable.get(114).put("Identifier", 162);
        gotoTable.put(123, new HashMap<>());
        gotoTable.get(123).put("VarDeclarationList", 124);
        gotoTable.put(75, new HashMap<>());
        gotoTable.get(75).put("NotExpression", 40);
        gotoTable.get(75).put("IntegerLiteral", 41);
        gotoTable.get(75).put("TimesExpression", 37);
        gotoTable.get(75).put("CompareExpression", 31);
        gotoTable.get(75).put("ArrayLookup", 32);
        gotoTable.get(75).put("AllocationExpression", 26);
        gotoTable.get(75).put("MessageSend", 25);
        gotoTable.get(75).put("PrimaryExpression", 30);
        gotoTable.get(75).put("BracketExpression", 29);
        gotoTable.get(75).put("ThisExpression", 28);
        gotoTable.get(75).put("ArrayLength", 46);
        gotoTable.get(75).put("Identifier", 27);
        gotoTable.get(75).put("MinusExpression", 34);
        gotoTable.get(75).put("Expression", 77);
        gotoTable.get(75).put("ArrayAllocationExpression", 23);
        gotoTable.get(75).put("AndExpression", 36);
        gotoTable.get(75).put("FalseLiteral", 24);
        gotoTable.get(75).put("TrueLiteral", 35);
        gotoTable.get(75).put("PlusExpression", 33);
        gotoTable.put(149, new HashMap<>());
        gotoTable.get(149).put("MinusExpression", 34);
        gotoTable.get(149).put("BracketExpression", 29);
        gotoTable.get(149).put("TrueLiteral", 35);
        gotoTable.get(149).put("ArrayLength", 46);
        gotoTable.get(149).put("AndExpression", 36);
        gotoTable.get(149).put("PlusExpression", 33);
        gotoTable.get(149).put("TimesExpression", 37);
        gotoTable.get(149).put("PrimaryExpression", 30);
        gotoTable.get(149).put("Identifier", 27);
        gotoTable.get(149).put("IntegerLiteral", 41);
        gotoTable.get(149).put("MessageSend", 25);
        gotoTable.get(149).put("ThisExpression", 28);
        gotoTable.get(149).put("CompareExpression", 31);
        gotoTable.get(149).put("ArrayAllocationExpression", 23);
        gotoTable.get(149).put("ArrayLookup", 32);
        gotoTable.get(149).put("AllocationExpression", 26);
        gotoTable.get(149).put("FalseLiteral", 24);
        gotoTable.get(149).put("NotExpression", 40);
        gotoTable.get(149).put("Expression", 150);
        gotoTable.put(153, new HashMap<>());
        gotoTable.get(153).put("PrimaryExpression", 30);
        gotoTable.get(153).put("IntegerLiteral", 41);
        gotoTable.get(153).put("TimesExpression", 37);
        gotoTable.get(153).put("ArrayLookup", 32);
        gotoTable.get(153).put("MessageSend", 25);
        gotoTable.get(153).put("CompareExpression", 31);
        gotoTable.get(153).put("NotExpression", 40);
        gotoTable.get(153).put("TrueLiteral", 35);
        gotoTable.get(153).put("FalseLiteral", 24);
        gotoTable.get(153).put("MinusExpression", 34);
        gotoTable.get(153).put("Expression", 157);
        gotoTable.get(153).put("BracketExpression", 29);
        gotoTable.get(153).put("PlusExpression", 33);
        gotoTable.get(153).put("ArrayAllocationExpression", 23);
        gotoTable.get(153).put("ThisExpression", 28);
        gotoTable.get(153).put("AndExpression", 36);
        gotoTable.get(153).put("Identifier", 27);
        gotoTable.get(153).put("ArrayLength", 46);
        gotoTable.get(153).put("AllocationExpression", 26);
        gotoTable.put(94, new HashMap<>());
        gotoTable.get(94).put("BooleanType", 97);
        gotoTable.get(94).put("MethodDeclarationList", 95);
        gotoTable.get(94).put("VarDeclaration", 96);
        gotoTable.get(94).put("IntegerType", 100);
        gotoTable.get(94).put("Identifier", 99);
        gotoTable.get(94).put("ArrayType", 102);
        gotoTable.get(94).put("Type", 101);
        gotoTable.put(124, new HashMap<>());
        gotoTable.get(124).put("Type", 101);
        gotoTable.get(124).put("StatementList", 125);
        gotoTable.get(124).put("VarDeclaration", 96);
        gotoTable.get(124).put("Identifier", 99);
        gotoTable.get(124).put("ArrayType", 102);
        gotoTable.get(124).put("IntegerType", 100);
        gotoTable.get(124).put("BooleanType", 97);
        gotoTable.put(141, new HashMap<>());
        gotoTable.get(141).put("IfStatement", 127);
        gotoTable.get(141).put("WhileStatement", 129);
        gotoTable.get(141).put("AssignmentStatement", 135);
        gotoTable.get(141).put("Statement", 137);
        gotoTable.get(141).put("ArrayAssignmentStatement", 130);
        gotoTable.get(141).put("PrintStatement", 133);
        gotoTable.get(141).put("Block", 134);
        gotoTable.get(141).put("Identifier", 126);
        gotoTable.put(147, new HashMap<>());
        gotoTable.get(147).put("Block", 134);
        gotoTable.get(147).put("WhileStatement", 129);
        gotoTable.get(147).put("IfStatement", 127);
        gotoTable.get(147).put("Statement", 148);
        gotoTable.get(147).put("PrintStatement", 133);
        gotoTable.get(147).put("Identifier", 126);
        gotoTable.get(147).put("ArrayAssignmentStatement", 130);
        gotoTable.get(147).put("AssignmentStatement", 135);
        gotoTable.put(72, new HashMap<>());
        gotoTable.get(72).put("ExpressionRestList", 74);
        gotoTable.put(159, new HashMap<>());
        gotoTable.get(159).put("ArrayAllocationExpression", 23);
        gotoTable.get(159).put("IntegerLiteral", 41);
        gotoTable.get(159).put("MinusExpression", 34);
        gotoTable.get(159).put("AllocationExpression", 26);
        gotoTable.get(159).put("FalseLiteral", 24);
        gotoTable.get(159).put("PrimaryExpression", 30);
        gotoTable.get(159).put("ArrayLookup", 32);
        gotoTable.get(159).put("MessageSend", 25);
        gotoTable.get(159).put("ThisExpression", 28);
        gotoTable.get(159).put("AndExpression", 36);
        gotoTable.get(159).put("CompareExpression", 31);
        gotoTable.get(159).put("Identifier", 27);
        gotoTable.get(159).put("TrueLiteral", 35);
        gotoTable.get(159).put("NotExpression", 40);
        gotoTable.get(159).put("TimesExpression", 37);
        gotoTable.get(159).put("ArrayLength", 46);
        gotoTable.get(159).put("Expression", 160);
        gotoTable.get(159).put("PlusExpression", 33);
        gotoTable.get(159).put("BracketExpression", 29);
        gotoTable.put(22, new HashMap<>());
        gotoTable.get(22).put("MessageSend", 25);
        gotoTable.get(22).put("ArrayLength", 46);
        gotoTable.get(22).put("CompareExpression", 31);
        gotoTable.get(22).put("AllocationExpression", 26);
        gotoTable.get(22).put("FalseLiteral", 24);
        gotoTable.get(22).put("TimesExpression", 37);
        gotoTable.get(22).put("IntegerLiteral", 41);
        gotoTable.get(22).put("AndExpression", 36);
        gotoTable.get(22).put("Identifier", 27);
        gotoTable.get(22).put("Expression", 81);
        gotoTable.get(22).put("ArrayLookup", 32);
        gotoTable.get(22).put("MinusExpression", 34);
        gotoTable.get(22).put("PlusExpression", 33);
        gotoTable.get(22).put("BracketExpression", 29);
        gotoTable.get(22).put("ArrayAllocationExpression", 23);
        gotoTable.get(22).put("TrueLiteral", 35);
        gotoTable.get(22).put("ThisExpression", 28);
        gotoTable.get(22).put("NotExpression", 40);
        gotoTable.get(22).put("PrimaryExpression", 30);
        gotoTable.put(62, new HashMap<>());
        gotoTable.get(62).put("IntegerLiteral", 41);
        gotoTable.get(62).put("TrueLiteral", 35);
        gotoTable.get(62).put("AllocationExpression", 26);
        gotoTable.get(62).put("PrimaryExpression", 63);
        gotoTable.get(62).put("Identifier", 27);
        gotoTable.get(62).put("ThisExpression", 28);
        gotoTable.get(62).put("BracketExpression", 29);
        gotoTable.get(62).put("FalseLiteral", 24);
        gotoTable.get(62).put("NotExpression", 40);
        gotoTable.get(62).put("ArrayAllocationExpression", 23);
        rules = new int[][] {
            { 0, 2 }, // Goal -> ...
            { 0, 0 }, // TypeDeclarationList -> ...
            { 0, 2 }, // TypeDeclarationList -> ...
            { 0, 17 }, // MainClass -> ...
            { 0, 1 }, // TypeDeclaration -> ...
            { 0, 1 }, // TypeDeclaration -> ...
            { 0, 6 }, // ClassDeclaration -> ...
            { 0, 8 }, // ClassExtendsDeclaration -> ...
            { 0, 0 }, // VarDeclarationList -> ...
            { 0, 2 }, // VarDeclarationList -> ...
            { 0, 0 }, // MethodDeclarationList -> ...
            { 0, 2 }, // MethodDeclarationList -> ...
            { 0, 3 }, // VarDeclaration -> ...
            { 0, 13 }, // MethodDeclaration -> ...
            { 0, 0 }, // FormalParameterListOpt -> ...
            { 0, 1 }, // FormalParameterListOpt -> ...
            { 0, 2 }, // FormalParameterList -> ...
            { 0, 0 }, // FormalParameterRestList -> ...
            { 0, 2 }, // FormalParameterRestList -> ...
            { 0, 2 }, // FormalParameter -> ...
            { 0, 2 }, // FormalParameterRest -> ...
            { 0, 1 }, // Type -> ...
            { 0, 1 }, // Type -> ...
            { 0, 1 }, // Type -> ...
            { 0, 1 }, // Type -> ...
            { 0, 3 }, // ArrayType -> ...
            { 0, 1 }, // BooleanType -> ...
            { 0, 1 }, // IntegerType -> ...
            { 0, 0 }, // StatementList -> ...
            { 0, 2 }, // StatementList -> ...
            { 0, 1 }, // Statement -> ...
            { 0, 1 }, // Statement -> ...
            { 0, 1 }, // Statement -> ...
            { 0, 1 }, // Statement -> ...
            { 0, 1 }, // Statement -> ...
            { 0, 1 }, // Statement -> ...
            { 0, 3 }, // Block -> ...
            { 0, 4 }, // AssignmentStatement -> ...
            { 0, 7 }, // ArrayAssignmentStatement -> ...
            { 0, 7 }, // IfStatement -> ...
            { 0, 5 }, // WhileStatement -> ...
            { 0, 5 }, // PrintStatement -> ...
            { 0, 1 }, // Expression -> ...
            { 0, 1 }, // Expression -> ...
            { 0, 1 }, // Expression -> ...
            { 0, 1 }, // Expression -> ...
            { 0, 1 }, // Expression -> ...
            { 0, 1 }, // Expression -> ...
            { 0, 1 }, // Expression -> ...
            { 0, 1 }, // Expression -> ...
            { 0, 1 }, // Expression -> ...
            { 0, 3 }, // AndExpression -> ...
            { 0, 3 }, // CompareExpression -> ...
            { 0, 3 }, // PlusExpression -> ...
            { 0, 3 }, // MinusExpression -> ...
            { 0, 3 }, // TimesExpression -> ...
            { 0, 4 }, // ArrayLookup -> ...
            { 0, 3 }, // ArrayLength -> ...
            { 0, 6 }, // MessageSend -> ...
            { 0, 0 }, // ExpressionListOpt -> ...
            { 0, 1 }, // ExpressionListOpt -> ...
            { 0, 2 }, // ExpressionList -> ...
            { 0, 0 }, // ExpressionRestList -> ...
            { 0, 2 }, // ExpressionRestList -> ...
            { 0, 2 }, // ExpressionRest -> ...
            { 0, 1 }, // PrimaryExpression -> ...
            { 0, 1 }, // PrimaryExpression -> ...
            { 0, 1 }, // PrimaryExpression -> ...
            { 0, 1 }, // PrimaryExpression -> ...
            { 0, 1 }, // PrimaryExpression -> ...
            { 0, 1 }, // PrimaryExpression -> ...
            { 0, 1 }, // PrimaryExpression -> ...
            { 0, 1 }, // PrimaryExpression -> ...
            { 0, 1 }, // PrimaryExpression -> ...
            { 0, 1 }, // IntegerLiteral -> ...
            { 0, 1 }, // TrueLiteral -> ...
            { 0, 1 }, // FalseLiteral -> ...
            { 0, 1 }, // Identifier -> ...
            { 0, 1 }, // ThisExpression -> ...
            { 0, 5 }, // ArrayAllocationExpression -> ...
            { 0, 4 }, // AllocationExpression -> ...
            { 0, 2 }, // NotExpression -> ...
            { 0, 3 }, // BracketExpression -> ...
        };
    }

    // Helper to get LHS string by rule index
    private String getLhs(int ruleId) {
        switch(ruleId) {
            case 0: return "Goal";
            case 1: return "TypeDeclarationList";
            case 2: return "TypeDeclarationList";
            case 3: return "MainClass";
            case 4: return "TypeDeclaration";
            case 5: return "TypeDeclaration";
            case 6: return "ClassDeclaration";
            case 7: return "ClassExtendsDeclaration";
            case 8: return "VarDeclarationList";
            case 9: return "VarDeclarationList";
            case 10: return "MethodDeclarationList";
            case 11: return "MethodDeclarationList";
            case 12: return "VarDeclaration";
            case 13: return "MethodDeclaration";
            case 14: return "FormalParameterListOpt";
            case 15: return "FormalParameterListOpt";
            case 16: return "FormalParameterList";
            case 17: return "FormalParameterRestList";
            case 18: return "FormalParameterRestList";
            case 19: return "FormalParameter";
            case 20: return "FormalParameterRest";
            case 21: return "Type";
            case 22: return "Type";
            case 23: return "Type";
            case 24: return "Type";
            case 25: return "ArrayType";
            case 26: return "BooleanType";
            case 27: return "IntegerType";
            case 28: return "StatementList";
            case 29: return "StatementList";
            case 30: return "Statement";
            case 31: return "Statement";
            case 32: return "Statement";
            case 33: return "Statement";
            case 34: return "Statement";
            case 35: return "Statement";
            case 36: return "Block";
            case 37: return "AssignmentStatement";
            case 38: return "ArrayAssignmentStatement";
            case 39: return "IfStatement";
            case 40: return "WhileStatement";
            case 41: return "PrintStatement";
            case 42: return "Expression";
            case 43: return "Expression";
            case 44: return "Expression";
            case 45: return "Expression";
            case 46: return "Expression";
            case 47: return "Expression";
            case 48: return "Expression";
            case 49: return "Expression";
            case 50: return "Expression";
            case 51: return "AndExpression";
            case 52: return "CompareExpression";
            case 53: return "PlusExpression";
            case 54: return "MinusExpression";
            case 55: return "TimesExpression";
            case 56: return "ArrayLookup";
            case 57: return "ArrayLength";
            case 58: return "MessageSend";
            case 59: return "ExpressionListOpt";
            case 60: return "ExpressionListOpt";
            case 61: return "ExpressionList";
            case 62: return "ExpressionRestList";
            case 63: return "ExpressionRestList";
            case 64: return "ExpressionRest";
            case 65: return "PrimaryExpression";
            case 66: return "PrimaryExpression";
            case 67: return "PrimaryExpression";
            case 68: return "PrimaryExpression";
            case 69: return "PrimaryExpression";
            case 70: return "PrimaryExpression";
            case 71: return "PrimaryExpression";
            case 72: return "PrimaryExpression";
            case 73: return "PrimaryExpression";
            case 74: return "IntegerLiteral";
            case 75: return "TrueLiteral";
            case 76: return "FalseLiteral";
            case 77: return "Identifier";
            case 78: return "ThisExpression";
            case 79: return "ArrayAllocationExpression";
            case 80: return "AllocationExpression";
            case 81: return "NotExpression";
            case 82: return "BracketExpression";
            default: return "";
        }
    }

    public int parse() {
        if (defaultLexer == null) {
            throw new IllegalStateException("No lexer provided. Use Parser(Lexer) or parse(Lexer).");
        }
        return parse(defaultLexer);
    }

    public int parse(Lexer lexer) {
        this.defaultLexer = lexer;
        Stack<Integer> stack = new Stack<>();
        Stack<Integer> valueStack = new Stack<>();
        stack.push(0);
        valueStack.push(0);
        Token token = lexer.nextToken();
        String sym = token.type;
        
        while (true) {
            int state = stack.peek();
            if (!actionTable.containsKey(state) || !actionTable.get(state).containsKey(sym)) {
                throw new RuntimeException("Syntax Error");
            }
            
            Action act = actionTable.get(state).get(sym);
            if (act.type == 'S') { // Shift
                stack.push(act.param);
                valueStack.push(token.value);
                token = lexer.nextToken();
                sym = token.type;
            } else if (act.type == 'R') { // Reduce
                int ruleId = act.param;
                int len = rules[ruleId][1];
                String lhs = getLhs(ruleId);
                
                // Get values from stack for semantic action
                int[] vals = new int[len];
                for (int i = len - 1; i >= 0; i--) {
                    vals[i] = valueStack.peek();
                    valueStack.pop();
                    stack.pop();
                }
                int result = (len > 0) ? vals[0] : 0;
                
                // Semantic Actions
                switch (ruleId) {
                    case 0:
                        System.out.printf("Parsed Goal\n");
                        break;
                    case 3:
                        System.out.printf("Parsed MainClass\n");
                        break;
                    case 6:
                        System.out.printf("Parsed ClassDeclaration\n");
                        break;
                    case 7:
                        System.out.printf("Parsed ClassExtendsDeclaration\n");
                        break;
                    case 12:
                        System.out.printf("Parsed VarDeclaration\n");
                        break;
                    case 13:
                        System.out.printf("Parsed MethodDeclaration\n");
                        break;
                }
                
                int top = stack.peek();
                if (gotoTable.containsKey(top) && gotoTable.get(top).containsKey(lhs)) {
                    stack.push(gotoTable.get(top).get(lhs));
                    valueStack.push(result);
                }
            } else if (act.type == 'A') {
                return valueStack.peek();
            }
        }
    }
    
    // Token and Lexer interfaces (implement or use generated lexer)
    public interface Lexer {
        Token nextToken();
    }
    
    public static class Token {
        public String type;
        public int value;
        public String text;
        public Token(String type, int value) {
            this.type = type;
            this.value = value;
            this.text = String.valueOf(value);
        }
        public Token(String type, String text) {
            this.type = type;
            this.text = text;
            try { this.value = Integer.parseInt(text); } catch (Exception e) { this.value = 0; }
        }
    }
    
    /* =========================================================================
     * Combined Lexer + Parser Test Driver
     * 
     * To use with generated Lexer.java:
     *   1. Generate lexer: openlexer gen-lexer -l grammar.l -L java -o output/
     *   2. Generate parser: openlexer gen-parser --parser grammar.y -L java -o output/
     *   3. Compile: javac Lexer.java Parser.java
     *   4. Run: java Parser "3 + 4 * 2"
     * ========================================================================= */
    
    /** Adapter to use generated Lexer with Parser. */
    public static class LexerAdapter implements Lexer {
        private Object lexer;
        private java.lang.reflect.Method nextTokenMethod;
        
        public LexerAdapter(Object lexer) {
            this.lexer = lexer;
            try {
                this.nextTokenMethod = lexer.getClass().getMethod("nextToken");
            } catch (Exception e) {
                throw new RuntimeException("Lexer must have nextToken() method", e);
            }
        }
        
        @Override
        public Token nextToken() {
            try {
                Object tok = nextTokenMethod.invoke(lexer);
                // Get type and text via reflection
                Object typeObj = tok.getClass().getField("type").get(tok);
                String type = typeObj.toString();
                if (type.equals("EOF")) type = "$";
                String text = (String) tok.getClass().getField("text").get(tok);
                return new Token(type, text);
            } catch (Exception e) {
                throw new RuntimeException("Failed to get next token", e);
            }
        }
    }
    
    /** Test parsing an expression. */
    public static void testParse(String expr) {
        System.out.println("Parsing: \"" + expr + "\"");
        try {
            // Try to load and use generated Lexer
            Class<?> lexerClass = Class.forName("Lexer");
            Object lexer = lexerClass.getConstructor(String.class).newInstance(expr);
            LexerAdapter adapter = new LexerAdapter(lexer);
            Parser parser = new Parser(adapter);
            int result = parser.parse();
            System.out.println("  Result: " + result);
        } catch (ClassNotFoundException e) {
            System.err.println("  Error: Lexer.class not found. Generate and compile Lexer.java first.");
        } catch (Exception e) {
            System.err.println("  Error: " + e.getMessage());
        }
    }
    
    public static void main(String[] args) {
        System.out.println("=== OpenLexer Parser Test ===");
        System.out.println();
        
        if (args.length > 0) {
            for (String arg : args) {
                testParse(arg);
            }
        } else {
            try {
                java.util.Scanner sc = new java.util.Scanner(System.in);
                boolean hasInput = false;
                while (sc.hasNextLine()) {
                    String line = sc.nextLine().trim();
                    if (!line.isEmpty()) { testParse(line); hasInput = true; }
                }
                if (!hasInput) {
                    testParse("3 + 4");
                    testParse("3 + 4 * 2");
                }
            } catch (Exception e) {
                testParse("3 + 4 * 2");
            }
        }
    }
}
