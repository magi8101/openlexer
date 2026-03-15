//! OpenLexer GUI - Modern egui-based interface for lexer/parser generation
//! Compiles to both native desktop and WASM for web browsers
//!
//! Features:
//! - Lexer generation with start conditions and Unicode support
//! - Parser generation with LALR(1) and GLR support
//! - Combined lexer+parser generation
//! - Multi-language output (Python, C, Java)
//! - Live code preview with syntax highlighting
//! - Test runner for generated code

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use openlexer_lib::{lexgen, parsegen};
use openlexer_lib::lexgen::dfa::Dfa;
use openlexer_lib::lexgen::nfa::Nfa;
use openlexer_lib::lexgen::rules::LexerSpec;
use openlexer_lib::parsegen::grammar::Grammar;
use openlexer_lib::parsegen::lalr::ParsingTable;
use openlexer_lib::parsegen::codegen as parser_codegen;
use openlexer_lib::debug::{LexerDebugger, ParserDebugger, LexerDebugStep, ParserDebugStep};

// Version info
const VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_NAME: &str = "OpenLexer";

// Web-specific imports for download functionality
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

// JS functions for in-browser code execution (defined in index.html)
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    fn startCodeRun(language: &str, code: &str, testInput: &str, extraCode: &str) -> i32;
    fn isRunDone(id: i32) -> bool;
    fn getRunResult(id: i32) -> String;
}

#[cfg(target_arch = "wasm32")]
mod web_utils {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

    pub fn download_file(filename: &str, content: &str) {
        let window = web_sys::window().expect("no window");
        let document = window.document().expect("no document");

        // Create blob from content
        let blob_parts = js_sys::Array::new();
        blob_parts.push(&JsValue::from_str(content));

        let options = BlobPropertyBag::new();
        options.set_type("text/plain;charset=utf-8");

        let blob = Blob::new_with_str_sequence_and_options(&blob_parts, &options)
            .expect("failed to create blob");

        // Create object URL
        let url = Url::create_object_url_with_blob(&blob).expect("failed to create object URL");

        // Create anchor element and trigger download
        let anchor: HtmlAnchorElement = document
            .create_element("a")
            .expect("failed to create anchor")
            .dyn_into()
            .expect("failed to cast to anchor");

        anchor.set_href(&url);
        anchor.set_download(filename);
        anchor.click();

        // Cleanup
        let _ = Url::revoke_object_url(&url);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title(format!("{} v{}", APP_NAME, VERSION)),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|cc| Ok(Box::new(OpenLexerApp::new(cc)))),
    )
}



// ============================================================================
// Enums and Types
// ============================================================================

#[derive(Default, PartialEq, Clone, Copy)]
enum MainTab {
    #[default]
    Lexer,
    Parser,
    Try,
    Help,
}

impl MainTab {
    fn label(&self) -> &'static str {
        match self {
            MainTab::Lexer => "Lexer",
            MainTab::Parser => "Parser",
            MainTab::Try => "Try",
            MainTab::Help => "Help",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            MainTab::Lexer => "L",
            MainTab::Parser => "P",
            MainTab::Try => "T",
            MainTab::Help => "?",
        }
    }
}

#[derive(Default, PartialEq, Clone, Copy)]
enum TryInclude {
    #[default]
    LexerOnly,
    ParserOnly,
    Both,
}

#[derive(Default, PartialEq, Clone, Copy)]
enum LexerSubTab {
    #[default]
    Code,
    Dfa,
    Nfa,
    Debug,
}

#[derive(Default, PartialEq, Clone, Copy)]
enum ParserSubTab {
    #[default]
    Code,
    LalrTable,
    Glr,
    Debug,
    Tree,
}

#[derive(Default, PartialEq, Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
enum ParserMode {
    #[default]
    LALR,
    GLR,
}

impl ParserMode {
    fn as_str(&self) -> &'static str {
        match self {
            ParserMode::LALR => "LALR(1)",
            ParserMode::GLR => "GLR",
        }
    }
}

#[derive(Default, PartialEq, Clone, Copy)]
enum TargetLanguage {
    #[default]
    Python,
    C,
    Java,
}

impl TargetLanguage {
    fn as_str(&self) -> &'static str {
        match self {
            TargetLanguage::Python => "python",
            TargetLanguage::C => "c",
            TargetLanguage::Java => "java",
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            TargetLanguage::Python => "Python",
            TargetLanguage::C => "C",
            TargetLanguage::Java => "Java",
        }
    }

    fn extension(&self) -> &'static str {
        match self {
            TargetLanguage::Python => ".py",
            TargetLanguage::C => ".c",
            TargetLanguage::Java => ".java",
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl LogLevel {
    fn color(&self) -> egui::Color32 {
        match self {
            LogLevel::Info => egui::Color32::from_rgb(100, 180, 255),
            LogLevel::Success => egui::Color32::from_rgb(100, 255, 100),
            LogLevel::Warning => egui::Color32::from_rgb(255, 200, 50),
            LogLevel::Error => egui::Color32::from_rgb(255, 100, 100),
        }
    }

    fn prefix(&self) -> &'static str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Success => "OK",
            LogLevel::Warning => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

#[derive(Clone)]
struct LogEntry {
    timestamp: String,
    level: LogLevel,
    message: String,
}

// ============================================================================
// Generation Options
// ============================================================================

#[derive(Clone)]
struct LexerOptions {
    include_test_driver: bool,
    enable_unicode: bool,
    optimize_dfa: bool,
}

impl Default for LexerOptions {
    fn default() -> Self {
        Self {
            include_test_driver: true,
            enable_unicode: true,
            optimize_dfa: true,
        }
    }
}

#[derive(Clone)]
struct ParserOptions {
    mode: ParserMode,
    include_test_driver: bool,
    verbose_errors: bool,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            mode: ParserMode::LALR,
            include_test_driver: true,
            verbose_errors: true,
        }
    }
}

// ============================================================================
// Main Application State
// ============================================================================

struct OpenLexerApp {
    // Navigation
    current_tab: MainTab,
    lexer_sub_tab: LexerSubTab,
    parser_sub_tab: ParserSubTab,

    // Language selection
    language: TargetLanguage,

    // Lexer state
    lexer_input: String,
    lexer_output: String,
    lexer_options: LexerOptions,

    // Parser state
    parser_input: String,
    parser_output: String,
    parser_options: ParserOptions,

    // Try tab - user test program
    try_code: String,
    try_include: TryInclude,

    // Cached build artifacts for visualization
    cached_nfa: Option<Nfa>,
    cached_dfa: Option<Dfa>,
    cached_spec: Option<LexerSpec>,
    cached_grammar: Option<Grammar>,
    cached_parsing_table: Option<ParsingTable>,

    // Debugger state
    lexer_debugger: Option<LexerDebugger>,
    parser_debugger: Option<ParserDebugger>,
    debug_input: String,
    lexer_debug_steps: Vec<LexerDebugStep>,
    parser_debug_steps: Vec<ParserDebugStep>,

    // UI state
    show_logs: bool,
    show_options: bool,
    logs: Vec<LogEntry>,
    status: String,
    error: Option<String>,

    // Theme
    dark_mode: bool,

    // Code execution
    run_output: String,
    run_input: String,
    _run_id: Option<i32>,
    is_running: bool,
}

struct TreeNodeLayout {
    id: usize,
    x: f32,
    y: f32,
    width: f32,
    children: Vec<TreeNodeLayout>,
}

impl TreeNodeLayout {
    fn new(id: usize, debugger: &ParserDebugger) -> Self {
        // Safe unwrap because ids come from debugger
        let node = debugger.get_node(id).unwrap();
        let children: Vec<TreeNodeLayout> = node.children.iter().map(|&c| Self::new(c, debugger)).collect();
        
        let spacing = 20.0;
        let width = if children.is_empty() {
            // Estimate text width
            (node.label.len() as f32 * 7.0 + 20.0).max(40.0)
        } else {
            let children_width: f32 = children.iter().map(|c| c.width).sum();
            children_width + (children.len().saturating_sub(1) as f32 * spacing)
        };
        
        Self { id, x: 0.0, y: 0.0, width, children }
    }
    
    fn assign_positions(&mut self, x: f32, y: f32) {
        self.x = x + self.width / 2.0;
        self.y = y;
        
        let start_x = x;
        let mut current_x = start_x;
        let spacing = 20.0;
        
        for child in &mut self.children {
            child.assign_positions(current_x, y + 60.0);
            current_x += child.width + spacing;
        }
    }
    
    fn draw(&self, ui: &mut egui::Ui, painter: &egui::Painter, offset: egui::Vec2, debugger: &ParserDebugger) {
        let node_pos = egui::pos2(self.x, self.y) + offset;
        
        // Draw edges
        for child in &self.children {
            let child_pos = egui::pos2(child.x, child.y) + offset;
            painter.line_segment(
                [node_pos + egui::vec2(0.0, 15.0), child_pos - egui::vec2(0.0, 15.0)],
                egui::Stroke::new(1.0, egui::Color32::GRAY),
            );
            child.draw(ui, painter, offset, debugger);
        }
        
        // Draw node
        let node = debugger.get_node(self.id).unwrap();
        let text = &node.label;
        
        // Background
        let rect_size = egui::vec2(text.len() as f32 * 7.0 + 10.0, 20.0).max(egui::vec2(30.0, 20.0));
        let rect = egui::Rect::from_center_size(node_pos, rect_size);
        
        painter.rect_filled(rect, 5.0, egui::Color32::from_rgb(40, 40, 40));
        painter.rect_stroke(
            rect,
            5.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 200)),
        );
        
        painter.text(
            node_pos,
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        );
    }
}

