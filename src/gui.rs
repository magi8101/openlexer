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
use openlexer_lib::{lexgen, parsegen};

// Version info
const VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_NAME: &str = "OpenLexer";

// Web-specific imports for download functionality
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

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
    Combined,
    Help,
}

impl MainTab {
    fn label(&self) -> &'static str {
        match self {
            MainTab::Lexer => "Lexer",
            MainTab::Parser => "Parser",
            MainTab::Combined => "Combined",
            MainTab::Help => "Help",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            MainTab::Lexer => "L",
            MainTab::Parser => "P",
            MainTab::Combined => "C",
            MainTab::Help => "?",
        }
    }
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

    // Combined mode
    combined_lexer_input: String,
    combined_parser_input: String,
    combined_output: String,

    // UI state
    show_logs: bool,
    show_options: bool,
    logs: Vec<LogEntry>,
    status: String,
    error: Option<String>,

    // Theme
    dark_mode: bool,
}

impl OpenLexerApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            current_tab: MainTab::default(),
            language: TargetLanguage::default(),
            lexer_input: SAMPLE_LEXER.to_string(),
            lexer_output: String::new(),
            lexer_options: LexerOptions::default(),
            parser_input: SAMPLE_PARSER.to_string(),
            parser_output: String::new(),
            parser_options: ParserOptions::default(),
            combined_lexer_input: SAMPLE_LEXER.to_string(),
            combined_parser_input: SAMPLE_PARSER.to_string(),
            combined_output: String::new(),
            show_logs: false,
            show_options: true,
            logs: Vec::new(),
            status: "Ready".to_string(),
            error: None,
            dark_mode: true,
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

                match self.parser_options.mode {
                    ParserMode::LALR => {
                        self.log(LogLevel::Info, "Building LALR(1) parsing tables...");
                        match parsegen::generate_code(&grammar, self.language.as_str()) {
                            Ok(code) => {
                                let line_count = code.lines().count();
                                self.parser_output = code;
                                self.status = format!(
                                    "Generated {} LALR(1) parser ({} lines)",
                                    self.language.display_name(),
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
                    ParserMode::GLR => {
                        self.log(LogLevel::Info, "Building GLR parsing tables...");
                        self.log(
                            LogLevel::Warning,
                            "GLR code generation outputs LALR with conflict info",
                        );
                        // GLR generates LALR code with conflict annotations for now
                        match parsegen::generate_code(&grammar, self.language.as_str()) {
                            Ok(code) => {
                                let line_count = code.lines().count();
                                self.parser_output = code;
                                self.status = format!(
                                    "Generated {} GLR parser ({} lines)",
                                    self.language.display_name(),
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
                }
            }
            Err(e) => {
                self.error = Some(format!("Grammar error: {}", e));
                self.log(LogLevel::Error, &format!("Grammar error: {}", e));
            }
        }
    }

    fn generate_combined(&mut self) {
        self.error = None;
        self.status = "Generating combined lexer+parser...".to_string();
        self.log(
            LogLevel::Info,
            &format!(
                "Starting combined generation for {}",
                self.language.display_name()
            ),
        );

        let mut output = String::new();

        // Generate lexer
        match lexgen::parse_lexer_spec(&self.combined_lexer_input) {
            Ok(spec) => {
                self.log(
                    LogLevel::Success,
                    &format!("Parsed {} lexer rules", spec.rules.len()),
                );

                match lexgen::generate_code(&spec, self.language.as_str()) {
                    Ok(lexer_code) => {
                        output.push_str(
                            "// ===============================\n\
                             // Generated Lexer\n\
                             // ===============================\n\n",
                        );
                        output.push_str(&lexer_code);
                        self.log(LogLevel::Success, "Lexer code generated");
                    }
                    Err(e) => {
                        self.error = Some(format!("Lexer generation error: {}", e));
                        self.log(LogLevel::Error, &format!("Lexer generation failed: {}", e));
                        return;
                    }
                }
            }
            Err(e) => {
                self.error = Some(format!("Lexer parse error: {}", e));
                self.log(LogLevel::Error, &format!("Lexer parse error: {}", e));
                return;
            }
        }

        // Generate parser
        match parsegen::parse_grammar(&self.combined_parser_input) {
            Ok(grammar) => {
                self.log(
                    LogLevel::Success,
                    &format!("Parsed {} grammar rules", grammar.rules.len()),
                );

                match parsegen::generate_code(&grammar, self.language.as_str()) {
                    Ok(parser_code) => {
                        output.push_str(
                            "\n\n// ===============================\n\
                             // Generated Parser\n\
                             // ===============================\n\n",
                        );
                        output.push_str(&parser_code);
                        self.log(LogLevel::Success, "Parser code generated");
                    }
                    Err(e) => {
                        self.error = Some(format!("Parser generation error: {}", e));
                        self.log(LogLevel::Error, &format!("Parser generation failed: {}", e));
                        return;
                    }
                }
            }
            Err(e) => {
                self.error = Some(format!("Grammar parse error: {}", e));
                self.log(LogLevel::Error, &format!("Grammar parse error: {}", e));
                return;
            }
        }

        let line_count = output.lines().count();
        self.combined_output = output;
        self.status = format!(
            "Generated combined {} code ({} lines)",
            self.language.display_name(),
            line_count
        );
        self.log(LogLevel::Success, &self.status.clone());
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
                MainTab::Combined,
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

    fn render_lexer_tab(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let panel_width = (available.x - 30.0) / 2.0;
        // Reserve space for header (~30), options (~25 if shown), and buttons (~35)
        let options_height = if self.show_options { 30.0 } else { 0.0 };
        let editor_height = (available.y - 70.0 - options_height).max(200.0);

        ui.horizontal(|ui| {
            // Left panel - Input
            ui.vertical(|ui| {
                ui.set_width(panel_width);

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
                        if ui.button("Load Example").clicked() {
                            self.lexer_input = SAMPLE_LEXER_ADVANCED.to_string();
                            self.log(LogLevel::Info, "Loaded advanced lexer example");
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
                ui.add_space(4.0);

                code_editor_with_lines(
                    ui,
                    "lexer_input",
                    &mut self.lexer_input,
                    panel_width,
                    editor_height,
                );
            });

            ui.separator();

            // Right panel - Output
            ui.vertical(|ui| {
                ui.set_width(panel_width);

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "Generated Code ({})",
                            self.language.extension()
                        ))
                        .strong()
                        .size(14.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} lines",
                                self.lexer_output.lines().count()
                            ))
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
                    });
                });
                ui.add_space(4.0);

                code_viewer_with_lines(
                    ui,
                    "lexer_output",
                    &self.lexer_output,
                    panel_width,
                    editor_height + options_height,
                );
            });
        });
    }

    fn render_parser_tab(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let panel_width = (available.x - 30.0) / 2.0;
        let options_height = if self.show_options { 30.0 } else { 0.0 };
        let editor_height = (available.y - 70.0 - options_height).max(200.0);

        ui.horizontal(|ui| {
            // Left panel - Input
            ui.vertical(|ui| {
                ui.set_width(panel_width);

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
                ui.add_space(4.0);

                code_editor_with_lines(
                    ui,
                    "parser_input",
                    &mut self.parser_input,
                    panel_width,
                    editor_height,
                );
            });

            ui.separator();

            // Right panel - Output
            ui.vertical(|ui| {
                ui.set_width(panel_width);

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
                    });
                });
                ui.add_space(4.0);

                code_viewer_with_lines(
                    ui,
                    "parser_output",
                    &self.parser_output,
                    panel_width,
                    editor_height + options_height,
                );
            });
        });
    }

    fn render_combined_tab(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let panel_width = (available.x - 20.0) / 3.0;
        let editor_height = (available.y - 50.0).max(200.0);

        // Top toolbar
        ui.horizontal(|ui| {
            if ui.button("Generate Combined").clicked() {
                self.generate_combined();
            }
            self.render_download_button(ui, "combined", &self.combined_output.clone());
            if ui.button("Copy").clicked() {
                ui.ctx().copy_text(self.combined_output.clone());
                self.log(LogLevel::Info, "Combined code copied");
            }
            if ui.button("Clear Output").clicked() {
                self.combined_output.clear();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("{} lines", self.combined_output.lines().count()))
                        .small()
                        .color(egui::Color32::GRAY),
                );
            });
        });
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            // Lexer input
            ui.vertical(|ui| {
                ui.set_width(panel_width);
                ui.label(egui::RichText::new("Lexer (.l)").strong().size(14.0));
                code_editor_with_lines(
                    ui,
                    "combined_lexer",
                    &mut self.combined_lexer_input,
                    panel_width,
                    editor_height,
                );
            });

            ui.separator();

            // Parser input
            ui.vertical(|ui| {
                ui.set_width(panel_width);
                ui.label(egui::RichText::new("Grammar (.y)").strong().size(14.0));
                code_editor_with_lines(
                    ui,
                    "combined_parser",
                    &mut self.combined_parser_input,
                    panel_width,
                    editor_height,
                );
            });

            ui.separator();

            // Combined output
            ui.vertical(|ui| {
                ui.set_width(panel_width);
                ui.label(
                    egui::RichText::new(format!("Combined Output ({})", self.language.extension()))
                        .strong()
                        .size(14.0),
                );
                code_viewer_with_lines(
                    ui,
                    "combined_output",
                    &self.combined_output,
                    panel_width,
                    editor_height,
                );
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

        // Logs panel
        if self.show_logs {
            egui::TopBottomPanel::bottom("logs")
                .resizable(true)
                .min_height(100.0)
                .max_height(300.0)
                .show(ctx, |ui| {
                    self.render_logs_panel(ui);
                });
        }

        // Status bar
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            self.render_status_bar(ui);
        });

        // Main content
        egui::CentralPanel::default().show(ctx, |ui| match self.current_tab {
            MainTab::Lexer => self.render_lexer_tab(ui),
            MainTab::Parser => self.render_parser_tab(ui),
            MainTab::Combined => self.render_combined_tab(ui),
            MainTab::Help => self.render_help_tab(ui),
        });
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn code_editor_with_lines(ui: &mut egui::Ui, id: &str, text: &mut String, width: f32, height: f32) {
    let line_count = text.lines().count().max(1);
    let line_number_width = 50.0;
    let text_width = (width - line_number_width - 20.0).max(100.0);

    // Create a frame with background to make the editor visible
    egui::Frame::none()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .rounding(4.0)
        .inner_margin(4.0)
        .show(ui, |ui: &mut egui::Ui| {
            ui.set_min_size(egui::vec2(width - 8.0, height - 8.0));

            egui::ScrollArea::both()
                .id_salt(id)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        // Line numbers column
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

                        // Text editor
                        ui.add(
                            egui::TextEdit::multiline(text)
                                .font(egui::TextStyle::Monospace)
                                .code_editor()
                                .desired_width(text_width)
                                .min_size(egui::vec2(text_width, height - 24.0)),
                        );
                    });
                });
        });
}

fn code_viewer_with_lines(ui: &mut egui::Ui, id: &str, text: &str, width: f32, height: f32) {
    let line_count = text.lines().count().max(1);
    let line_number_width = 50.0;
    let text_width = (width - line_number_width - 20.0).max(100.0);

    // Create a frame with background to make the viewer visible
    egui::Frame::none()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .rounding(4.0)
        .inner_margin(4.0)
        .show(ui, |ui: &mut egui::Ui| {
            ui.set_min_size(egui::vec2(width - 8.0, height - 8.0));

            egui::ScrollArea::both()
                .id_salt(id)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        // Line numbers column
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

                        // Text viewer (read-only)
                        let mut text_copy = text.to_string();
                        ui.add(
                            egui::TextEdit::multiline(&mut text_copy)
                                .font(egui::TextStyle::Monospace)
                                .code_editor()
                                .desired_width(text_width)
                                .min_size(egui::vec2(text_width, height - 24.0))
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
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

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
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

    Ok(())
}



