//! OpenLexer GUI - egui-based interface for lexer/parser generation
//! Compiles to both native desktop and WASM for web browsers

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use openlexer_lib::{lexgen, parsegen};

// Web-specific imports for download functionality
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
mod web_utils {
    use wasm_bindgen::prelude::*;
    use web_sys::{Blob, BlobPropertyBag, Url, HtmlAnchorElement};
    use wasm_bindgen::JsCast;

    pub fn download_file(filename: &str, content: &str) {
        let window = web_sys::window().expect("no window");
        let document = window.document().expect("no document");
        
        // Create blob from content
        let blob_parts = js_sys::Array::new();
        blob_parts.push(&JsValue::from_str(content));
        
        let mut options = BlobPropertyBag::new();
        options.type_("text/plain;charset=utf-8");
        
        let blob = Blob::new_with_str_sequence_and_options(&blob_parts, &options)
            .expect("failed to create blob");
        
        // Create object URL
        let url = Url::create_object_url_with_blob(&blob)
            .expect("failed to create object URL");
        
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
            .with_inner_size([1200.0, 800.0])
            .with_title("OpenLexer - Lexer & Parser Generator"),
        // Use glow renderer for better cursor support
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    
    eframe::run_native(
        "OpenLexer",
        options,
        Box::new(|cc| Ok(Box::new(OpenLexerApp::new(cc)))),
    )
}

#[derive(Default, PartialEq)]
enum Tab {
    #[default]
    Lexer,
    Parser,
}

#[derive(Default, PartialEq, Clone, Copy)]
enum Language {
    #[default]
    Python,
    C,
    Java,
}

impl Language {
    fn as_str(&self) -> &'static str {
        match self {
            Language::Python => "python",
            Language::C => "c",
            Language::Java => "java",
        }
    }
    
    fn extension(&self) -> &'static str {
        match self {
            Language::Python => ".py",
            Language::C => ".c",
            Language::Java => ".java",
        }
    }
}

/// Code editor with line numbers - single scroll area for synchronized scrolling
fn code_editor_with_lines(
    ui: &mut egui::Ui,
    id: &str,
    text: &mut String,
    width: f32,
    height: f32,
) {
    let line_count = text.lines().count().max(1);
    let line_number_width = 50.0;
    
    egui::ScrollArea::both()
        .id_salt(id)
        .max_height(height)
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                // Line numbers - rendered as a single text block for proper alignment
                let line_numbers: String = (1..=line_count)
                    .map(|i| format!("{:>4}\n", i))
                    .collect();
                
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(line_numbers.trim_end())
                            .monospace()
                            .color(egui::Color32::GRAY)
                    )
                );
                
                ui.add_space(8.0);
                
                // Vertical separator
                ui.separator();
                
                ui.add_space(4.0);
                
                // Code editor
                ui.add(
                    egui::TextEdit::multiline(text)
                        .font(egui::TextStyle::Monospace)
                        .code_editor()
                        .desired_width(width - line_number_width - 30.0)
                        .desired_rows(line_count.max(20))
                );
            });
        });
}

/// Read-only code viewer with line numbers - single scroll area for synchronized scrolling
fn code_viewer_with_lines(
    ui: &mut egui::Ui,
    id: &str,
    text: &str,
    width: f32,
    height: f32,
) {
    let line_count = text.lines().count().max(1);
    let line_number_width = 50.0;
    
    egui::ScrollArea::both()
        .id_salt(id)
        .max_height(height)
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                // Line numbers - rendered as a single text block for proper alignment
                let line_numbers: String = (1..=line_count)
                    .map(|i| format!("{:>4}\n", i))
                    .collect();
                
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(line_numbers.trim_end())
                            .monospace()
                            .color(egui::Color32::GRAY)
                    )
                );
                
                ui.add_space(8.0);
                
                // Vertical separator
                ui.separator();
                
                ui.add_space(4.0);
                
                // Code viewer (read-only)
                let mut text_copy = text.to_string();
                ui.add(
                    egui::TextEdit::multiline(&mut text_copy)
                        .font(egui::TextStyle::Monospace)
                        .code_editor()
                        .desired_width(width - line_number_width - 30.0)
                        .desired_rows(line_count.max(20))
                        .interactive(true) // Allow selection/copy
                );
            });
        });
}