impl OpenLexerApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            current_tab: MainTab::default(),
            lexer_sub_tab: LexerSubTab::default(),
            parser_sub_tab: ParserSubTab::default(),
            language: TargetLanguage::default(),
            lexer_input: String::new(),
            lexer_output: String::new(),
            lexer_options: LexerOptions::default(),
            parser_input: String::new(),
            parser_output: String::new(),
            parser_options: ParserOptions::default(),
            try_code: String::new(),
            try_include: TryInclude::LexerOnly,
            cached_nfa: None,
            cached_dfa: None,
            cached_spec: None,
            cached_grammar: None,
            cached_parsing_table: None,
            lexer_debugger: None,
            parser_debugger: None,
            debug_input: String::new(),
            lexer_debug_steps: Vec::new(),
            parser_debug_steps: Vec::new(),
            show_logs: true,
            show_options: true,
            logs: Vec::new(),
            status: "Ready".to_string(),
            error: None,
            dark_mode: true,
            run_output: String::new(),
            run_input: "3 + 4 * 2".to_string(),
            _run_id: None,
            is_running: false,
        };

        app.log(
            LogLevel::Info,
            &format!("OpenLexer v{} initialized", VERSION),
        );
        app.log(
            LogLevel::Info,
            "Features: LALR(1), GLR, Unicode, Start Conditions",
        );
        app
    }

    fn log(&mut self, level: LogLevel, message: &str) {
        let now = chrono_lite_time();
        self.logs.push(LogEntry {
            timestamp: now,
            level,
            message: message.to_string(),
        });
        if self.logs.len() > 500 {
            self.logs.remove(0);
        }
    }

    // ========================================================================
    // Generation Functions
    // ========================================================================

    fn generate_lexer(&mut self) {
        self.error = None;
        self.status = "Generating lexer...".to_string();
        self.log(
            LogLevel::Info,
            &format!(
                "Starting lexer generation for {}",
                self.language.display_name()
            ),
        );

        match lexgen::parse_lexer_spec(&self.lexer_input) {
            Ok(spec) => {
                let rule_count = spec.rules.len();
                let condition_count = spec.condition_names().len();
                self.log(
                    LogLevel::Success,
                    &format!(
                        "Parsed {} rules, {} start conditions",
                        rule_count, condition_count
                    ),
                );

                // Build NFA and DFA for the INITIAL condition and cache them
                match Nfa::from_lexer_spec_for_condition(&spec, "INITIAL") {
                    Ok(nfa) => {
                        self.log(
                            LogLevel::Info,
                            &format!("Built NFA: {} states", nfa.states.len()),
                        );
                        match Dfa::from_nfa(&nfa) {
                            Ok(dfa) => {
                                self.log(
                                    LogLevel::Info,
                                    &format!("Built DFA: {} states", dfa.states.len()),
                                );
                                self.cached_nfa = Some(nfa);
                                self.cached_dfa = Some(dfa);
                            }
                            Err(e) => {
                                self.log(
                                    LogLevel::Warning,
                                    &format!("DFA construction failed (debug tables unavailable): {}", e),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        self.log(
                            LogLevel::Warning,
                            &format!("NFA construction failed (debug tables unavailable): {}", e),
                        );
                    }
                }

                self.cached_spec = Some(spec.clone());

                // Generate code using the standard pipeline
                match lexgen::generate_code(&spec, self.language.as_str()) {
                    Ok(code) => {
                        let line_count = code.lines().count();
                        self.lexer_output = code;
                        self.status = format!(
                            "Generated {} lexer ({} lines)",
                            self.language.display_name(),
                            line_count
                        );
                        self.log(LogLevel::Success, &self.status.clone());
                    }
                    Err(e) => {
                        self.error = Some(format!("Code generation error: {}", e));
                        self.log(LogLevel::Error, &format!("Generation failed: {}", e));
                    }
                }
            }
            Err(e) => {
                self.error = Some(format!("Parse error: {}", e));
                self.log(LogLevel::Error, &format!("Parse error: {}", e));
            }
        }
    }

    fn generate_parser(&mut self) {
        self.error = None;
        let mode_str = self.parser_options.mode.as_str();
        self.status = format!("Generating {} parser...", mode_str);
        self.log(
            LogLevel::Info,
            &format!(
                "Starting {} parser generation for {}",
                mode_str,
                self.language.display_name()
            ),
        );

        match parsegen::parse_grammar(&self.parser_input) {
            Ok(grammar) => {
                let rule_count = grammar.rules.len();
                let token_count = grammar.tokens.len();
                self.log(
                    LogLevel::Success,
                    &format!("Parsed {} rules, {} tokens", rule_count, token_count),
                );

                // Build parsing table and cache it along with the grammar
                let lang_str = self.language.as_str();
                let target = match lang_str.to_lowercase().as_str() {
                    "c" => openlexer_lib::lexgen::codegen::TargetLanguage::C,
                    "java" => openlexer_lib::lexgen::codegen::TargetLanguage::Java,
                    _ => openlexer_lib::lexgen::codegen::TargetLanguage::Python,
                };

                self.log(LogLevel::Info, &format!("Building {} parsing tables...", mode_str));

                match ParsingTable::build(&grammar) {
                    Ok(table) => {
                        let state_count = table.action.len();
                        let conflict_count = table.shift_reduce_conflicts + table.reduce_reduce_conflicts;
                        self.log(
                            LogLevel::Info,
                            &format!(
                                "Built parsing table: {} states, {} conflicts",
                                state_count, conflict_count
                            ),
                        );

                        // Cache for visualization
                        self.cached_grammar = Some(grammar.clone());
                        self.cached_parsing_table = Some(table.clone());

                        // Generate code from the cached table
                        match parser_codegen::generate_parser(&table, &grammar, target) {
                            Ok(code) => {
                                let line_count = code.lines().count();
                                self.parser_output = code;
                                self.status = format!(
                                    "Generated {} {} parser ({} lines)",
                                    self.language.display_name(),
                                    mode_str,
                                    line_count
                                );
                                self.log(LogLevel::Success, &self.status.clone());
                            }
                            Err(e) => {
                                self.error = Some(format!("Generation error: {}", e));
                                self.log(LogLevel::Error, &format!("Generation failed: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        self.error = Some(format!("Table construction error: {}", e));
                        self.log(LogLevel::Error, &format!("Table construction failed: {}", e));
                    }
                }
            }
            Err(e) => {
                self.error = Some(format!("Grammar error: {}", e));
                self.log(LogLevel::Error, &format!("Grammar error: {}", e));
            }
        }
    }

    fn run_try_code(&mut self) {
        if self.try_code.trim().is_empty() {
            self.log(LogLevel::Warning, "Write your test program first, then click Run.");
            return;
        }

        // Auto-prepend generated code based on include mode
        let mut full_code = String::new();

        let include_lexer = self.try_include == TryInclude::LexerOnly || self.try_include == TryInclude::Both;
        let include_parser = self.try_include == TryInclude::ParserOnly || self.try_include == TryInclude::Both;

        // For C, define LEXER_NO_MAIN and PARSER_NO_MAIN to suppress default main functions
        // but keep the test() helpers available.
        if self.language == TargetLanguage::C {
            full_code.push_str("#define LEXER_NO_MAIN\n");
            if include_parser {
                full_code.push_str("#define PARSER_NO_MAIN\n");
            }
        }

        if self.language == TargetLanguage::Java {
            // For Java, we MUST handle the "one public class per file" rule
            // Java requires exactly ONE public class per .java file, and the filename must match.
            // 
            // Strategy: Make Parser class non-public so Lexer.java can contain both
            // This file would be saved as "Lexer.java" (matching the public class name)
            
            let mut base_code = String::new();
            if include_lexer { base_code.push_str(&self.lexer_output); }
            if include_parser {
                // Make Parser class non-public to avoid Java compilation error
                // Change "public class Parser" to "class Parser"
                let parser_code = self.parser_output.replace("public class Parser", "class Parser");
                base_code.push_str("\n\n");
                base_code.push_str(&parser_code);
            }
            
            if let Some(start_idx) = base_code.find("public static void main(String[] args) {") {
                let after_brace = start_idx + "public static void main(String[] args) {".len();
                
                // Find matching closing brace for main
                let mut brace_count = 1;
                let mut end_idx = after_brace;
                for (i, c) in base_code[after_brace..].char_indices() {
                    match c {
                        '{' => brace_count += 1,
                        '}' => brace_count -= 1,
                        _ => {}
                    }
                    if brace_count == 0 {
                        end_idx = after_brace + i;
                        break;
                    }
                }
                
                if brace_count == 0 {
                    // Replace body of main with user code
                    full_code.push_str(&base_code[..after_brace]);
                    full_code.push_str("\n        // User Test Code Injected Here\n");
                    full_code.push_str(&self.try_code);
                    full_code.push_str("\n    "); // Indentation before closing brace
                    full_code.push_str(&base_code[end_idx..]);
                } else {
                    // Fallback if parsing fails
                    full_code.push_str(&base_code);
                }
            } else {
                 full_code.push_str(&base_code);
            }
        } else {
            // Standard append logic for Python/C
            if include_lexer {
                if self.lexer_output.is_empty() {
                    self.log(LogLevel::Warning, "No generated lexer. Go to Lexer tab and Generate first.");
                    return;
                }
                full_code.push_str(&self.strip_default_main(&self.lexer_output));
                full_code.push('\n');
            }

            if include_parser {
                if self.parser_output.is_empty() {
                    self.log(LogLevel::Warning, "No generated parser. Go to Parser tab and Generate first.");
                    return;
                }
                full_code.push_str(&self.strip_default_main(&self.parser_output));
                full_code.push('\n');
            }

            // Append user's test program
            full_code.push_str(&self.try_code);
        }

        let total_lines = full_code.lines().count();
        self.log(LogLevel::Info, &format!("Running {} code ({} lines total: generated + your test)", self.language.display_name(), total_lines));
        self.run_generated_code(&full_code, None);
    }

    fn strip_default_main(&self, code: &str) -> String {
        match self.language {
            TargetLanguage::Python => {
                // Strip if __name__ == '__main__': block
                let code = if let Some(idx) = code.find("if __name__ == '__main__':") {
                    &code[..idx]
                } else {
                    code
                };
                // Also strip parse_expression and test_parse functions that contain
                // 'from lexer import Lexer' — these fail in combined single-file execution
                let code = if let Some(idx) = code.find("def parse_expression(") {
                    code[..idx].to_string()
                } else {
                    code.to_string()
                };
                code
            }
            _ => code.to_string(),
        }
    }

    // ========================================================================
    // Code Execution
    // ========================================================================

    fn run_generated_code(&mut self, code: &str, extra_code: Option<&str>) {
        if code.is_empty() {
            self.log(LogLevel::Warning, "No generated code to run. Click Generate first.");
            return;
        }

        let lang_str = self.language.as_str();
        self.log(LogLevel::Info, &format!("Running {} code with input: {}", lang_str, &self.run_input));
        self.run_output = format!("Running {} code...\n", self.language.display_name());
        self.is_running = true;

        #[cfg(target_arch = "wasm32")]
        {
            let id = startCodeRun(lang_str, code, &self.run_input, extra_code.unwrap_or(""));
            self._run_id = Some(id);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.run_output = format!(
                "In-browser execution is only available in the web version.\n\
                 To run locally:\n  {} {}",
                match self.language {
                    TargetLanguage::C => "gcc -o lexer lexer.c && ./lexer",
                    TargetLanguage::Python => "python lexer.py",
                    TargetLanguage::Java => "javac Lexer.java && java Lexer",
                },
                &self.run_input
            );
            self.is_running = false;
        }
    }

    fn poll_run_result(&mut self) {
        #[cfg(target_arch = "wasm32")]
        if let Some(id) = self._run_id {
            if isRunDone(id) {
                let result = getRunResult(id);
                self.run_output = result;
                self._run_id = None;
                self.is_running = false;
                self.log(LogLevel::Success, "Code execution completed");
            }
        }
    }

    // ========================================================================
    // UI Rendering
    // ========================================================================

    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Logo and title
            ui.heading(
                egui::RichText::new(format!("{} v{}", APP_NAME, VERSION))
                    .strong()
                    .color(egui::Color32::from_rgb(64, 160, 255)),
            );

            ui.separator();

            // Tab buttons
            for tab in [
                MainTab::Lexer,
                MainTab::Parser,
                MainTab::Try,
                MainTab::Help,
            ] {
                let selected = self.current_tab == tab;
                let text = egui::RichText::new(format!("[{}] {}", tab.icon(), tab.label()));
                let text = if selected {
                    text.strong().color(egui::Color32::WHITE)
                } else {
                    text.color(egui::Color32::GRAY)
                };

                if ui
                    .add(egui::Button::new(text).fill(if selected {
                        egui::Color32::from_rgb(50, 100, 150)
                    } else {
                        egui::Color32::TRANSPARENT
                    }))
                    .clicked()
                {
                    self.current_tab = tab;
                }
            }

            ui.separator();

            // Language selector
            ui.label("Target:");
            egui::ComboBox::from_id_salt("lang_select")
                .selected_text(self.language.display_name())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.language, TargetLanguage::Python, "Python");
                    ui.selectable_value(&mut self.language, TargetLanguage::C, "C");
                    ui.selectable_value(&mut self.language, TargetLanguage::Java, "Java");
                });

            // Right-aligned controls
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Theme toggle
                let theme_text = if self.dark_mode { "Light" } else { "Dark" };
                if ui.button(theme_text).clicked() {
                    self.dark_mode = !self.dark_mode;
                }

                // Options toggle
                let opts_text = if self.show_options {
                    "Hide Opts"
                } else {
                    "Options"
                };
                if ui.button(opts_text).clicked() {
                    self.show_options = !self.show_options;
                }

                // Logs toggle
                let log_text = if self.show_logs { "Hide Logs" } else { "Logs" };
                if ui.button(log_text).clicked() {
                    self.show_logs = !self.show_logs;
                }
            });
        });
    }


    fn render_spec_editor(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Lexer Specification (.l)")
                    .strong()
                    .size(14.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Clear").clicked() {
                    self.lexer_input.clear();
                }
                if ui.button("Load Advanced").clicked() {
                    self.lexer_input = SAMPLE_LEXER_ADVANCED.to_string();
                    self.log(LogLevel::Info, "Loaded advanced lexer example");
                }
                if ui.button("Load Sample").clicked() {
                    self.lexer_input = SAMPLE_LEXER.to_string();
                    self.log(LogLevel::Info, "Loaded basic lexer sample");
                }
                if ui.button("Generate").clicked() {
                    self.generate_lexer();
                }
            });
        });
        if self.show_options {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.lexer_options.include_test_driver, "Test Driver");
                ui.checkbox(&mut self.lexer_options.enable_unicode, "Unicode");
                ui.checkbox(&mut self.lexer_options.optimize_dfa, "Optimize");
            });
        }
        ui.add_space(2.0);
        let editor_height = (available.y - 60.0).max(100.0);
        code_editor_with_lines(ui, "lexer_input", &mut self.lexer_input, available.x - 10.0, editor_height);
    }

    fn render_code_output(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();

        // Sub-tab bar
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.lexer_sub_tab, LexerSubTab::Code, "Code");
            ui.selectable_value(&mut self.lexer_sub_tab, LexerSubTab::Dfa, "DFA States");
            ui.selectable_value(&mut self.lexer_sub_tab, LexerSubTab::Nfa, "NFA States");
            ui.selectable_value(&mut self.lexer_sub_tab, LexerSubTab::Debug, "Debug");
        });
        ui.separator();

        match self.lexer_sub_tab {
            LexerSubTab::Code => {
                // Original code output with toolbar
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Generated Code ({})", self.language.extension()))
                            .strong()
                            .size(14.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("{} lines", self.lexer_output.lines().count()))
                                .small()
                                .color(egui::Color32::GRAY),
                        );
                        if ui.button("Clear").clicked() {
                            self.lexer_output.clear();
                        }
                        if ui.button("Copy").clicked() {
                            ui.ctx().copy_text(self.lexer_output.clone());
                            self.log(LogLevel::Info, "Code copied to clipboard");
                        }
                        self.render_download_button(ui, "lexer", &self.lexer_output.clone());
                        ui.separator();
                        if ui.button("\u{1F41B} Debug").clicked() {
                            self.lexer_sub_tab = LexerSubTab::Debug;
                        }
                    });
                });
                ui.add_space(2.0);
                let viewer_height = (available.y - 70.0).max(100.0);
                code_viewer_with_lines(ui, "lexer_output", &self.lexer_output, available.x - 10.0, viewer_height);
            }
            LexerSubTab::Dfa => {
                self.render_dfa_table(ui, available);
            }
            LexerSubTab::Nfa => {
                self.render_nfa_table(ui, available);
            }
            LexerSubTab::Debug => {
                self.render_lexer_debug_panel(ui, available);
            }
        }
    }

    fn render_run_panel(&mut self, ui: &mut egui::Ui) {
        // Use a SidePanel for the left input area, making it resizable.
        // We use show_inside to constrain it to the parent ui (which is the BottomPanel).
        egui::SidePanel::left("run_input_panel")
            .resizable(true)
            .default_width(220.0)
            .width_range(150.0..=500.0)
            .show_inside(ui, |ui| {
                ui.vertical(|ui| {
                    ui.add_space(5.0);
                    ui.label(
                        egui::RichText::new("Test Input")
                            .strong()
                            .size(13.0)
                            .color(egui::Color32::from_rgb(100, 200, 255)),
                    );

                    ui.add_space(5.0);

                    // Input Text Area
                    egui::ScrollArea::vertical()
                        .id_salt("input_scroll")
                        .max_height(ui.available_height() - 40.0) 
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.run_input)
                                    .code_editor()
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(10) // default rows
                                    .hint_text("Enter test input (e.g. 3 + 4 * 2)"),
                            );
                        });

                    ui.add_space(5.0);

                    // Run Button (at bottom of side panel)
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        let run_text = if self.is_running {
                            "\u{23f3} Running..."
                        } else {
                            "\u{25b6} Run Code"
                        };
                        let run_btn = ui.add_enabled(
                            !self.is_running,
                            egui::Button::new(
                                egui::RichText::new(run_text)
                                    .color(egui::Color32::WHITE)
                                    .strong(),
                            )
                            .fill(egui::Color32::from_rgb(0, 150, 80))
                            .min_size(egui::vec2(120.0, 30.0)),
                        );

                        if run_btn.clicked() {
                            let code = self.lexer_output.clone();
                            self.run_generated_code(&code, None);
                        }
                    });
                });
            });

        // The rest is the Output Panel
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.vertical(|ui| {
                ui.add_space(5.0);
                ui.label(
                    egui::RichText::new("Execution Output")
                        .strong()
                        .size(13.0)
                        .color(egui::Color32::from_rgb(100, 255, 150)),
                );
                
                ui.add_space(5.0);

                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(15, 15, 25))
                    .rounding(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("output_scroll")
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.set_min_height(ui.available_height()); // Fill remaining
                                
                                if self.run_output.is_empty() {
                                    ui.centered_and_justified(|ui| {
                                        ui.label(
                                            egui::RichText::new("Click \u{25b6} Run to execute code in browser\n(Uses Wasmer for C, CheerpJ for Java)")
                                                .color(egui::Color32::from_rgb(100, 100, 130))
                                                .italics(),
                                        );
                                    });
                                } else {
                                    let color = if self.run_output.contains("[Error]") || self.run_output.contains("Error]") {
                                        egui::Color32::from_rgb(255, 120, 120)
                                    } else {
                                        egui::Color32::from_rgb(180, 255, 180)
                                    };
                                    
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&self.run_output)
                                                .monospace()
                                                .color(color)
                                                .size(13.0)
                                        ).wrap_mode(egui::TextWrapMode::Wrap)
                                    );
                                }
                            });
                    });
            });
        });
    }



    fn render_parser_editor(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Grammar Specification (.y)")
                    .strong()
                    .size(14.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Clear").clicked() {
                    self.parser_input.clear();
                }
                if ui.button("Load GLR").clicked() {
                    self.parser_input = SAMPLE_PARSER_AMBIGUOUS.to_string();
                    self.parser_options.mode = ParserMode::GLR;
                    self.log(LogLevel::Info, "Loaded ambiguous grammar for GLR");
                }
                if ui.button("Load Sample").clicked() {
                    self.parser_input = SAMPLE_PARSER.to_string();
                    self.log(LogLevel::Info, "Loaded basic grammar sample");
                }
                if ui.button("Generate").clicked() {
                    self.generate_parser();
                }
            });
        });
        if self.show_options {
            ui.horizontal(|ui| {
                ui.label("Mode:");
                ui.selectable_value(
                    &mut self.parser_options.mode,
                    ParserMode::LALR,
                    "LALR(1)",
                );
                ui.selectable_value(&mut self.parser_options.mode, ParserMode::GLR, "GLR");
                ui.separator();
                ui.checkbox(&mut self.parser_options.include_test_driver, "Test Driver");
                ui.checkbox(&mut self.parser_options.verbose_errors, "Verbose");
            });
        }
        ui.add_space(2.0);
        let editor_height = (available.y - 60.0).max(100.0);
        code_editor_with_lines(ui, "parser_input", &mut self.parser_input, available.x - 10.0, editor_height);
    }

    fn render_parser_output(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();

        // Sub-tab bar
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.parser_sub_tab, ParserSubTab::Code, "Code");
            ui.selectable_value(&mut self.parser_sub_tab, ParserSubTab::LalrTable, "LALR Table");
            ui.selectable_value(&mut self.parser_sub_tab, ParserSubTab::Glr, "GLR Info");
            ui.selectable_value(&mut self.parser_sub_tab, ParserSubTab::Debug, "Debug");
            ui.selectable_value(&mut self.parser_sub_tab, ParserSubTab::Tree, "Tree");
        });
        ui.separator();

        match self.parser_sub_tab {
            ParserSubTab::Code => {
                // Original code output with toolbar
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "Generated {} Parser ({})",
                            self.parser_options.mode.as_str(),
                            self.language.extension()
                        ))
                        .strong()
                        .size(14.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} lines",
                                self.parser_output.lines().count()
                            ))
                            .small()
                            .color(egui::Color32::GRAY),
                        );
                        if ui.button("Clear").clicked() {
                            self.parser_output.clear();
                        }
                        if ui.button("Copy").clicked() {
                            ui.ctx().copy_text(self.parser_output.clone());
                            self.log(LogLevel::Info, "Code copied to clipboard");
                        }
                        self.render_download_button(ui, "parser", &self.parser_output.clone());
                        ui.separator();
                        if ui.button("\u{1F41B} Debug").clicked() {
                            self.parser_sub_tab = ParserSubTab::Debug;
                        }
                    });
                });
                ui.add_space(2.0);
                let viewer_height = (available.y - 70.0).max(100.0);
                code_viewer_with_lines(
                    ui,
                    "parser_output",
                    &self.parser_output,
                    available.x - 10.0,
                    viewer_height,
                );
            }
            ParserSubTab::LalrTable => {
                self.render_lalr_table(ui);
            }
            ParserSubTab::Glr => {
                self.render_glr_conflicts(ui);
            }
            ParserSubTab::Debug => {
                self.render_parser_debug_panel(ui);
            }
            ParserSubTab::Tree => {
                self.render_parser_tree(ui);
            }
        }
    }

    fn render_parser_run_panel(&mut self, ui: &mut egui::Ui) {
        egui::SidePanel::left("parser_run_input_panel")
            .resizable(true)
            .default_width(220.0)
            .width_range(150.0..=500.0)
            .show_inside(ui, |ui| {
                ui.vertical(|ui| {
                    ui.add_space(5.0);
                    ui.label(
                        egui::RichText::new("Test Input")
                            .strong()
                            .size(13.0)
                            .color(egui::Color32::from_rgb(100, 200, 255)),
                    );

                    ui.add_space(5.0);

                    egui::ScrollArea::vertical()
                        .id_salt("parser_input_scroll")
                        .max_height(ui.available_height() - 40.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.run_input)
                                    .code_editor()
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(10)
                                    .hint_text("Enter test input (e.g. 3 + 4 * 2)"),
                            );
                        });

                    ui.add_space(5.0);

                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        let run_text = if self.is_running {
                            "\u{23f3} Running..."
                        } else {
                            "\u{25b6} Run Code"
                        };
                        let run_btn = ui.add_enabled(
                            !self.is_running,
                            egui::Button::new(
                                egui::RichText::new(run_text)
                                    .color(egui::Color32::WHITE)
                                    .strong(),
                            )
                            .fill(egui::Color32::from_rgb(0, 150, 80))
                            .min_size(egui::vec2(120.0, 30.0)),
                        );

                        if run_btn.clicked() {
                            let code = self.parser_output.clone();
                            let extra = if self.language == TargetLanguage::Python && !self.lexer_output.trim().is_empty() {
                                Some(self.lexer_output.clone())
                            } else {
                                None
                            };
                            self.run_generated_code(&code, extra.as_deref());
                        }
                    });
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.vertical(|ui| {
                ui.add_space(5.0);
                ui.label(
                    egui::RichText::new("Execution Output")
                        .strong()
                        .size(13.0)
                        .color(egui::Color32::from_rgb(100, 255, 150)),
                );

                ui.add_space(5.0);

                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(15, 15, 25))
                    .rounding(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("parser_output_scroll")
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.set_min_height(ui.available_height());

                                if self.run_output.is_empty() {
                                    ui.centered_and_justified(|ui| {
                                        ui.label(
                                            egui::RichText::new("Click \u{25b6} Run to execute code in browser\n(Uses Wasmer for C, CheerpJ for Java)")
                                                .color(egui::Color32::from_rgb(100, 100, 130))
                                                .italics(),
                                        );
                                    });
                                } else {
                                    let color = if self.run_output.contains("[Error]") || self.run_output.contains("Error]") {
                                        egui::Color32::from_rgb(255, 120, 120)
                                    } else {
                                        egui::Color32::from_rgb(180, 255, 180)
                                    };

                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&self.run_output)
                                                .monospace()
                                                .color(color)
                                                .size(13.0)
                                        ).wrap_mode(egui::TextWrapMode::Wrap)
                                    );
                                }
                            });
                    });
            });
        });
    }

    fn render_try_editor(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();

        // Toolbar with include selector and status
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("Your Test Program ({})", self.language.extension()))
                    .strong()
                    .size(14.0),
            );

            ui.separator();

            // Include selector
            ui.label("Import:");
            ui.selectable_value(&mut self.try_include, TryInclude::LexerOnly, "Lexer");
            ui.selectable_value(&mut self.try_include, TryInclude::ParserOnly, "Parser");
            ui.selectable_value(&mut self.try_include, TryInclude::Both, "Both");

            ui.separator();

            // Run button
            let run_text = if self.is_running { "\u{23f3} Running..." } else { "\u{25b6} Run" };
            let run_btn = ui.add_enabled(
                !self.is_running,
                egui::Button::new(
                    egui::RichText::new(run_text).color(egui::Color32::WHITE).strong(),
                )
                .fill(egui::Color32::from_rgb(0, 150, 80)),
            );
            if run_btn.clicked() {
                self.run_try_code();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Clear").clicked() {
                    self.try_code.clear();
                }

                // Show what generated code is available
                let lexer_lines = self.lexer_output.lines().count();
                let parser_lines = self.parser_output.lines().count();

                let lexer_status = if lexer_lines > 0 {
                    format!("Lexer: {} lines", lexer_lines)
                } else {
                    "Lexer: \u{2717}".to_string()
                };
                let parser_status = if parser_lines > 0 {
                    format!("Parser: {} lines", parser_lines)
                } else {
                    "Parser: \u{2717}".to_string()
                };

                let lexer_color = if lexer_lines > 0 {
                    egui::Color32::from_rgb(100, 255, 150)
                } else {
                    egui::Color32::from_rgb(255, 100, 100)
                };
                let parser_color = if parser_lines > 0 {
                    egui::Color32::from_rgb(100, 255, 150)
                } else {
                    egui::Color32::from_rgb(255, 100, 100)
                };

                ui.label(egui::RichText::new(parser_status).small().color(parser_color));
                ui.label(egui::RichText::new("|").small().color(egui::Color32::GRAY));
                ui.label(egui::RichText::new(lexer_status).small().color(lexer_color));
            });
        });

        ui.add_space(2.0);

        let editor_height = (available.y - 40.0).max(100.0);
        code_editor_with_lines(ui, "try_code", &mut self.try_code, available.x - 10.0, editor_height);
    }

    fn render_try_run_panel(&mut self, ui: &mut egui::Ui) {
        egui::SidePanel::left("try_run_input_panel")
            .resizable(true)
            .default_width(220.0)
            .width_range(150.0..=500.0)
            .show_inside(ui, |ui| {
                ui.vertical(|ui| {
                    ui.add_space(5.0);
                    ui.label(
                        egui::RichText::new("Test Input")
                            .strong()
                            .size(13.0)
                            .color(egui::Color32::from_rgb(100, 200, 255)),
                    );

                    ui.add_space(5.0);

                    egui::ScrollArea::vertical()
                        .id_salt("try_input_scroll")
                        .max_height(ui.available_height() - 40.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.run_input)
                                    .code_editor()
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(10)
                                    .hint_text("Enter test input (e.g. 3 + 4 * 2)"),
                            );
                        });

                    ui.add_space(5.0);

                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        let run_text = if self.is_running {
                            "\u{23f3} Running..."
                        } else {
                            "\u{25b6} Run Code"
                        };
                        let run_btn = ui.add_enabled(
                            !self.is_running,
                            egui::Button::new(
                                egui::RichText::new(run_text)
                                    .color(egui::Color32::WHITE)
                                    .strong(),
                            )
                            .fill(egui::Color32::from_rgb(0, 150, 80))
                            .min_size(egui::vec2(120.0, 30.0)),
                        );

                        if run_btn.clicked() {
                            self.run_try_code();
                        }
                    });
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.vertical(|ui| {
                ui.add_space(5.0);
                ui.label(
                    egui::RichText::new("Execution Output")
                        .strong()
                        .size(13.0)
                        .color(egui::Color32::from_rgb(100, 255, 150)),
                );

                ui.add_space(5.0);

                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(15, 15, 25))
                    .rounding(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("try_output_scroll")
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.set_min_height(ui.available_height());

                                if self.run_output.is_empty() {
                                    ui.centered_and_justified(|ui| {
                                        ui.label(
                                            egui::RichText::new("Write your test program above, then click \u{25b6} Run\nGenerated lexer/parser code is auto-imported")
                                                .color(egui::Color32::from_rgb(100, 100, 130))
                                                .italics(),
                                        );
                                    });
                                } else {
                                    let color = if self.run_output.contains("[Error]") || self.run_output.contains("Error]") {
                                        egui::Color32::from_rgb(255, 120, 120)
                                    } else {
                                        egui::Color32::from_rgb(180, 255, 180)
                                    };

                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&self.run_output)
                                                .monospace()
                                                .color(color)
                                                .size(13.0)
                                        ).wrap_mode(egui::TextWrapMode::Wrap)
                                    );
                                }
                            });
                    });
            });
        });
    }

    fn render_help_tab(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("OpenLexer Help");
            ui.add_space(8.0);

            ui.group(|ui| {
                ui.label(egui::RichText::new("Features").strong().size(16.0));
                ui.add_space(4.0);

                let features = [
                    (
                        "Lexer Generation",
                        "Convert regex patterns to DFA-based lexers",
                    ),
                    ("LALR(1) Parser", "Standard shift-reduce parser generation"),
                    ("GLR Parser", "Generalized LR for ambiguous grammars"),
                    (
                        "Unicode Support",
                        "Full Unicode character classes and properties",
                    ),
                    ("Start Conditions", "Context-dependent lexing states"),
                    ("Multi-Language", "Generate C, Java, or Python code"),
                    ("Test Drivers", "Built-in test runners for verification"),
                ];

                for (name, desc) in features {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("  {}", name)).strong());
                        ui.label(format!("- {}", desc));
                    });
                }
            });

            ui.add_space(16.0);

            ui.group(|ui| {
                ui.label(egui::RichText::new("Lexer Syntax").strong().size(16.0));
                ui.add_space(4.0);

                ui.code(
                    r#"%{
/* Definitions section */
%}

%x COMMENT   /* Exclusive start condition */
%s STRING    /* Inclusive start condition */

%%
/* Rules section */
[0-9]+         { return NUMBER; }
[a-zA-Z_]+     { return IDENTIFIER; }
\"             { BEGIN(STRING); }
<STRING>\"     { BEGIN(INITIAL); return STRING; }
"//".*         { /* skip line comment */ }
"/*"           { BEGIN(COMMENT); }
<COMMENT>"*/"  { BEGIN(INITIAL); }
%%
"#,
                );
            });

            ui.add_space(16.0);

            ui.group(|ui| {
                ui.label(egui::RichText::new("Parser Syntax").strong().size(16.0));
                ui.add_space(4.0);

                ui.code(
                    r#"%token NUMBER IDENTIFIER PLUS MINUS

%left PLUS MINUS
%left TIMES DIVIDE
%right UMINUS

%%

expr:
    expr PLUS expr   { $$ = $1 + $3; }
  | expr MINUS expr  { $$ = $1 - $3; }
  | LPAREN expr RPAREN { $$ = $2; }
  | NUMBER           { $$ = $1; }
  ;

%%
"#,
                );
            });

            ui.add_space(16.0);

            ui.group(|ui| {
                ui.label(egui::RichText::new("GLR Parsing").strong().size(16.0));
                ui.add_space(4.0);
                ui.label("GLR (Generalized LR) parsing handles ambiguous grammars by:");
                ui.label("  - Forking the parse stack on conflicts");
                ui.label("  - Exploring all valid parses in parallel");
                ui.label("  - Merging when paths reach the same state");
                ui.label("  - Building a shared parse forest (SPPF)");
                ui.add_space(4.0);
                ui.label("Use GLR mode for grammars with shift/reduce or reduce/reduce conflicts.");
            });

            ui.add_space(16.0);

            ui.group(|ui| {
                ui.label(egui::RichText::new("Test Drivers (Try Tab)").strong().size(16.0));
                ui.add_space(4.0);
                ui.label("The generated code includes a 'test(expr)' helper function. You can use it in the Try tab:");
                ui.add_space(8.0);

                ui.label(egui::RichText::new("Python").strong());
                ui.code(
                    r#"# Standard usage
test("3 + 4 * 5")

# Manual usage
lex = Lexer("10 / 2")
for token in lex.tokenize():
    print(token)"#
                );

                ui.add_space(8.0);
                ui.label(egui::RichText::new("C").strong());
                ui.code(
                    r#"// Define your own main
int main() {
    test("3 + 4 * 5");
    return 0;
}"#
                );

                ui.add_space(8.0);
                ui.label(egui::RichText::new("Java").strong());
                ui.code(
                    r#"// Code is injected into the main() method
test("3 + 4 * 5");

// Or manual usage
Lexer l = new Lexer("10 / 2");
Token t;
while ((t = l.next()).type != TokenType.EOF) {
    System.out.println(t.type);
}"#
                );
            });

            ui.add_space(16.0);

            ui.group(|ui| {
                ui.label(egui::RichText::new("Unicode Support").strong().size(16.0));
                ui.add_space(4.0);
                ui.label("Unicode character classes supported:");
                ui.code(
                    r#"\\p{Letter}     - Any Unicode letter
\\p{Nd}         - Decimal digit
\\p{Lu}         - Uppercase letter
\\p{Greek}      - Greek script
\\p{Emoji}      - Emoji characters
\\u{XXXX}       - Hex code point"#,
                );
            });

            ui.add_space(16.0);

            ui.horizontal(|ui| {
                ui.label("Version:");
                ui.label(VERSION);
                ui.separator();
                ui.hyperlink_to("Documentation", "https://github.com/magi8101/openlexer");
                ui.separator();
                ui.label("License: MIT");
            });
        });
    }

    // ========================================================================
    // Table and Debug Renderers
    // ========================================================================

    fn render_dfa_table(&mut self, ui: &mut egui::Ui, _available: egui::Vec2) {
        let dfa = match &self.cached_dfa {
            Some(d) => d,
            None => {
                ui.label("DFA not generated yet.");
                return;
            }
        };

        ui.label(egui::RichText::new("DFA Transition Table").strong().size(16.0));
        ui.add_space(5.0);

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::initial(50.0).resizable(true)) // State ID
            .column(Column::initial(80.0).resizable(true)) // Accepting
            .column(Column::initial(150.0).resizable(true)) // Rule/Token
            .column(Column::remainder()) // Transitions
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("State");
                });
                header.col(|ui| {
                    ui.strong("Accepting");
                });
                header.col(|ui| {
                    ui.strong("Rule / Token");
                });
                header.col(|ui| {
                    ui.strong("Transitions");
                });
            })
            .body(|mut body| {
                for (state_id, state) in dfa.states.iter().enumerate() {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.label(state_id.to_string());
                        });
                        row.col(|ui| {
                            if state.is_accepting {
                                ui.label(egui::RichText::new("Yes").color(egui::Color32::GREEN));
                            } else {
                                ui.label("No");
                            }
                        });
                        row.col(|ui| {
                            if let Some(rule_idx) = state.rule_index {
                                let mut token_name = format!("Rule #{}", rule_idx);
                                if let Some(spec) = &self.cached_spec {
                                    if let Some(rule) = spec.rules.get(rule_idx) {
                                        match &rule.action {
                                            openlexer_lib::lexgen::rules::RuleAction::Token(name) => {
                                                token_name = name.clone();
                                            }
                                            openlexer_lib::lexgen::rules::RuleAction::Skip => {
                                                token_name = "(skip)".to_string();
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                ui.label(token_name);
                            } else {
                                ui.label("-");
                            }
                        });
                        row.col(|ui| {
                            let transitions: Vec<String> = state
                                .range_transitions
                                .iter()
                                .map(|(start, end, target)| {
                                    if start == end {
                                        let c = std::char::from_u32(*start).unwrap_or('?');
                                        format!("'{}' -> {}", c.escape_debug(), target)
                                    } else {
                                        let s = std::char::from_u32(*start).unwrap_or('?');
                                        let e = std::char::from_u32(*end).unwrap_or('?');
                                        format!(
                                            "'{}'-'{}' -> {}",
                                            s.escape_debug(),
                                            e.escape_debug(),
                                            target
                                        )
                                    }
                                })
                                .collect();
                            ui.label(transitions.join(", "));
                        });
                    });
                }
            });
    }

    fn render_nfa_table(&mut self, ui: &mut egui::Ui, _available: egui::Vec2) {
        let nfa = match &self.cached_nfa {
            Some(n) => n,
            None => {
                ui.label("NFA not generated yet.");
                return;
            }
        };

        ui.label(egui::RichText::new("NFA Transition Table").strong().size(16.0));
        ui.add_space(5.0);

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::initial(50.0).resizable(true)) // State ID
            .column(Column::initial(80.0).resizable(true)) // Accepting
            .column(Column::remainder()) // Transitions
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("State");
                });
                header.col(|ui| {
                    ui.strong("Accepting");
                });
                header.col(|ui| {
                    ui.strong("Transitions");
                });
            })
            .body(|mut body| {
                for (state_id, state) in nfa.states.iter().enumerate() {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.label(state_id.to_string());
                        });
                        row.col(|ui| {
                            if let Some(rule_idx) = state.rule_index {
                                ui.label(
                                    egui::RichText::new(format!("Yes (Rule {})", rule_idx))
                                        .color(egui::Color32::GREEN),
                                );
                            } else {
                                ui.label("No");
                            }
                        });
                        row.col(|ui| {
                            let transitions: Vec<String> = state
                                .transitions
                                .iter()
                                .map(|t| match t {
                                    openlexer_lib::lexgen::nfa::Transition::Char(c, target) => {
                                        format!("'{}' -> {}", c.escape_debug(), target)
                                    }
                                    openlexer_lib::lexgen::nfa::Transition::CharRange(
                                        s,
                                        e,
                                        target,
                                    ) => {
                                        let starts = std::char::from_u32(*s).unwrap_or('?');
                                        let ends = std::char::from_u32(*e).unwrap_or('?');
                                        format!(
                                            "'{}'-'{}' -> {}",
                                            starts.escape_debug(),
                                            ends.escape_debug(),
                                            target
                                        )
                                    },
                                    openlexer_lib::lexgen::nfa::Transition::Epsilon(target) => {
                                        format!("ε -> {}", target)
                                    }
                                })
                                .collect();
                            ui.label(transitions.join(", "));
                        });
                    });
                }
            });
    }

    fn render_lexer_debug_panel(&mut self, ui: &mut egui::Ui, _available: egui::Vec2) {
        let dfa = match self.cached_dfa.clone() {
            Some(d) => d,
            None => {
                ui.label("DFA not available (generate lexer first).");
                return;
            }
        };
        let spec = match self.cached_spec.clone() {
            Some(s) => s,
            None => {
                ui.label("Spec not available.");
                return;
            }
        };

        // Initialize debugger if not present
        if self.lexer_debugger.is_none() {
            self.lexer_debugger = Some(LexerDebugger::new(dfa, spec, &self.debug_input));
        }

        let debugger = self.lexer_debugger.as_mut().unwrap();

        ui.horizontal(|ui| {
            ui.label("Input:");
            if ui.text_edit_singleline(&mut self.debug_input).changed() {
                debugger.reset_with_input(&self.debug_input);
                self.lexer_debug_steps.clear();
            }
        });

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("Step Char").clicked() {
                if let Some(step) = debugger.step() {
                    self.lexer_debug_steps.push(step);
                }
            }
            if ui.button("Step Token").clicked() {
                let steps = debugger.step_token();
                self.lexer_debug_steps.extend(steps);
            }
            if ui.button("Run All").clicked() {
                let steps = debugger.run_all();
                self.lexer_debug_steps.extend(steps);
            }
            if ui.button("Reset").clicked() {
                debugger.reset();
                self.lexer_debug_steps.clear();
            }

            ui.add_space(20.0);
            if debugger.is_finished() {
                ui.label(egui::RichText::new("Finished").color(egui::Color32::GREEN));
            } else {
                ui.label(format!(
                    "Pos: {}, State: {}",
                    debugger.current_position(),
                    debugger.current_state_id()
                ));
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            TableBuilder::new(ui)
                .striped(true)
                .column(Column::initial(40.0)) // Char
                .column(Column::initial(80.0)) // From -> To
                .column(Column::initial(100.0)) // Is Accepting
                .column(Column::remainder()) // Token Info
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("Char");
                    });
                    header.col(|ui| {
                        ui.strong("State");
                    });
                    header.col(|ui| {
                        ui.strong("Status");
                    });
                    header.col(|ui| {
                        ui.strong("Token");
                    });
                })
                .body(|mut body| {
                    for step in &self.lexer_debug_steps {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                ui.label(format!("'{}'", step.current_char.escape_debug()));
                            });
                            row.col(|ui| {
                                if let Some(to) = step.to_state {
                                    ui.label(format!("{} -> {}", step.from_state, to));
                                } else {
                                    ui.label(format!("{} -> (dead)", step.from_state));
                                }
                            });
                            row.col(|ui| {
                                if step.token_completed {
                                    ui.label(
                                        egui::RichText::new("TOKEN EMIT")
                                            .color(egui::Color32::from_rgb(100, 200, 255))
                                            .strong(),
                                    );
                                } else if step.is_accepting {
                                    ui.label(
                                        egui::RichText::new("Accepts").color(egui::Color32::GREEN),
                                    );
                                } else if step.to_state.is_none() {
                                    ui.label(
                                        egui::RichText::new("Dead State").color(egui::Color32::RED),
                                    );
                                } else {
                                    ui.label("-");
                                }
                            });
                            row.col(|ui| {
                                if step.token_completed {
                                    if let Some(name) = &step.token_name {
                                        ui.label(format!(
                                            "{} (\"{}\")",
                                            name,
                                            step.current_lexeme.escape_debug()
                                        ));
                                    } else {
                                        ui.label("(token)"); // Fallback
                                    }
                                } else if step.is_accepting {
                                    if let Some(rule) = step.rule_index {
                                        ui.label(format!("Rule {}", rule));
                                    }
                                }
                            });
                        });
                    }
                });
        });
    }

    fn render_lalr_table(&mut self, ui: &mut egui::Ui) {
        let table = match &self.cached_parsing_table {
            Some(t) => t,
            None => {
                ui.label("Parsing table not generated yet.");
                return;
            }
        };

        ui.label(egui::RichText::new("LALR(1) Parsing Table").strong().size(16.0));
        ui.add_space(5.0);

        // Collect all symbols (terminals and non-terminals) used in the table
        let mut symbols: Vec<String> = Vec::new();
        for row in table.action.values() {
            for k in row.keys() {
                if !symbols.contains(k) {
                    symbols.push(k.clone());
                }
            }
        }
        for row in table.goto.values() {
            for k in row.keys() {
                if !symbols.contains(k) {
                    symbols.push(k.clone());
                }
            }
        }
        symbols.sort();

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(Column::initial(50.0).resizable(true)) // State ID
            .columns(Column::initial(60.0).resizable(true), symbols.len())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("State");
                });
                for sym in &symbols {
                    header.col(|ui| {
                        ui.label(egui::RichText::new(sym).strong());
                    });
                }
            })
            .body(|mut body| {
                // Determine max state ID
                let max_action = table.action.keys().max().copied().unwrap_or(0);
                let max_goto = table.goto.keys().max().copied().unwrap_or(0);
                let max_state = max_action.max(max_goto);

                for state_id in 0..=max_state {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.label(state_id.to_string());
                        });

                        for sym in &symbols {
                            row.col(|ui| {
                                // check action
                                if let Some(action_row) = table.action.get(&state_id) {
                                    if let Some(action) = action_row.get(sym) {
                                        let text = match action {
                                            openlexer_lib::parsegen::lalr::Action::Shift(s) => {
                                                format!("s{}", s)
                                            }
                                            openlexer_lib::parsegen::lalr::Action::Reduce(r) => {
                                                format!("r{}", r)
                                            }
                                            openlexer_lib::parsegen::lalr::Action::Accept => {
                                                "acc".to_string()
                                            }
                                        };
                                        ui.label(text);
                                        return;
                                    }
                                }
                                // check goto
                                if let Some(goto_row) = table.goto.get(&state_id) {
                                    if let Some(target) = goto_row.get(sym) {
                                        ui.label(target.to_string());
                                        return;
                                    }
                                }
                            });
                        }
                    });
                }
            });
    }

    fn render_glr_conflicts(&mut self, ui: &mut egui::Ui) {
        let table = match &self.cached_parsing_table {
            Some(t) => t,
            None => {
                ui.label("Parsing table not generated yet.");
                return;
            }
        };

        ui.label(egui::RichText::new("GLR Conflict Information").strong().size(16.0));
        ui.add_space(5.0);
        ui.label("These are states where Shift/Reduce or Reduce/Reduce conflicts occur. In GLR mode, the parser forks at these points to explore both paths.");
        ui.add_space(10.0);

        if table.glr_conflict_actions.is_empty() {
            ui.label(
                egui::RichText::new("No conflicts found (grammar is LALR(1)).")
                    .color(egui::Color32::GREEN),
            );
            return;
        }

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(Column::initial(50.0)) // State
            .column(Column::initial(80.0)) // Symbol
            .column(Column::remainder()) // Actions
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("State");
                });
                header.col(|ui| {
                    ui.strong("Symbol");
                });
                header.col(|ui| {
                    ui.strong("Conflicting Actions");
                });
            })
            .body(|mut body| {
                for (state_id, row) in &table.glr_conflict_actions {
                    for (symbol, actions) in row {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                ui.label(state_id.to_string());
                            });
                            row.col(|ui| {
                                ui.label(symbol);
                            });
                            row.col(|ui| {
                                let action_strs: Vec<String> = actions
                                    .iter()
                                    .map(|a| match a {
                                        openlexer_lib::parsegen::lalr::Action::Shift(s) => {
                                            format!("Shift {}", s)
                                        }
                                        openlexer_lib::parsegen::lalr::Action::Reduce(r) => {
                                            format!("Reduce {}", r)
                                        }
                                        openlexer_lib::parsegen::lalr::Action::Accept => {
                                            "Accept".to_string()
                                        }
                                    })
                                    .collect();
                                ui.label(
                                    egui::RichText::new(action_strs.join(" / "))
                                        .color(egui::Color32::RED),
                                );
                            });
                        });
                    }
                }
            });
    }

    fn render_parser_debug_panel(&mut self, ui: &mut egui::Ui) {
        let table = match self.cached_parsing_table.clone() {
            Some(t) => t,
            None => {
                ui.label("Parsing table not available (generate parser first).");
                return;
            }
        };
        let grammar = match self.cached_grammar.clone() {
            Some(g) => g,
            None => {
                ui.label("Grammar not available.");
                return;
            }
        };

        ui.horizontal(|ui| {
            ui.label("Source Code:");
            if ui.text_edit_singleline(&mut self.debug_input).changed() {
                self.parser_debugger = None;
                self.parser_debug_steps.clear();
            }
        });

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("Start / Reset").clicked() {
                // Use the generated lexer to tokenize the input if available
                let tokens = self.tokenize_for_parser_debug(&self.debug_input);
                
                self.parser_debugger = Some(ParserDebugger::new(table.clone(), grammar.clone(), tokens));
                self.parser_debug_steps.clear();
            }

            if let Some(debugger) = &mut self.parser_debugger {
                if ui.button("Step").clicked() {
                    if let Some(step) = debugger.step() {
                        self.parser_debug_steps.push(step);
                    }
                }
                if ui.button("Run All").clicked() {
                    let steps = debugger.run_all();
                    self.parser_debug_steps.extend(steps);
                }

                ui.add_space(20.0);
                if debugger.is_finished() {
                    if debugger.accepted {
                         ui.label(egui::RichText::new("Accepted").color(egui::Color32::GREEN));
                    } else if let Some(err) = &debugger.error {
                         ui.label(egui::RichText::new(format!("Error: {}", err)).color(egui::Color32::RED));
                    }
                } else {
                    ui.label(format!("Token: {} / {}", debugger.current_token_pos(), debugger.token_count()));
                }
            } else {
                 ui.label("(Click Start to begin)");
            }
        });

        ui.separator();
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .column(Column::initial(40.0)) // Step
                .column(Column::initial(100.0)) // Action
                .column(Column::initial(80.0)) // Lookahead
                .column(Column::initial(150.0)) // Stack
                .column(Column::remainder()) // Rule Reduced
                .header(20.0, |mut header| {
                    header.col(|ui| { ui.strong("#"); });
                    header.col(|ui| { ui.strong("Action"); });
                    header.col(|ui| { ui.strong("Lookahead"); });
                    header.col(|ui| { ui.strong("Stack"); });
                    header.col(|ui| { ui.strong("Reduction"); });
                })
                .body(|mut body| {
                    for step in &self.parser_debug_steps {
                        body.row(20.0, |mut row| {
                            row.col(|ui| { ui.label(step.step_number.to_string()); });
                            row.col(|ui| { 
                                let color = if step.action_description.starts_with("Error") {
                                    egui::Color32::RED
                                } else if step.action_description == "Accept" {
                                    egui::Color32::GREEN
                                } else {
                                    ui.visuals().text_color()
                                };
                                ui.label(egui::RichText::new(&step.action_description).color(color)); 
                            });
                            row.col(|ui| { ui.label(&step.lookahead); });
                            row.col(|ui| { 
                                let stack_str = step.stack_states.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(" ");
                                ui.label(stack_str); 
                            });
                            row.col(|ui| { 
                                if let Some(rule) = &step.reduced_rule {
                                    ui.label(rule);
                                }
                            });
                        });
                    }
                });
        });
    }

    fn render_parser_tree(&mut self, ui: &mut egui::Ui) {
        let debugger = match &self.parser_debugger {
            Some(d) => d,
            None => {
                ui.label("Tree is empty. Please go to the 'Debug' tab and run the parser (Step or Run All) to generate the tree.");
                return;
            }
        };

        // Determine roots
        let roots = if let Some(root) = debugger.get_tree_root() {
            vec![root]
        } else {
            // Show stack forest if incomplete
            debugger.node_stack.clone()
        };

        if roots.is_empty() {
             ui.label("Tree is empty. Try stepping or running the parser in the 'Debug' tab.");
             return;
        }

        egui::ScrollArea::both().show(ui, |ui| {
            // Build layout for all roots
            let mut root_layouts: Vec<TreeNodeLayout> = roots.iter().map(|&id| TreeNodeLayout::new(id, debugger)).collect();
            
            // Calculate total width and positioning
            let spacing = 40.0;
            let total_width: f32 = root_layouts.iter().map(|l| l.width).sum::<f32>() 
                                 + (root_layouts.len().saturating_sub(1) as f32 * spacing);
            
            // Assign positions
            let mut current_x = 0.0;
            for layout in &mut root_layouts {
                layout.assign_positions(current_x, 20.0);
                current_x += layout.width + spacing;
            }
            
            // Determine canvas size
            // For now, use a fixed large height to accommodate most trees.
            // A more advanced implementation would traverse layouts to find max_y.
            let total_height = 2000.0;
            
            // Just allocate a large enough canvas for now, or use ui.min_rect() after drawing?
            // Painter draws relative to a rect.
            let (response, painter) = ui.allocate_painter(
                egui::vec2(total_width + 100.0, total_height), 
                egui::Sense::hover()
            );
            
            let offset = response.rect.min.to_vec2() + egui::vec2(50.0, 0.0);
            
            for layout in &root_layouts {
                layout.draw(ui, &painter, offset, debugger);
            }
        });
    }

    fn tokenize_for_parser_debug(&self, input: &str) -> Vec<openlexer_lib::debug::ParserToken> {
        // If we have a cached DFA and LexerSpec, run the lexer
        if let (Some(dfa), Some(spec)) = (&self.cached_dfa, &self.cached_spec) {
            let mut lexer = openlexer_lib::debug::LexerDebugger::new(dfa.clone(), spec.clone(), input);
            let _ = lexer.run_all();
            
            lexer.tokens.iter().map(|t| openlexer_lib::debug::ParserToken {
                token_type: t.token_type.clone(),
                value: t.lexeme.clone(),
            }).collect()
        } else {
             // Fallback
             input.split_whitespace().map(|s| openlexer_lib::debug::ParserToken {
                token_type: s.to_string(),
                value: s.to_string(),
             }).collect()
        }
    }

    fn render_logs_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Logs").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Clear").clicked() {
                    self.logs.clear();
                    self.log(LogLevel::Info, "Logs cleared");
                }
            });
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("logs_scroll")
            .stick_to_bottom(true)
            .max_height(150.0)
            .show(ui, |ui| {
                for entry in &self.logs {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(&entry.timestamp)
                                .monospace()
                                .small()
                                .color(egui::Color32::GRAY),
                        );
                        ui.label(
                            egui::RichText::new(format!("[{}]", entry.level.prefix()))
                                .monospace()
                                .small()
                                .color(entry.level.color()),
                        );
                        ui.label(egui::RichText::new(&entry.message).small());
                    });
                }
            });
    }

    fn render_status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(ref err) = self.error {
                ui.label(egui::RichText::new(err).color(egui::Color32::from_rgb(255, 100, 100)));
            } else {
                ui.label(&self.status);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} | {}",
                        self.language.display_name(),
                        self.current_tab.label()
                    ))
                    .small()
                    .color(egui::Color32::GRAY),
                );
            });
        });
    }

    fn render_download_button(&mut self, ui: &mut egui::Ui, prefix: &str, content: &str) {
        if content.is_empty() {
            ui.add_enabled(false, egui::Button::new("Download"));
            return;
        }

        if ui.button("Download").clicked() {
            let filename = format!("{}{}", prefix, self.language.extension());

            #[cfg(target_arch = "wasm32")]
            {
                web_utils::download_file(&filename, content);
                self.status = format!("Downloaded {}", filename);
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name(&filename)
                    .add_filter(
                        "Source Code",
                        &[self.language.extension().trim_start_matches('.')],
                    )
                    .save_file()
                {
                    if let Err(e) = std::fs::write(&path, content) {
                        self.error = Some(format!("Failed to save: {}", e));
                    } else {
                        self.status = format!("Saved to {}", path.display());
                        self.log(LogLevel::Success, &self.status.clone());
                    }
                }
            }
        }
    }
}

