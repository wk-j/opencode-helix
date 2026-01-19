//! CLI argument parsing for opencode-helix

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// External TUI for integrating opencode AI assistant with Helix editor
#[derive(Parser, Debug)]
#[command(name = "opencode-helix")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Connect to a specific port (skips server discovery)
    #[arg(short, long, global = true)]
    pub port: Option<u16>,

    /// Current file path (for @this and @buffer context)
    #[arg(short, long, global = true)]
    pub file: Option<PathBuf>,

    /// Cursor line number (1-based)
    #[arg(short, long, global = true)]
    pub line: Option<u32>,

    /// Cursor column number (1-based, grapheme clusters)
    #[arg(short, long, global = true)]
    pub column: Option<u32>,

    /// Path to file containing selection text (file is deleted after reading)
    #[arg(long, global = true)]
    pub selection_file: Option<PathBuf>,

    /// Selection start line (1-based)
    #[arg(long, global = true)]
    pub selection_start: Option<u32>,

    /// Selection end line (1-based)
    #[arg(long, global = true)]
    pub selection_end: Option<u32>,

    /// Working directory (for server discovery, defaults to current dir)
    #[arg(long, global = true)]
    pub cwd: Option<PathBuf>,

    /// File language (e.g., "rust", "python")
    #[arg(long, global = true)]
    pub language: Option<String>,

    /// Enable debug mode (writes debug info to /tmp/opencode-helix-debug.log)
    #[arg(long, global = true)]
    pub debug: bool,

    /// UI theme: minimal, hacker (default), matrix, crt
    #[arg(long, global = true, default_value = "hacker")]
    pub theme: String,

    /// Disable animations (blinking cursor, typing effect, scanline)
    #[arg(long, global = true)]
    pub no_anim: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Open input prompt to type a message
    Ask {
        /// Initial text to populate the input with
        #[arg(default_value = "")]
        initial: String,
    },

    /// Open menu to select from prompts/commands
    Select,

    /// Send a prompt directly (non-interactive)
    Prompt {
        /// Prompt name (e.g., "explain", "review") or raw text
        text: String,

        /// Submit the prompt immediately (don't just append)
        #[arg(short, long, default_value = "true")]
        submit: bool,
    },

    /// Show current opencode status
    Status,
}

impl Cli {
    /// Parse CLI arguments
    pub fn parse_args() -> Self {
        Cli::parse()
    }

    /// Get the working directory (from --cwd or current directory)
    pub fn working_directory(&self) -> PathBuf {
        self.cwd
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    /// Check if we have selection context
    pub fn has_selection(&self) -> bool {
        self.selection_file.is_some() || self.selection_start.is_some()
    }

    /// Check if we have cursor context
    pub fn has_cursor(&self) -> bool {
        self.line.is_some()
    }

    /// Check if we have any file context
    pub fn has_file(&self) -> bool {
        self.file.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ask() {
        let cli = Cli::parse_from(["opencode-helix", "ask"]);
        assert!(matches!(cli.command, Command::Ask { initial } if initial.is_empty()));
    }

    #[test]
    fn test_parse_ask_with_context() {
        let cli = Cli::parse_from([
            "opencode-helix",
            "-f",
            "src/main.rs",
            "-l",
            "42",
            "-c",
            "10",
            "ask",
        ]);
        assert_eq!(cli.file, Some(PathBuf::from("src/main.rs")));
        assert_eq!(cli.line, Some(42));
        assert_eq!(cli.column, Some(10));
    }

    #[test]
    fn test_parse_prompt() {
        let cli = Cli::parse_from(["opencode-helix", "prompt", "explain"]);
        assert!(
            matches!(cli.command, Command::Prompt { text, submit } if text == "explain" && submit)
        );
    }

    #[test]
    fn test_parse_select() {
        let cli = Cli::parse_from(["opencode-helix", "select"]);
        assert!(matches!(cli.command, Command::Select));
    }
}
