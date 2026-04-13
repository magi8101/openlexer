pub struct ErrorRecoveryHandler;

impl ErrorRecoveryHandler {
    pub fn translate_yyerrok_c() -> &'static str {
        "yyerrstatus = 0;"
    }

    pub fn translate_yyerrok_java() -> &'static str {
        "yyerrstatus = 0;"
    }

    pub fn translate_yyerrok_python() -> &'static str {
        "pass  # Error recovery reset"
    }

    pub fn translate_yyclearin_c() -> &'static str {
        "yychar = -1;"
    }

    pub fn translate_yyclearin_java() -> &'static str {
        "yychar = -1;"
    }

    pub fn translate_yyclearin_python() -> &'static str {
        "pass  # Clear lookahead"
    }

    pub fn should_skip_error_handling(action: &str) -> bool {
        action.contains("yyerrok") || action.contains("yyclearin")
    }

    pub fn replace_error_handling_c(action: &str) -> String {
        let s = action.replace("yyerrok;", Self::translate_yyerrok_c());
        s.replace("yyclearin;", Self::translate_yyclearin_c())
    }

    pub fn replace_error_handling_java(action: &str) -> String {
        let s = action.replace("yyerrok;", Self::translate_yyerrok_java());
        s.replace("yyclearin;", Self::translate_yyclearin_java())
    }

    pub fn replace_error_handling_python(action: &str) -> String {
        let s = action.replace("yyerrok;", Self::translate_yyerrok_python());
        s.replace("yyclearin;", Self::translate_yyclearin_python())
    }

    pub fn has_error_recovery(action: &str) -> bool {
        action.contains("yyerror") || action.contains("yyerrok") || action.contains("YYERROR")
    }
}