impl eframe::App for OpenLexerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for async code execution results
        if self.is_running {
            self.poll_run_result();
        }
        ctx.request_repaint();

        // Apply theme
        if self.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }

        // Header
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            self.render_header(ui);
        });

        // Status bar (always at very bottom)
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            self.render_status_bar(ui);
        });

        // Logs panel (resizable, above status bar)
        if self.show_logs {
            egui::TopBottomPanel::bottom("logs")
                .resizable(true)
                .default_height(120.0)
                .min_height(60.0)
                .max_height(400.0)
                .show(ctx, |ui| {
                    self.render_logs_panel(ui);
                });
        }

        // Lexer tab: use native resizable panels
        if self.current_tab == MainTab::Lexer {
            // Run panel at the bottom (resizable up/down)
            egui::TopBottomPanel::bottom("run_panel")
                .resizable(true)
                .default_height(180.0)
                .min_height(80.0)
                .max_height(600.0)
                .show(ctx, |ui| {
                    self.render_run_panel(ui);
                });

            // Spec editor on the left (resizable left/right)
            egui::SidePanel::left("spec_panel")
                .resizable(true)
                .default_width(ctx.screen_rect().width() * 0.45)
                .min_width(200.0)
                .max_width(ctx.screen_rect().width() * 0.75)
                .show(ctx, |ui| {
                    self.render_spec_editor(ui);
                });

            // Generated code fills the remaining center
            egui::CentralPanel::default().show(ctx, |ui| {
                self.render_code_output(ui);
            });
        } else if self.current_tab == MainTab::Parser {
            // Parser tab: same 3-panel layout as lexer
            egui::TopBottomPanel::bottom("parser_run_panel")
                .resizable(true)
                .default_height(180.0)
                .min_height(80.0)
                .max_height(600.0)
                .show(ctx, |ui| {
                    self.render_parser_run_panel(ui);
                });

            // Grammar editor on the left (resizable left/right)
            egui::SidePanel::left("parser_spec_panel")
                .resizable(true)
                .default_width(ctx.screen_rect().width() * 0.45)
                .min_width(200.0)
                .max_width(ctx.screen_rect().width() * 0.75)
                .show(ctx, |ui| {
                    self.render_parser_editor(ui);
                });

            // Generated parser code fills the remaining center
            egui::CentralPanel::default().show(ctx, |ui| {
                self.render_parser_output(ui);
            });
        } else if self.current_tab == MainTab::Try {
            // Try tab: run panel at bottom, full-width editor above
            egui::TopBottomPanel::bottom("try_run_panel")
                .resizable(true)
                .default_height(180.0)
                .min_height(80.0)
                .max_height(600.0)
                .show(ctx, |ui| {
                    self.render_try_run_panel(ui);
                });

            // User's test program fills the center
            egui::CentralPanel::default().show(ctx, |ui| {
                self.render_try_editor(ui);
            });
        } else {
            // Help tab
            egui::CentralPanel::default().show(ctx, |ui| match self.current_tab {
                MainTab::Lexer => unreachable!(),
                MainTab::Parser => unreachable!(),
                MainTab::Try => unreachable!(),
                MainTab::Help => self.render_help_tab(ui),
            });
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn code_editor_with_lines(ui: &mut egui::Ui, id: &str, text: &mut String, _width: f32, _height: f32) {
    let line_count = text.lines().count().max(1);
    let avail = ui.available_size();
    let w = avail.x.max(200.0);
    let h = avail.y.max(80.0);
    let line_number_width = 50.0;
    let text_width = (w - line_number_width - 30.0).max(100.0);

    egui::Frame::none()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .rounding(4.0)
        .inner_margin(4.0)
        .show(ui, |ui: &mut egui::Ui| {
            ui.set_max_size(egui::vec2(w - 8.0, h - 8.0));

            egui::ScrollArea::both()
                .id_salt(id)
                .auto_shrink([true, false])
                .max_height(h - 16.0)
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        let line_numbers: String =
                            (1..=line_count.max(25)).map(|i| format!("{:>4}\n", i)).collect();
                        ui.add(egui::Label::new(
                            egui::RichText::new(line_numbers.trim_end())
                                .monospace()
                                .color(egui::Color32::from_rgb(100, 100, 120)),
                        ));
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);

                        ui.add(
                            egui::TextEdit::multiline(text)
                                .font(egui::TextStyle::Monospace)
                                .code_editor()
                                .desired_width(text_width),
                        );
                    });
                });
        });
}

