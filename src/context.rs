//! Context handling for opencode-helix
//!
//! Maps Helix editor context to opencode format.

use crate::cli::Cli;
use std::fs;

/// Editor context captured from Helix
#[derive(Debug, Clone, Default)]
pub struct Context {
    /// Current file path (relative)
    pub file: Option<String>,

    /// Cursor line (1-based)
    pub line: Option<u32>,

    /// Cursor column (1-based)
    pub column: Option<u32>,

    /// Selection text content
    pub selection: Option<String>,

    /// Selection start line (1-based)
    pub selection_start: Option<u32>,

    /// Selection end line (1-based)
    pub selection_end: Option<u32>,

    /// File language
    pub language: Option<String>,
}

impl Context {
    /// Create context from CLI arguments
    pub fn from_cli(cli: &Cli) -> Self {
        let selection = cli.selection_file.as_ref().and_then(|path| {
            let content = fs::read_to_string(path).ok();
            // Clean up the temp file after reading
            let _ = fs::remove_file(path);
            content
        });

        Self {
            file: cli.file.as_ref().map(|p| p.display().to_string()),
            line: cli.line,
            column: cli.column,
            selection,
            selection_start: cli.selection_start,
            selection_end: cli.selection_end,
            language: cli.language.clone(),
        }
    }

    /// Format a file reference for opencode
    /// e.g., `@src/main.rs`
    pub fn format_file(&self) -> Option<String> {
        self.file.as_ref().map(|f| format!("@{}", f))
    }

    /// Format a location for opencode
    /// e.g., `@src/main.rs L42:C10` or `@src/main.rs L10-L20`
    pub fn format_location(&self) -> Option<String> {
        let file = self.file.as_ref()?;

        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            // Selection range
            Some(format!("@{} L{}-L{}", file, start, end))
        } else if let Some(line) = self.line {
            // Cursor position
            if let Some(col) = self.column {
                Some(format!("@{} L{}:C{}", file, line, col))
            } else {
                Some(format!("@{} L{}", file, line))
            }
        } else {
            // Just file
            Some(format!("@{}", file))
        }
    }

    /// Format @this context - selection range if available, else cursor position
    pub fn format_this(&self) -> Option<String> {
        self.format_location()
    }

    /// Format @buffer context - just the file reference
    pub fn format_buffer(&self) -> Option<String> {
        self.format_file()
    }

    /// Format @selection context - includes the selection text
    /// Returns None if no selection is available
    pub fn format_selection(&self) -> Option<String> {
        // Only return something if we have actual selection content
        if let Some(ref sel) = self.selection {
            let location = self.format_location()?;
            Some(format!("{}\n```\n{}\n```", location, sel))
        } else {
            // No selection available - return None so @selection is not replaced
            None
        }
    }

    /// Get git diff output
    pub fn format_diff(&self) -> Option<String> {
        std::process::Command::new("git")
            .args(["--no-pager", "diff"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    let diff = String::from_utf8_lossy(&output.stdout).to_string();
                    if diff.is_empty() {
                        None
                    } else {
                        Some(diff)
                    }
                } else {
                    None
                }
            })
    }

    /// Expand context placeholders in a prompt
    pub fn expand(&self, prompt: &str) -> String {
        let mut result = prompt.to_string();

        // Replace @this
        if let Some(this) = self.format_this() {
            result = result.replace("@this", &this);
        }

        // Replace @buffer
        if let Some(buffer) = self.format_buffer() {
            result = result.replace("@buffer", &buffer);
        }

        // Replace @selection
        if let Some(selection) = self.format_selection() {
            result = result.replace("@selection", &selection);
        }

        // Replace @diff
        if result.contains("@diff") {
            if let Some(diff) = self.format_diff() {
                result = result.replace("@diff", &diff);
            }
        }

        result
    }

    /// Check if context has any file information
    pub fn has_file(&self) -> bool {
        self.file.is_some()
    }

    /// Check if context has selection
    pub fn has_selection(&self) -> bool {
        self.selection.is_some() || self.selection_start.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_location_cursor() {
        let ctx = Context {
            file: Some("src/main.rs".to_string()),
            line: Some(42),
            column: Some(10),
            ..Default::default()
        };
        assert_eq!(
            ctx.format_location(),
            Some("@src/main.rs L42:C10".to_string())
        );
    }

    #[test]
    fn test_format_location_selection() {
        let ctx = Context {
            file: Some("src/main.rs".to_string()),
            selection_start: Some(10),
            selection_end: Some(20),
            ..Default::default()
        };
        assert_eq!(
            ctx.format_location(),
            Some("@src/main.rs L10-L20".to_string())
        );
    }

    #[test]
    fn test_expand_this() {
        let ctx = Context {
            file: Some("src/main.rs".to_string()),
            line: Some(42),
            ..Default::default()
        };
        let result = ctx.expand("Explain @this");
        assert_eq!(result, "Explain @src/main.rs L42");
    }

    #[test]
    fn test_expand_no_context() {
        let ctx = Context::default();
        let result = ctx.expand("Hello @this world");
        // @this should remain as-is if no context
        assert_eq!(result, "Hello @this world");
    }
}
