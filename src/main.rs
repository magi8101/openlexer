use clap::{Parser, Subcommand, ValueEnum};
use openlexer_lib::lexgen::{self, LexerSpec, Nfa, Dfa};
use openlexer_lib::parsegen::{self, Grammar, ParsingTable};
use openlexer_lib::lexgen::codegen::TargetLanguage;
use std::path::PathBuf;
use std::fs;
use anyhow::{Context, Result};

#[derive(Parser)]
#[command(name = "openlexer")]
#[command(about = "Flex/Bison replacement for C, Java, Python", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a lexer from rules
    GenLexer {
        /// Path to lexer rules file (.l)
        #[arg(short, long)]
        lexer: PathBuf,

        /// Target language
        #[arg(short = 'L', long, value_enum)]
        lang: Language,

        /// Output directory
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Generate a parser from grammar
    GenParser {
        /// Path to grammar file (.y)
        #[arg(long)]
        parser: PathBuf,

        /// Target language
        #[arg(short = 'L', long, value_enum)]
        lang: Language,

        /// Output directory
        #[arg(short, long)]
        output: PathBuf,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Language {
    C,
    Java,
    Python,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GenLexer { lexer, lang, output } => {
            println!("Generating lexer...");
            println!("  Rules: {}", lexer.display());
            println!("  Lang:  {:?}", lang);
            println!("  Out:   {}", output.display());

            // 1. Read rules file
            let content = fs::read_to_string(&lexer)
                .with_context(|| format!("Failed to read lexer file: {}", lexer.display()))?;

            // 2. Parse lexer specification
            let spec = LexerSpec::parse(&content)
                .context("Failed to parse lexer specification")?;
            
            println!("  Rules: {}", spec.rules.len());
            for (i, rule) in spec.rules.iter().enumerate() {
                println!("    [{}] {} -> {:?}", i, rule.pattern, rule.action);
            }

            // 3. Build combined NFA from all rules
            let nfa = Nfa::from_lexer_spec(&spec)
                .context("Failed to build NFA")?;
            
            println!("  NFA States: {}", nfa.states.len());

            // 4. NFA -> DFA
            let dfa = Dfa::from_nfa(&nfa)
                .context("Failed to build DFA")?;

            println!("  DFA States: {}", dfa.states.len());

            // 5. Generate Code
            let target_lang = match lang {
                Language::C => TargetLanguage::C,
                Language::Java => TargetLanguage::Java,
                Language::Python => TargetLanguage::Python,
            };

            let code = lexgen::codegen::generate_lexer_from_spec(&dfa, &spec, target_lang)
                .context("Failed to generate code")?;

            // 6. Write Output
            fs::create_dir_all(&output)?;
            
            let filename = match lang {
                Language::C => "lexer.c",
                Language::Java => "Lexer.java",
                Language::Python => "lexer.py",
            };
            
            let output_path = output.join(filename);
            fs::write(&output_path, code)?;
            
            println!("Success! Generated {}", output_path.display());
        }

        Commands::GenParser { parser, lang, output } => {
            println!("Generating parser...");
            println!("  Grammar: {}", parser.display());
            
            let content = fs::read_to_string(&parser)
                .with_context(|| format!("Failed to read grammar file: {}", parser.display()))?;

            let grammar = Grammar::parse(&content)
                .context("Failed to parse grammar")?;
            
            // Debug: show parsed grammar
            println!("  Tokens: {:?}", grammar.tokens);
            println!("  Start: {}", grammar.start_symbol);
            println!("  Rules: {}", grammar.rules.len());
            for (i, rule) in grammar.rules.iter().enumerate() {
                println!("    [{}] {} -> {:?}", i, rule.lhs, rule.rhs);
            }
                
            let table = ParsingTable::build(&grammar)
                .context("Failed to build tables")?;
            
            println!("  States: {}", table.action.len());

            let target_lang = match lang {
                Language::C => TargetLanguage::C,
                Language::Java => TargetLanguage::Java,
                Language::Python => TargetLanguage::Python,
            };

            let code = parsegen::codegen::generate_parser(&table, &grammar, target_lang)
                .context("Failed to generate code")?;

            fs::create_dir_all(&output)?;
             let filename = match lang {
                Language::C => "parser.c",
                Language::Java => "Parser.java",
                Language::Python => "parser.py",
            };
            
            let output_path = output.join(filename);
            fs::write(&output_path, code)?;
            println!("Success! Generated {}", output_path.display());
        }
    }

    Ok(())
}