fn code_viewer_with_lines(ui: &mut egui::Ui, id: &str, text: &str, _width: f32, _height: f32) {
    let line_count = text.lines().count().max(1);
    let avail = ui.available_size();
    let w = avail.x.max(200.0);
    let h = avail.y.max(80.0);
    let line_number_width = 50.0;
    let text_width = (w - line_number_width - 30.0).max(100.0);

    egui::Frame::none()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .rounding(4.0)
        .inner_margin(4.0)
        .show(ui, |ui: &mut egui::Ui| {
            ui.set_max_size(egui::vec2(w - 8.0, h - 8.0));

            egui::ScrollArea::both()
                .id_salt(id)
                .auto_shrink([true, false])
                .max_height(h - 16.0)
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        let line_numbers: String =
                            (1..=line_count.max(25)).map(|i| format!("{:>4}\n", i)).collect();
                        ui.add(egui::Label::new(
                            egui::RichText::new(line_numbers.trim_end())
                                .monospace()
                                .color(egui::Color32::from_rgb(100, 100, 120)),
                        ));
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);

                        let mut text_copy = text.to_string();
                        ui.add(
                            egui::TextEdit::multiline(&mut text_copy)
                                .font(egui::TextStyle::Monospace)
                                .code_editor()
                                .desired_width(text_width)
                                .interactive(true),
                        );
                    });
                });
        });
}