/// Log level for messages
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
            LogLevel::Info => egui::Color32::LIGHT_BLUE,
            LogLevel::Success => egui::Color32::GREEN,
            LogLevel::Warning => egui::Color32::YELLOW,
            LogLevel::Error => egui::Color32::RED,
        }
    }
    
    fn prefix(&self) -> &'static str {
        match self {
            LogLevel::Info => "[INFO]",
            LogLevel::Success => "[OK]",
            LogLevel::Warning => "[WARN]",
            LogLevel::Error => "[ERROR]",
        }
    }
}

/// A single log entry
#[derive(Clone)]
struct LogEntry {
    timestamp: String,
    level: LogLevel,
    message: String,
}

struct OpenLexerApp {
    // Current tab
    tab: Tab,
    
    // Lexer input
    lexer_input: String,
    
    // Parser input
    parser_input: String,
    
    // Output language
    language: Language,
    
    // Generated output
    output: String,
    
    // Status message
    status: String,
    
    // Error message
    error: Option<String>,
    
    // Log entries
    logs: Vec<LogEntry>,
    
    // Show logs panel
    show_logs: bool,
}

impl OpenLexerApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            tab: Tab::default(),
            lexer_input: SAMPLE_LEXER.to_string(),
            parser_input: SAMPLE_PARSER.to_string(),
            language: Language::default(),
            output: String::new(),
            status: "Ready".to_string(),
            error: None,
            logs: Vec::new(),
            show_logs: false,
        };
        app.log(LogLevel::Info, "OpenLexer initialized");
        app.log(LogLevel::Info, "Ready to generate lexers and parsers");
        app
    }
    
    fn log(&mut self, level: LogLevel, message: &str) {
        let now = chrono_lite_time();
        self.logs.push(LogEntry {
            timestamp: now,
            level,
            message: message.to_string(),
        });
        // Keep only last 500 logs
        if self.logs.len() > 500 {
            self.logs.remove(0);
        }
    }
    
    fn generate_lexer(&mut self) {
        self.error = None;
        self.status = "Generating lexer...".to_string();
        self.log(LogLevel::Info, &format!("Starting lexer generation for {}", self.language.as_str()));
        
        let input_lines = self.lexer_input.lines().count();
        self.log(LogLevel::Info, &format!("Parsing lexer specification ({} lines)", input_lines));
        
        match lexgen::parse_lexer_spec(&self.lexer_input) {
            Ok(spec) => {
                self.log(LogLevel::Success, &format!("Parsed {} rules from specification", spec.rules.len()));
                self.log(LogLevel::Info, "Generating code...");
                
                match lexgen::generate_code(&spec, self.language.as_str()) {
                    Ok(code) => {
                        let output_lines = code.lines().count();
                        self.output = code;
                        self.status = format!("Lexer generated successfully for {}", 
                            self.language.as_str());
                        self.log(LogLevel::Success, &format!("Generated {} lines of {} code", output_lines, self.language.as_str()));
                    }
                    Err(e) => {
                        self.error = Some(format!("Code generation error: {}", e));
                        self.status = "Generation failed".to_string();
                        self.log(LogLevel::Error, &format!("Code generation failed: {}", e));
                    }
                }
            }
            Err(e) => {
                self.error = Some(format!("Parse error: {}", e));
                self.status = "Parse failed".to_string();
                self.log(LogLevel::Error, &format!("Parse error: {}", e));
            }
        }
    }
    
    fn generate_parser(&mut self) {
        self.error = None;
        self.status = "Generating parser...".to_string();
        self.log(LogLevel::Info, &format!("Starting parser generation for {}", self.language.as_str()));
        
        let input_lines = self.parser_input.lines().count();
        self.log(LogLevel::Info, &format!("Parsing grammar specification ({} lines)", input_lines));
        
        match parsegen::parse_grammar(&self.parser_input) {
            Ok(grammar) => {
                self.log(LogLevel::Success, &format!("Parsed {} grammar rules", grammar.rules.len()));
                self.log(LogLevel::Info, &format!("Found {} tokens", grammar.tokens.len()));
                self.log(LogLevel::Info, "Building LALR(1) parsing tables...");
                
                match parsegen::generate_code(&grammar, self.language.as_str()) {
                    Ok(code) => {
                        let output_lines = code.lines().count();
                        self.output = code;
                        self.status = format!("Parser generated successfully for {} ({} rules)", 
                            self.language.as_str(),
                            grammar.rules.len());
                        self.log(LogLevel::Success, &format!("Generated {} lines of {} code", output_lines, self.language.as_str()));
                    }
                    Err(e) => {
                        self.error = Some(format!("Code generation error: {}", e));
                        self.status = "Generation failed".to_string();
                        self.log(LogLevel::Error, &format!("Code generation failed: {}", e));
                    }
                }
            }
            Err(e) => {
                self.error = Some(format!("Grammar error: {}", e));
                self.status = "Parse failed".to_string();
                self.log(LogLevel::Error, &format!("Grammar parse error: {}", e));
            }
        }
    }
}

