//! Visual themes for the TUI

use ratatui::style::Color;
use ratatui::widgets::BorderType;

/// Available UI themes
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ThemeKind {
    /// Minimal, clean theme (original)
    Minimal,
    /// Hacker/cyberpunk green theme
    #[default]
    Hacker,
    /// Matrix-inspired with rain effect colors
    Matrix,
    /// Retro CRT amber theme
    Crt,
}

impl ThemeKind {
    /// Parse theme from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "minimal" | "min" | "clean" => Self::Minimal,
            "hacker" | "hack" | "cyber" => Self::Hacker,
            "matrix" | "neo" => Self::Matrix,
            "crt" | "retro" | "amber" => Self::Crt,
            _ => Self::default(),
        }
    }

    /// Get the theme configuration
    pub fn config(&self) -> Theme {
        match self {
            Self::Minimal => Theme::minimal(),
            Self::Hacker => Theme::hacker(),
            Self::Matrix => Theme::matrix(),
            Self::Crt => Theme::crt(),
        }
    }
}

/// Theme configuration with colors and styling
#[derive(Debug, Clone)]
pub struct Theme {
    /// Primary accent color (borders, highlights)
    pub primary: Color,
    /// Secondary accent color
    pub secondary: Color,
    /// Tertiary/accent color for special elements
    pub accent: Color,
    /// Warning/attention color
    pub warning: Color,
    /// Error/cancel color
    pub error: Color,
    /// Dimmed text color
    pub dim: Color,
    /// Normal text color
    pub text: Color,
    /// Input text color
    pub input: Color,
    /// Title text for dialogs
    pub title: String,
    /// Prompt character(s)
    pub prompt: String,
    /// Filter prompt character(s)
    pub filter_prompt: String,
    /// Selected item prefix
    pub selected_prefix: String,
    /// Unselected item prefix
    pub unselected_prefix: String,
    /// Border style: "rounded", "double", "thick", "plain"
    pub border_style: &'static str,
}

impl Default for Theme {
    fn default() -> Self {
        Self::hacker()
    }
}

impl Theme {
    /// Minimal clean theme (original style)
    pub fn minimal() -> Self {
        Self {
            primary: Color::Cyan,
            secondary: Color::Blue,
            accent: Color::Magenta,
            warning: Color::Yellow,
            error: Color::Red,
            dim: Color::DarkGray,
            text: Color::White,
            input: Color::White,
            title: " opencode ".to_string(),
            prompt: "> ".to_string(),
            filter_prompt: "/ ".to_string(),
            selected_prefix: "> ".to_string(),
            unselected_prefix: "  ".to_string(),
            border_style: "rounded",
        }
    }

    /// Hacker/cyberpunk green theme
    pub fn hacker() -> Self {
        Self {
            primary: Color::Rgb(0, 255, 0),     // Bright green
            secondary: Color::Rgb(0, 255, 255), // Cyan
            accent: Color::Rgb(255, 0, 255),    // Magenta
            warning: Color::Rgb(255, 170, 0),   // Amber
            error: Color::Rgb(255, 50, 50),     // Red
            dim: Color::Rgb(0, 140, 0),         // Dark green
            text: Color::Rgb(0, 230, 0),        // Light green
            input: Color::Rgb(0, 255, 0),       // Bright green
            title: " ░▒▓ OPENCODE ▓▒░ ".to_string(),
            prompt: "λ ".to_string(),
            filter_prompt: "⟫ ".to_string(),
            selected_prefix: "▸ ".to_string(),
            unselected_prefix: "  ".to_string(),
            border_style: "thick",
        }
    }

    /// Matrix-inspired theme
    pub fn matrix() -> Self {
        Self {
            primary: Color::Rgb(0, 200, 0),    // Matrix green
            secondary: Color::Rgb(0, 255, 0),  // Bright green
            accent: Color::Rgb(150, 255, 150), // Light green
            warning: Color::Rgb(200, 255, 0),  // Yellow-green
            error: Color::Rgb(255, 100, 100),  // Soft red
            dim: Color::Rgb(0, 80, 0),         // Very dark green
            text: Color::Rgb(0, 180, 0),       // Medium green
            input: Color::Rgb(0, 255, 0),      // Bright green
            title: " ⟨ MATRIX ⟩ ".to_string(),
            prompt: "$ ".to_string(),
            filter_prompt: ">> ".to_string(),
            selected_prefix: "█ ".to_string(),
            unselected_prefix: "░ ".to_string(),
            border_style: "thick",
        }
    }

    /// Retro CRT amber theme
    pub fn crt() -> Self {
        Self {
            primary: Color::Rgb(255, 170, 0),    // Amber
            secondary: Color::Rgb(255, 200, 50), // Light amber
            accent: Color::Rgb(255, 220, 100),   // Pale amber
            warning: Color::Rgb(255, 255, 0),    // Yellow
            error: Color::Rgb(255, 100, 0),      // Orange-red
            dim: Color::Rgb(140, 90, 0),         // Dark amber
            text: Color::Rgb(255, 170, 0),       // Amber
            input: Color::Rgb(255, 200, 50),     // Light amber
            title: " ◄ TERMINAL ► ".to_string(),
            prompt: "C:\\> ".to_string(),
            filter_prompt: "? ".to_string(),
            selected_prefix: "=> ".to_string(),
            unselected_prefix: "   ".to_string(),
            border_style: "double",
        }
    }

    /// Get border type from style string
    pub fn border_type(&self) -> BorderType {
        match self.border_style {
            "double" => BorderType::Double,
            "thick" => BorderType::Thick,
            "plain" => BorderType::Plain,
            _ => BorderType::Rounded,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_from_str() {
        assert_eq!(ThemeKind::from_str("hacker"), ThemeKind::Hacker);
        assert_eq!(ThemeKind::from_str("HACKER"), ThemeKind::Hacker);
        assert_eq!(ThemeKind::from_str("minimal"), ThemeKind::Minimal);
        assert_eq!(ThemeKind::from_str("matrix"), ThemeKind::Matrix);
        assert_eq!(ThemeKind::from_str("crt"), ThemeKind::Crt);
        assert_eq!(ThemeKind::from_str("unknown"), ThemeKind::Hacker); // default
    }

    #[test]
    fn test_theme_configs() {
        let hacker = ThemeKind::Hacker.config();
        assert_eq!(hacker.border_style, "thick");
        assert_eq!(hacker.prompt, "λ ");

        let minimal = ThemeKind::Minimal.config();
        assert_eq!(minimal.border_style, "rounded");
        assert_eq!(minimal.prompt, "> ");
    }
}