#[cfg(not(target_arch = "wasm32"))]
fn chrono_lite_time() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let hours = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    let seconds = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, seconds)
}

#[cfg(target_arch = "wasm32")]
fn chrono_lite_time() -> String {
    let date = js_sys::Date::new_0();
    format!(
        "{:02}:{:02}:{:02}",
        date.get_hours(),
        date.get_minutes(),
        date.get_seconds()
    )
}

// ============================================================================
// Sample Inputs
// ============================================================================

const SAMPLE_LEXER: &str = r#"%{
/* Simple Calculator Lexer */
%}

%%
[0-9]+      { return NUMBER; }
[a-zA-Z_][a-zA-Z0-9_]*   { return IDENTIFIER; }
"+"         { return PLUS; }
"-"         { return MINUS; }
"*"         { return TIMES; }
"/"         { return DIVIDE; }
"("         { return LPAREN; }
")"         { return RPAREN; }
"="         { return ASSIGN; }
";"         { return SEMICOLON; }
[ \t\n]+    { /* skip whitespace */ }
.           { return ERROR; }
%%
"#;

const SAMPLE_LEXER_ADVANCED: &str = r#"%{
/* Advanced Lexer with Start Conditions and Unicode */
%}

%x COMMENT
%x STRING

%%
"//".*           { /* skip line comment */ }
"/*"             { BEGIN(COMMENT); }
<COMMENT>"*/"    { BEGIN(INITIAL); }
<COMMENT>.|\n    { /* skip comment content */ }