/// Simple timestamp function (no external deps)
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

impl eframe::App for OpenLexerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let screen_rect = ctx.screen_rect();
        let _is_narrow = screen_rect.width() < 800.0;
        let is_mobile = screen_rect.width() < 500.0;
        
        // Top panel with tabs and controls - wraps on small screens
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            if is_mobile {
                // Mobile: stack controls vertically
                ui.vertical_centered(|ui| {
                    ui.heading("OpenLexer");
                    ui.add_space(4.0);
                    
                    ui.horizontal_wrapped(|ui| {
                        ui.selectable_value(&mut self.tab, Tab::Lexer, "Lexer");
                        ui.selectable_value(&mut self.tab, Tab::Parser, "Parser");
                        
                        egui::ComboBox::from_id_salt("lang")
                            .selected_text(self.language.as_str())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.language, Language::Python, "Python");
                                ui.selectable_value(&mut self.language, Language::C, "C");
                                ui.selectable_value(&mut self.language, Language::Java, "Java");
                            });
                    });
                    
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("▶ Generate").clicked() {
                            match self.tab {
                                Tab::Lexer => self.generate_lexer(),
                                Tab::Parser => self.generate_parser(),
                            }
                        }
                        
                        if ui.button("✖ Clear").clicked() {
                            self.output.clear();
                            self.error = None;
                            self.log(LogLevel::Info, "Output cleared");
                        }
                        
                        if !self.output.is_empty() {
                            self.render_download_button(ui);
                        }
                        
                        let log_btn_text = if self.show_logs { "📋 Hide" } else { "📋 Logs" };
                        if ui.button(log_btn_text).clicked() {
                            self.show_logs = !self.show_logs;
                        }
                    });
                });
            } else {
                // Desktop: horizontal layout
                ui.horizontal_wrapped(|ui| {
                    ui.heading("OpenLexer");
                    ui.separator();
                    
                    ui.selectable_value(&mut self.tab, Tab::Lexer, "Lexer (.l)");
                    ui.selectable_value(&mut self.tab, Tab::Parser, "Parser (.y)");
                    
                    ui.separator();
                    
                    egui::ComboBox::from_label("Target")
                        .selected_text(self.language.as_str())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.language, Language::Python, "Python");
                            ui.selectable_value(&mut self.language, Language::C, "C");
                            ui.selectable_value(&mut self.language, Language::Java, "Java");
                        });
                    
                    ui.separator();
                    
                    if ui.button("▶ Generate").clicked() {
                        match self.tab {
                            Tab::Lexer => self.generate_lexer(),
                            Tab::Parser => self.generate_parser(),
                        }
                    }
                    
                    if ui.button("✖ Clear").clicked() {
                        self.output.clear();
                        self.error = None;
                        self.log(LogLevel::Info, "Output cleared");
                    }
                    
                    if !self.output.is_empty() {
                        ui.separator();
                        self.render_download_button(ui);
                    }
                    
                    ui.separator();
                    let log_btn_text = if self.show_logs { "📋 Hide Logs" } else { "📋 Logs" };
                    if ui.button(log_btn_text).clicked() {
                        self.show_logs = !self.show_logs;
                    }
                });
            }
        });
        
        // Logs panel (collapsible, at bottom)
        if self.show_logs {
            egui::TopBottomPanel::bottom("logs_panel")
                .resizable(true)
                .min_height(100.0)
                .max_height(300.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("📋 Logs");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🗑 Clear Logs").clicked() {
                                self.logs.clear();
                                self.log(LogLevel::Info, "Logs cleared");
                            }
                        });
                    });
                    ui.separator();
                    
                    egui::ScrollArea::vertical()
                        .id_salt("logs_scroll")
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for entry in &self.logs {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(&entry.timestamp)
                                            .monospace()
                                            .color(egui::Color32::GRAY)
                                    );
                                    ui.label(
                                        egui::RichText::new(entry.level.prefix())
                                            .monospace()
                                            .color(entry.level.color())
                                    );
                                    ui.label(&entry.message);
                                });
                            }
                        });
                });
        }
        
        // Status panel
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if let Some(ref err) = self.error {
                    ui.colored_label(egui::Color32::RED, err);
                } else {
                    ui.label(&self.status);
                }
            });
        });
        
        // Central panel with responsive layout
        egui::CentralPanel::default().show(ctx, |ui| {
            // Use StripBuilder pattern for reliable sizing
            let available = ui.available_size();
            let is_narrow = available.x < 700.0;
            
            if is_narrow {
                // Vertical layout for narrow screens
                let half_height = (available.y - 60.0) / 2.0;
                
                // Input editor
                ui.horizontal(|ui| {
                    ui.heading(match self.tab {
                        Tab::Lexer => "📝 Lexer Spec",
                        Tab::Parser => "📝 Grammar Spec",
                    });
                });
                
                let input = match self.tab {
                    Tab::Lexer => &mut self.lexer_input,
                    Tab::Parser => &mut self.parser_input,
                };
                
                code_editor_with_lines(ui, "input", input, available.x, half_height);
                
                ui.separator();
                
                // Output header with download
                ui.horizontal(|ui| {
                    ui.heading(format!("📄 Output ({})", self.language.extension()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !self.output.is_empty() {
                            self.render_download_button(ui);
                        }
                    });
                });
                
                code_viewer_with_lines(ui, "output", &self.output, available.x, half_height);
            } else {
                // Side-by-side layout for wide screens
                let half_width = (available.x - 30.0) / 2.0;
                let editor_height = available.y - 50.0;
                
                ui.horizontal_top(|ui| {
                    // Left panel - Input
                    ui.vertical(|ui| {
                        ui.set_width(half_width);
                        
                        ui.heading(match self.tab {
                            Tab::Lexer => "📝 Lexer Specification (.l)",
                            Tab::Parser => "📝 Grammar Specification (.y)",
                        });
                        
                        let input = match self.tab {
                            Tab::Lexer => &mut self.lexer_input,
                            Tab::Parser => &mut self.parser_input,
                        };
                        
                        code_editor_with_lines(ui, "input", input, half_width, editor_height);
                    });
                    
                    ui.separator();
                    
                    // Right panel - Output
                    ui.vertical(|ui| {
                        ui.set_width(half_width);
                        
                        ui.horizontal(|ui| {
                            ui.heading(format!("📄 Generated Code ({})", self.language.extension()));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if !self.output.is_empty() {
                                    self.render_download_button(ui);
                                }
                            });
                        });
                        
                        code_viewer_with_lines(ui, "output", &self.output, half_width, editor_height);
                    });
                });
            }
        });
    }
}