\"               { BEGIN(STRING); }
<STRING>\"       { BEGIN(INITIAL); return STRING_LITERAL; }
<STRING>\\.      { /* escape sequence */ }
<STRING>.        { /* string content */ }

[0-9]+           { return INTEGER; }
[0-9]+\.[0-9]+   { return FLOAT; }
0x[0-9a-fA-F]+   { return HEX; }

[a-zA-Z_][a-zA-Z0-9_]*   { return IDENTIFIER; }

"+"              { return PLUS; }
"-"              { return MINUS; }
"*"              { return STAR; }
"/"              { return SLASH; }
"=="             { return EQ; }
"!="             { return NEQ; }
"<="             { return LE; }
">="             { return GE; }
"<"              { return LT; }
">"              { return GT; }
"&&"             { return AND; }
"||"             { return OR; }
"!"              { return NOT; }

"("              { return LPAREN; }
")"              { return RPAREN; }
"{"              { return LBRACE; }
"}"              { return RBRACE; }
"["              { return LBRACKET; }
"]"              { return RBRACKET; }
";"              { return SEMI; }
","              { return COMMA; }
"."              { return DOT; }

[ \t\r\n]+       { /* skip whitespace */ }
.                { return ERROR; }
%%
"#;

const SAMPLE_PARSER: &str = r#"%token NUMBER IDENTIFIER PLUS MINUS TIMES DIVIDE LPAREN RPAREN

%left PLUS MINUS
%left TIMES DIVIDE

%%

expr:
    expr PLUS term   { $$ = $1 + $3; }
  | expr MINUS term  { $$ = $1 - $3; }
  | term             { $$ = $1; }
  ;

term:
    term TIMES factor { $$ = $1 * $3; }
  | term DIVIDE factor { $$ = $1 / $3; }
  | factor            { $$ = $1; }
  ;

factor:
    LPAREN expr RPAREN { $$ = $2; }
  | NUMBER             { $$ = $1; }
  | IDENTIFIER         { $$ = lookup($1); }
  ;

%%
"#;

const SAMPLE_PARSER_AMBIGUOUS: &str = r#"%token IF THEN ELSE EXPR STMT

/* This grammar is ambiguous (dangling else problem)
   GLR mode will handle both possible parses */

%%

stmt:
    IF EXPR THEN stmt ELSE stmt  { $$ = if_else($2, $4, $6); }
  | IF EXPR THEN stmt            { $$ = if_then($2, $4); }
  | EXPR                         { $$ = $1; }
  ;

%%
"#;

// ============================================================================
// WASM Entry Point
// ============================================================================

#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect traps to console.error
    console_error_panic_hook::set_once();

    wasm_bindgen_futures::spawn_local(async {
        let web_options = eframe::WebOptions::default();

        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("openlexer_canvas")
            .expect("No canvas element with id 'openlexer_canvas'")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("Element is not a canvas");

        if let Some(loading) = document.get_element_by_id("loading") {
            let _ = loading.set_attribute("style", "display: none !important;");
        }

        eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(OpenLexerApp::new(cc)))),
            )
            .await
            .expect("failed to start eframe");
    });
}