impl OpenLexerApp {
    fn render_download_button(&mut self, ui: &mut egui::Ui) {
        if ui.button("⬇ Download").clicked() {
            let filename = match self.tab {
                Tab::Lexer => format!("lexer{}", self.language.extension()),
                Tab::Parser => format!("parser{}", self.language.extension()),
            };
            
            #[cfg(target_arch = "wasm32")]
            {
                web_utils::download_file(&filename, &self.output);
                self.status = format!("Downloaded {}", filename);
            }
            
            #[cfg(not(target_arch = "wasm32"))]
            {
                // Desktop: use native file dialog if available, otherwise save to current dir
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name(&filename)
                    .add_filter("Source Code", &[self.language.extension().trim_start_matches('.')])
                    .save_file()
                {
                    if let Err(e) = std::fs::write(&path, &self.output) {
                        self.error = Some(format!("Failed to save: {}", e));
                    } else {
                        self.status = format!("Saved to {}", path.display());
                    }
                }
            }
        }
    }
}

const SAMPLE_LEXER: &str = r#"%{
/* Simple Calculator Lexer */
%}

%%
[0-9]+      { return NUMBER; }
[a-zA-Z]+   { return IDENTIFIER; }
"+"         { return PLUS; }
"-"         { return MINUS; }
"*"         { return TIMES; }
"/"         { return DIVIDE; }
"("         { return LPAREN; }
")"         { return RPAREN; }
[ \t\n]+    { /* skip whitespace */ }
.           { return ERROR; }
%%
"#;

const SAMPLE_PARSER: &str = r#"%token NUMBER IDENTIFIER PLUS MINUS TIMES DIVIDE LPAREN RPAREN

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
  | IDENTIFIER         { $$ = $1; }
  ;

%%
"#;

// WASM entry point
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    // Redirect panics to console.error
    console_error_panic_hook::set_once();
    
    let web_options = eframe::WebOptions::default();
    
    wasm_bindgen_futures::spawn_local(async {
        let _ = eframe::WebRunner::new()
            .start(
                "openlexer_canvas",
                web_options,
                Box::new(|cc| Ok(Box::new(OpenLexerApp::new(cc)))),
            )
            .await;
    });
    
    Ok(())
}
