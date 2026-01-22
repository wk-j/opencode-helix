//! Main TUI application

use anyhow::Result;
use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::fs::File;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::time::{Duration, Instant};

use crate::context::Context;

use crate::tui::theme::{Theme, ThemeKind};

const DEBUG_LOG_PATH: &str = "/tmp/opencode-helix-debug.log";

/// Find the @word being typed at cursor position
/// Returns (start_position, partial_word) if cursor is within or right after an @word
fn find_at_word(input: &str, cursor_pos: usize) -> Option<(usize, &str)> {
    // Look backwards from cursor to find @
    let before_cursor = &input[..cursor_pos];
    let at_pos = before_cursor.rfind('@')?;

    // Check there's no space between @ and cursor
    let partial = &input[at_pos..cursor_pos];
    if partial.contains(' ') {
        return None;
    }

    Some((at_pos, partial))
}

/// Filter placeholders that match the partial input
fn filter_placeholders<'a>(partial: &str, placeholders: &[&'a str]) -> Vec<&'a str> {
    let partial_lower = partial.to_lowercase();
    placeholders
        .iter()
        .filter(|p| p.to_lowercase().starts_with(&partial_lower))
        .copied()
        .collect()
}

/// Multi-line input helper: convert flat cursor position to (line, column)
fn cursor_to_line_col(text: &str, pos: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    for (i, c) in text.char_indices() {
        if i >= pos {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Multi-line input helper: convert (line, column) to flat cursor position
fn line_col_to_cursor(text: &str, target_line: usize, target_col: usize) -> usize {
    let mut current_line = 0;
    let mut line_start = 0;

    for (i, c) in text.char_indices() {
        if current_line == target_line {
            // We're on the target line, find the column
            let line_end = text[i..].find('\n').map(|p| i + p).unwrap_or(text.len());
            let line_len = line_end - i;
            return i + target_col.min(line_len);
        }
        if c == '\n' {
            current_line += 1;
            line_start = i + 1;
        }
    }

    // If target_line is beyond the last line, return end of text
    if current_line == target_line {
        let line_len = text.len() - line_start;
        return line_start + target_col.min(line_len);
    }
    text.len()
}

/// Get the length of a specific line (without newline)
fn get_line_length(text: &str, line_idx: usize) -> usize {
    text.lines().nth(line_idx).map(|l| l.len()).unwrap_or(0)
}

/// Count the number of lines in text
fn count_lines(text: &str) -> usize {
    if text.is_empty() {
        1
    } else {
        text.lines().count() + if text.ends_with('\n') { 1 } else { 0 }
    }
}

/// Represents a visual line after soft wrapping
#[derive(Debug, Clone)]
struct WrappedLine {
    /// The text content of this visual line
    text: String,
    /// The logical line index this belongs to
    logical_line: usize,
    /// Whether this is the first visual line of the logical line
    is_first: bool,
    /// Start position in the original text (byte offset)
    start_pos: usize,
}

/// Wrap text to fit within a given width, respecting logical line breaks
fn wrap_text(text: &str, width: usize, prefix_width: usize) -> Vec<WrappedLine> {
    let mut wrapped = Vec::new();
    let mut byte_offset = 0;

    for (logical_line, line) in text.split('\n').enumerate() {
        if line.is_empty() {
            // Empty line
            wrapped.push(WrappedLine {
                text: String::new(),
                logical_line,
                is_first: true,
                start_pos: byte_offset,
            });
        } else {
            // Calculate effective width (first line has prompt, others have indent)
            let effective_width = width.saturating_sub(prefix_width);
            if effective_width == 0 {
                // Fallback if width is too small
                wrapped.push(WrappedLine {
                    text: line.to_string(),
                    logical_line,
                    is_first: true,
                    start_pos: byte_offset,
                });
            } else {
                let mut remaining = line;
                let mut is_first = true;
                let mut line_byte_offset = byte_offset;

                while !remaining.is_empty() {
                    // Find break point
                    let break_at = if remaining.chars().count() <= effective_width {
                        remaining.len()
                    } else {
                        // Find the byte position for the character at effective_width
                        remaining
                            .char_indices()
                            .nth(effective_width)
                            .map(|(i, _)| i)
                            .unwrap_or(remaining.len())
                    };

                    let (chunk, rest) = remaining.split_at(break_at);
                    wrapped.push(WrappedLine {
                        text: chunk.to_string(),
                        logical_line,
                        is_first,
                        start_pos: line_byte_offset,
                    });
                    line_byte_offset += chunk.len();
                    remaining = rest;
                    is_first = false;
                }
            }
        }
        byte_offset += line.len() + 1; // +1 for newline
    }

    if wrapped.is_empty() {
        wrapped.push(WrappedLine {
            text: String::new(),
            logical_line: 0,
            is_first: true,
            start_pos: 0,
        });
    }

    wrapped
}

/// Find the visual row and column for a cursor position in wrapped text
fn cursor_to_visual_pos(
    text: &str,
    cursor_pos: usize,
    width: usize,
    prefix_width: usize,
) -> (usize, usize) {
    let wrapped = wrap_text(text, width, prefix_width);
    let mut visual_row = 0;

    for (i, wline) in wrapped.iter().enumerate() {
        let line_end = wline.start_pos + wline.text.len();
        // Check if cursor is in this wrapped line
        if cursor_pos >= wline.start_pos && cursor_pos <= line_end {
            let col_in_line = cursor_pos - wline.start_pos;
            return (i, col_in_line);
        }
        visual_row = i;
    }

    // Cursor is at the end
    (
        visual_row,
        wrapped.last().map(|l| l.text.len()).unwrap_or(0),
    )
}

/// Count total visual lines after wrapping
fn count_visual_lines(text: &str, width: usize, prefix_width: usize) -> usize {
    wrap_text(text, width, prefix_width).len()
}

/// Update scroll offset to keep cursor visible (using visual lines with wrapping)
fn update_scroll_for_cursor(
    text: &str,
    cursor_pos: usize,
    scroll_offset: &mut usize,
    visible_lines: usize,
    text_width: usize,
    prefix_width: usize,
) {
    let (visual_row, _) = cursor_to_visual_pos(text, cursor_pos, text_width, prefix_width);
    if visual_row < *scroll_offset {
        *scroll_offset = visual_row;
    } else if visual_row >= *scroll_offset + visible_lines {
        *scroll_offset = visual_row - visible_lines + 1;
    }
}

/// Write debug info to log file if debug mode is enabled
fn debug_log(debug: bool, msg: &str) {
    if debug {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(DEBUG_LOG_PATH)
        {
            let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
            let _ = writeln!(file, "[{}] {}", timestamp, msg);
        }
    }
}

/// Result of running the TUI app
#[derive(Debug)]
pub enum AppResult {
    /// User submitted input
    Submit(String),
    /// User cancelled
    Cancel,
}

/// TUI Application state
pub struct App {
    /// Terminal backend - uses /dev/tty to work when stdout is piped
    terminal: Terminal<CrosstermBackend<File>>,
    /// TTY file for reading input
    tty_reader: File,
    /// Debug mode
    debug: bool,
    /// Visual theme
    theme: Theme,
}

impl App {
    /// Create a new TUI application
    /// Uses /dev/tty directly to support running via Helix's :insert-output
    pub fn new(debug: bool) -> Result<Self> {
        Self::with_theme(debug, ThemeKind::default())
    }

    /// Create a new TUI application with a specific theme
    pub fn with_theme(debug: bool, theme_kind: ThemeKind) -> Result<Self> {
        // Open /dev/tty directly - this works even when stdout is piped
        let tty_write = File::options().read(true).write(true).open("/dev/tty")?;
        let tty_reader = File::options().read(true).open("/dev/tty")?;

        // Setup terminal
        enable_raw_mode()?;

        // Use a separate scope to handle the execute macro
        let backend = {
            let mut tty = tty_write;
            // Write escape sequences directly
            use std::io::Write;
            write!(tty, "\x1b[?1049h")?; // Enter alternate screen
            write!(tty, "\x1b[?1000h")?; // Enable mouse capture
            tty.flush()?;
            CrosstermBackend::new(tty)
        };
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            terminal,
            tty_reader,
            debug,
            theme: theme_kind.config(),
        })
    }

    /// Restore terminal to normal state
    pub fn restore(&mut self) -> Result<()> {
        disable_raw_mode()?;
        // Write escape sequences directly
        use std::io::Write;
        let tty = self.terminal.backend_mut();
        write!(tty, "\x1b[?1000l")?; // Disable mouse capture
        write!(tty, "\x1b[?1049l")?; // Leave alternate screen
        std::io::Write::flush(tty)?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    /// Read a key event from /dev/tty with timeout
    fn read_key(&mut self, timeout: Duration) -> Result<Option<KeyEvent>> {
        let fd = self.tty_reader.as_raw_fd();

        // Use poll to check if data is available
        let mut pollfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };

        let timeout_ms = timeout.as_millis() as i32;
        let ret = unsafe { libc::poll(&mut pollfd, 1, timeout_ms) };

        if ret <= 0 {
            return Ok(None);
        }

        // Read first byte
        let mut buf = [0u8; 1];
        let n = self.tty_reader.read(&mut buf)?;
        if n == 0 {
            return Ok(None);
        }

        let first_byte = buf[0];

        // Debug log raw bytes
        debug_log(self.debug, &format!("Key byte: 0x{:02x}", first_byte));

        // If it's an escape byte, check if more bytes follow (escape sequence)
        if first_byte == 0x1b {
            // Poll briefly to see if more bytes are coming (escape sequence)
            let mut pollfd2 = libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let ret2 = unsafe { libc::poll(&mut pollfd2, 1, 50) }; // 50ms timeout

            if ret2 > 0 {
                // More bytes available - read the escape sequence
                let mut seq_buf = [0u8; 16];
                let seq_n = self.tty_reader.read(&mut seq_buf)?;
                if seq_n > 0 {
                    // Combine escape + sequence bytes
                    let mut full_seq = vec![0x1b];
                    full_seq.extend_from_slice(&seq_buf[..seq_n]);

                    // Debug log escape sequence
                    debug_log(self.debug, &format!("Escape seq: {:02x?}", full_seq));

                    return Ok(self.parse_key(&full_seq));
                }
            }
            // No more bytes - it's a bare Escape key
            return Ok(Some(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
        }

        // Parse single byte
        Ok(self.parse_key(&[first_byte]))
    }

    /// Parse raw bytes into a KeyEvent
    fn parse_key(&self, bytes: &[u8]) -> Option<KeyEvent> {
        if bytes.is_empty() {
            return None;
        }

        let key = match bytes {
            // Escape
            [0x1b] => KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            // Enter
            [0x0d] | [0x0a] => KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            // Backspace
            [0x7f] | [0x08] => KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
            // Tab
            [0x09] => KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
            // Ctrl+C
            [0x03] => KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            // Ctrl+D
            [0x04] => KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
            // Explicit Ctrl+N and Ctrl+P
            [0x0e] => KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL),
            [0x10] => KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
            // Generic Control characters (Ctrl+A to Ctrl+Z)
            // 0x01 (A) to 0x1A (Z), excluding those handled above
            [c] if *c >= 0x01 && *c <= 0x1A => {
                let char_code = c + 0x60; // 1 -> 'a'
                KeyEvent::new(KeyCode::Char(char_code as char), KeyModifiers::CONTROL)
            }
            // Arrow keys and other escape sequences
            [0x1b, 0x5b, rest @ ..] => match rest {
                [0x41] => KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
                [0x42] => KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
                [0x43] => KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
                [0x44] => KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
                [0x48] => KeyEvent::new(KeyCode::Home, KeyModifiers::NONE),
                [0x46] => KeyEvent::new(KeyCode::End, KeyModifiers::NONE),
                [0x33, 0x7e] => KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE),
                [0x5a] => KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT), // Shift+Tab
                // Any other escape sequence - treat as Escape key
                _ => KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            },
            // Alt + Char
            [0x1b, c] if *c >= 0x20 && *c < 0x7f => {
                KeyEvent::new(KeyCode::Char(*c as char), KeyModifiers::ALT)
            }
            // Any other escape sequence - treat as Escape key
            [0x1b, ..] => KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            // Regular ASCII character
            [c] if *c >= 0x20 && *c < 0x7f => {
                KeyEvent::new(KeyCode::Char(*c as char), KeyModifiers::NONE)
            }
            // UTF-8 character (2-4 bytes)
            _ => {
                if let Ok(s) = std::str::from_utf8(bytes) {
                    if let Some(c) = s.chars().next() {
                        return Some(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
                    }
                }
                return None;
            }
        };

        Some(key)
    }

    /// Run the ask (input) mode
    pub fn run_ask(
        &mut self,
        initial: &str,
        context_hint: Option<&str>,
        context: Option<&Context>,
        animations: bool,
    ) -> Result<AppResult> {
        let mut input = initial.to_string();
        let mut cursor_pos = input.len();
        // Focus: 0 = input, 1 = Send button, 2 = Cancel button
        let mut focus: u8 = 0;

        // Multi-line input state
        let input_visible_lines: u16 = 5; // Number of visible lines in input area
        let mut scroll_offset: usize = 0; // First visible line
        let mut last_text_width: usize = 60; // Track text width for scroll calculations

        // Autocomplete state
        let mut autocomplete_active = false;
        let mut autocomplete_selected: usize = 0;

        // Get placeholders if context is available
        let placeholders = context
            .map(|ctx| ctx.list_placeholders())
            .unwrap_or_default();

        // Available placeholder names for autocomplete
        let placeholder_names: Vec<&str> = placeholders.iter().map(|(name, _)| *name).collect();

        // Clone theme for use in closure
        let theme = self.theme.clone();

        let mut cursor_visible = true;
        let mut cursor_timer = Instant::now();

        // Help text (static)
        let help_text = "[Tab] Focus  [Enter] Send  [Esc] Abort";

        loop {
            // Update cursor blink
            if cursor_timer.elapsed() >= Duration::from_millis(530) {
                cursor_visible = !cursor_visible;
                cursor_timer = Instant::now();
            }

            // Draw UI
            self.terminal.draw(|frame| {
                let area = frame.area();

                // Dialog size - always include space for placeholders if we have them
                let has_placeholders = !placeholders.is_empty();
                let dialog_width = if has_placeholders {
                    area.width.min(80)
                } else {
                    area.width.min(70)
                };
                // Base height: hint(1) + input area(5) + gap(1) + buttons(1) + help(1) + borders(2) = 11
                // With placeholders: add title(1) + placeholder lines + gap(1)
                let dialog_height = if has_placeholders {
                    13 + input_visible_lines + placeholders.len() as u16
                } else {
                    9 + input_visible_lines
                };
                let dialog_area = Rect {
                    x: (area.width - dialog_width) / 2,
                    y: (area.height - dialog_height) / 2,
                    width: dialog_width,
                    height: dialog_height,
                };

                // Clear background
                frame.render_widget(Clear, dialog_area);

                // Dialog box with themed styling
                let block = Block::default()
                    .title(theme.title.as_str())
                    .title_style(
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    )
                    .borders(Borders::ALL)
                    .border_type(theme.border_type())
                    .border_style(Style::default().fg(theme.primary));

                let inner = block.inner(dialog_area);
                frame.render_widget(block, dialog_area);

                // Context hint (if any)
                let mut current_y = inner.y;
                if let Some(hint) = context_hint {
                    let hint_para = Paragraph::new(hint).style(Style::default().fg(theme.dim));
                    frame.render_widget(
                        hint_para,
                        Rect {
                            x: inner.x + 1, // Padding
                            y: current_y,
                            width: inner.width.saturating_sub(2),
                            height: 1,
                        },
                    );
                    current_y += 1;
                }

                // Input field (multi-line with soft wrap)
                let input_style = if focus == 0 {
                    Style::default().fg(theme.input)
                } else {
                    Style::default().fg(theme.dim)
                };

                // Calculate available width for text (minus padding and borders)
                let prompt_len = theme.prompt.chars().count();
                let text_width = inner.width.saturating_sub(2) as usize; // -2 for padding
                last_text_width = text_width; // Save for scroll calculations in key handlers

                // Get wrapped lines
                let wrapped_lines = wrap_text(&input, text_width, prompt_len);
                let total_visual_lines = wrapped_lines.len();

                // Find cursor visual position
                let (cursor_visual_row, cursor_visual_col) =
                    cursor_to_visual_pos(&input, cursor_pos, text_width, prompt_len);

                // Build display lines with scroll
                let input_lines: Vec<Line> = wrapped_lines
                    .iter()
                    .enumerate()
                    .skip(scroll_offset)
                    .take(input_visible_lines as usize)
                    .map(|(visual_idx, wline)| {
                        let is_cursor_line = visual_idx == cursor_visual_row;
                        let style = input_style;

                        // Determine prefix: prompt for first line of first logical line,
                        // continuation marker for wrapped lines, indent for other logical lines
                        let (prefix, prefix_style) = if wline.logical_line == 0 && wline.is_first {
                            // First line has prompt
                            (
                                theme.prompt.clone(),
                                Style::default()
                                    .fg(theme.primary)
                                    .add_modifier(Modifier::BOLD),
                            )
                        } else if !wline.is_first {
                            // Continuation of wrapped line - use a subtle marker
                            let cont = format!("{:>width$}", "↪ ", width = prompt_len);
                            (cont, Style::default().fg(theme.dim))
                        } else {
                            // Other logical lines - indent to align
                            let indent = " ".repeat(prompt_len);
                            (indent, Style::default().fg(theme.dim))
                        };

                        let prefix_span = Span::styled(prefix, prefix_style);
                        let text_span = Span::styled(&wline.text, style);

                        if is_cursor_line && focus == 0 {
                            let cursor_char = if cursor_visible { "█" } else { " " };
                            let cursor_span =
                                Span::styled(cursor_char, Style::default().fg(theme.primary));
                            Line::from(vec![prefix_span, text_span, cursor_span])
                        } else {
                            Line::from(vec![prefix_span, text_span])
                        }
                    })
                    .collect();

                // Show scroll indicator if needed
                let scroll_indicator = if total_visual_lines > input_visible_lines as usize {
                    format!(" [{}/{}]", cursor_visual_row + 1, total_visual_lines)
                } else {
                    String::new()
                };

                let input_para = Paragraph::new(input_lines);
                let input_y = current_y;
                let input_area_height = input_visible_lines;
                frame.render_widget(
                    input_para,
                    Rect {
                        x: inner.x + 1, // Padding
                        y: current_y,
                        width: inner.width.saturating_sub(2),
                        height: input_area_height,
                    },
                );

                // Scroll indicator on the right side
                if !scroll_indicator.is_empty() {
                    let indicator = Paragraph::new(scroll_indicator)
                        .style(Style::default().fg(theme.dim))
                        .alignment(Alignment::Right);
                    frame.render_widget(
                        indicator,
                        Rect {
                            x: inner.x + 1,
                            y: current_y + input_area_height - 1,
                            width: inner.width.saturating_sub(2),
                            height: 1,
                        },
                    );
                }

                current_y += input_area_height + 1;

                // Placeholders panel (always show when we have placeholders)
                if !placeholders.is_empty() {
                    // Simple section header
                    let title_para = Paragraph::new("Placeholders:").style(
                        Style::default()
                            .fg(theme.dim)
                            .add_modifier(Modifier::ITALIC),
                    );
                    frame.render_widget(
                        title_para,
                        Rect {
                            x: inner.x + 1,
                            y: current_y,
                            width: inner.width.saturating_sub(2),
                            height: 1,
                        },
                    );
                    current_y += 1;

                    for (placeholder, value) in &placeholders {
                        // Truncate value if too long
                        let max_value_len = (inner.width as usize).saturating_sub(20);
                        let display_value = if value.len() > max_value_len {
                            format!("{}...", &value[..max_value_len.saturating_sub(3)])
                        } else {
                            value.clone()
                        };

                        let line = Line::from(vec![
                            Span::styled(
                                format!("  {:<12}", placeholder),
                                Style::default().fg(theme.secondary),
                            ),
                            Span::styled(display_value, Style::default().fg(theme.dim)),
                        ]);

                        let para = Paragraph::new(line);
                        frame.render_widget(
                            para,
                            Rect {
                                x: inner.x + 1,
                                y: current_y,
                                width: inner.width.saturating_sub(2),
                                height: 1,
                            },
                        );
                        current_y += 1;
                    }
                    current_y += 1;
                }

                // Buttons row
                let button_y = current_y;

                // Send button (themed)
                let send_style = if focus == 1 {
                    Style::default()
                        .fg(Color::Black)
                        .bg(theme.primary)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.dim)
                };
                let send_btn = Paragraph::new(" SEND ")
                    .style(send_style)
                    .alignment(Alignment::Center);
                frame.render_widget(
                    send_btn,
                    Rect {
                        x: inner.x + 1,
                        y: button_y,
                        width: 8,
                        height: 1,
                    },
                );

                // Cancel button (themed)
                let cancel_style = if focus == 2 {
                    Style::default()
                        .fg(Color::Black)
                        .bg(theme.error)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.dim)
                };
                let cancel_btn = Paragraph::new(" CANCEL ")
                    .style(cancel_style)
                    .alignment(Alignment::Center);
                frame.render_widget(
                    cancel_btn,
                    Rect {
                        x: inner.x + 11,
                        y: button_y,
                        width: 10,
                        height: 1,
                    },
                );

                // Help text (themed)
                let help_display = format!(" {} ", help_text);
                let help_para = Paragraph::new(help_display)
                    .style(Style::default().fg(theme.dim))
                    .alignment(Alignment::Center);
                frame.render_widget(
                    help_para,
                    Rect {
                        x: inner.x,
                        y: inner.y + inner.height - 1,
                        width: inner.width,
                        height: 1,
                    },
                );

                // Autocomplete popup (rendered last to appear on top)
                let filtered_completions: Vec<&str> =
                    if let Some((_, partial)) = find_at_word(&input, cursor_pos) {
                        if autocomplete_active {
                            filter_placeholders(partial, &placeholder_names)
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    };

                if !filtered_completions.is_empty() {
                    let popup_width = 16u16;
                    let popup_height = (filtered_completions.len() as u16 + 2).min(8); // +2 for border
                    let prompt_len = theme.prompt.chars().count() as u16;

                    // Position popup below the @ symbol
                    let at_pos = find_at_word(&input, cursor_pos)
                        .map(|(p, _)| p)
                        .unwrap_or(0);
                    let popup_x = (inner.x + 1 + prompt_len + at_pos as u16)
                        .min(area.width.saturating_sub(popup_width + 1));
                    let popup_y = input_y + 1;

                    let popup_area = Rect {
                        x: popup_x,
                        y: popup_y,
                        width: popup_width,
                        height: popup_height,
                    };

                    // Clear and draw popup background
                    frame.render_widget(Clear, popup_area);
                    let popup_block = Block::default()
                        .borders(Borders::ALL)
                        .border_type(ratatui::widgets::BorderType::Rounded)
                        .border_style(Style::default().fg(theme.secondary));
                    let popup_inner = popup_block.inner(popup_area);
                    frame.render_widget(popup_block, popup_area);

                    // Draw completion items
                    for (i, completion) in filtered_completions.iter().enumerate() {
                        if i >= popup_inner.height as usize {
                            break;
                        }
                        let style = if i == autocomplete_selected {
                            Style::default()
                                .fg(Color::Black)
                                .bg(theme.primary)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(theme.text)
                        };
                        let item = Paragraph::new(*completion).style(style);
                        frame.render_widget(
                            item,
                            Rect {
                                x: popup_inner.x,
                                y: popup_inner.y + i as u16,
                                width: popup_inner.width,
                                height: 1,
                            },
                        );
                    }
                }

                // Position cursor only when input is focused (hidden, we use block cursor)
                if focus == 0 {
                    let prompt_len = theme.prompt.chars().count() as u16;
                    let visible_cursor_row = cursor_visual_row.saturating_sub(scroll_offset);
                    let cursor_y_pos = input_y + visible_cursor_row as u16;
                    // Column offset includes prefix width
                    let col_offset = prompt_len + cursor_visual_col as u16;
                    frame.set_cursor_position(Position {
                        x: inner.x + 1 + col_offset,
                        y: cursor_y_pos,
                    });
                }
            })?;

            // Check if autocomplete should be shown
            let current_completions: Vec<&str> =
                if let Some((_, partial)) = find_at_word(&input, cursor_pos) {
                    filter_placeholders(partial, &placeholder_names)
                } else {
                    vec![]
                };

            // Update autocomplete state
            if !current_completions.is_empty() && focus == 0 {
                autocomplete_active = true;
                // Clamp selection to valid range
                if autocomplete_selected >= current_completions.len() {
                    autocomplete_selected = 0;
                }
            } else {
                autocomplete_active = false;
                autocomplete_selected = 0;
            }

            // Handle input from /dev/tty
            if let Some(key) = self.read_key(Duration::from_millis(16))? {
                // Handle autocomplete navigation first
                if autocomplete_active && !current_completions.is_empty() {
                    match key.code {
                        KeyCode::Down | KeyCode::Char('n')
                            if key.code == KeyCode::Down
                                || key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            autocomplete_selected =
                                (autocomplete_selected + 1) % current_completions.len();
                            continue;
                        }
                        KeyCode::Up | KeyCode::Char('p')
                            if key.code == KeyCode::Up
                                || key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            autocomplete_selected = if autocomplete_selected == 0 {
                                current_completions.len() - 1
                            } else {
                                autocomplete_selected - 1
                            };
                            continue;
                        }
                        KeyCode::Tab | KeyCode::Enter => {
                            // Accept completion
                            if let Some((at_pos, _)) = find_at_word(&input, cursor_pos) {
                                let completion = current_completions[autocomplete_selected];
                                // Replace the partial @word with the full completion
                                input.replace_range(at_pos..cursor_pos, completion);
                                cursor_pos = at_pos + completion.len();
                                // Add a space after completion
                                input.insert(cursor_pos, ' ');
                                cursor_pos += 1;
                                autocomplete_active = false;
                                autocomplete_selected = 0;
                            }
                            continue;
                        }
                        KeyCode::Esc => {
                            // Cancel autocomplete but don't exit dialog
                            autocomplete_active = false;
                            autocomplete_selected = 0;
                            continue;
                        }
                        _ => {}
                    }
                }

                match key.code {
                    KeyCode::Tab if !autocomplete_active => {
                        // Cycle focus: input -> Send -> Cancel -> input
                        focus = (focus + 1) % 3;
                    }
                    KeyCode::BackTab => {
                        // Reverse cycle
                        focus = if focus == 0 { 2 } else { focus - 1 };
                    }
                    // Enter to submit (text auto-wraps visually, no manual newlines needed)
                    KeyCode::Enter => {
                        match focus {
                            0 => {
                                // Submit from input field
                                if !input.is_empty() {
                                    return Ok(AppResult::Submit(input));
                                }
                            }
                            1 => {
                                // Send button
                                if !input.is_empty() {
                                    return Ok(AppResult::Submit(input));
                                }
                            }
                            2 => {
                                // Cancel button
                                return Ok(AppResult::Cancel);
                            }
                            _ => {}
                        }
                    }
                    KeyCode::Esc => {
                        return Ok(AppResult::Cancel);
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(AppResult::Cancel);
                    }
                    // Up arrow for multi-line navigation
                    KeyCode::Up if focus == 0 && !autocomplete_active => {
                        let (cursor_line, cursor_col) = cursor_to_line_col(&input, cursor_pos);
                        if cursor_line > 0 {
                            // Move to previous line, same column (or end of line if shorter)
                            cursor_pos = line_col_to_cursor(&input, cursor_line - 1, cursor_col);
                            // Update scroll using visual lines
                            let prefix_len = theme.prompt.chars().count();
                            update_scroll_for_cursor(
                                &input,
                                cursor_pos,
                                &mut scroll_offset,
                                input_visible_lines as usize,
                                last_text_width,
                                prefix_len,
                            );
                        }
                    }
                    // Down arrow for multi-line navigation
                    KeyCode::Down if focus == 0 && !autocomplete_active => {
                        let (cursor_line, cursor_col) = cursor_to_line_col(&input, cursor_pos);
                        let total_lines = count_lines(&input);
                        if cursor_line < total_lines - 1 {
                            // Move to next line, same column (or end of line if shorter)
                            cursor_pos = line_col_to_cursor(&input, cursor_line + 1, cursor_col);
                            // Update scroll using visual lines
                            let prefix_len = theme.prompt.chars().count();
                            update_scroll_for_cursor(
                                &input,
                                cursor_pos,
                                &mut scroll_offset,
                                input_visible_lines as usize,
                                last_text_width,
                                prefix_len,
                            );
                        }
                    }
                    // Only handle text input when input field is focused
                    KeyCode::Char(c)
                        if focus == 0
                            && !key
                                .modifiers
                                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        input.insert(cursor_pos, c);
                        cursor_pos += 1;
                        // Update scroll (line might wrap)
                        let prefix_len = theme.prompt.chars().count();
                        update_scroll_for_cursor(
                            &input,
                            cursor_pos,
                            &mut scroll_offset,
                            input_visible_lines as usize,
                            last_text_width,
                            prefix_len,
                        );
                    }
                    KeyCode::Backspace if focus == 0 => {
                        if cursor_pos > 0 {
                            input.remove(cursor_pos - 1);
                            cursor_pos -= 1;
                            // Update scroll using visual lines
                            let prefix_len = theme.prompt.chars().count();
                            update_scroll_for_cursor(
                                &input,
                                cursor_pos,
                                &mut scroll_offset,
                                input_visible_lines as usize,
                                last_text_width,
                                prefix_len,
                            );
                        }
                    }
                    KeyCode::Delete if focus == 0 => {
                        if cursor_pos < input.len() {
                            input.remove(cursor_pos);
                        }
                    }
                    KeyCode::Left if focus == 0 => {
                        if cursor_pos > 0 {
                            cursor_pos -= 1;
                            // Update scroll using visual lines
                            let prefix_len = theme.prompt.chars().count();
                            update_scroll_for_cursor(
                                &input,
                                cursor_pos,
                                &mut scroll_offset,
                                input_visible_lines as usize,
                                last_text_width,
                                prefix_len,
                            );
                        }
                    }
                    KeyCode::Right if focus == 0 => {
                        if cursor_pos < input.len() {
                            cursor_pos += 1;
                            // Update scroll using visual lines
                            let prefix_len = theme.prompt.chars().count();
                            update_scroll_for_cursor(
                                &input,
                                cursor_pos,
                                &mut scroll_offset,
                                input_visible_lines as usize,
                                last_text_width,
                                prefix_len,
                            );
                        }
                    }
                    KeyCode::Home if focus == 0 => {
                        // Move to start of current line
                        let (cursor_line, _) = cursor_to_line_col(&input, cursor_pos);
                        cursor_pos = line_col_to_cursor(&input, cursor_line, 0);
                    }
                    KeyCode::End if focus == 0 => {
                        // Move to end of current line
                        let (cursor_line, _) = cursor_to_line_col(&input, cursor_pos);
                        let line_len = get_line_length(&input, cursor_line);
                        cursor_pos = line_col_to_cursor(&input, cursor_line, line_len);
                    }
                    // Arrow keys for button navigation
                    KeyCode::Left if focus > 0 => {
                        focus -= 1;
                    }
                    KeyCode::Right if focus > 0 && focus < 2 => {
                        focus += 1;
                    }
                    _ => {}
                }
            }
        }
    }

    /// Run the select (menu) mode
    pub fn run_select(&mut self, items: &[SelectItem], animations: bool) -> Result<AppResult> {
        if items.is_empty() {
            return Ok(AppResult::Cancel);
        }

        let mut selected = 0;
        let mut filter = String::new();

        // Clone theme for use in closure
        let theme = self.theme.clone();

        let mut cursor_visible = true;
        let mut cursor_timer = Instant::now();

        // Help text (static)
        let help_text = "[Tab] Navigate  [Enter] Execute  [Esc] Abort";

        loop {
            // Update cursor blink
            if cursor_timer.elapsed() >= Duration::from_millis(530) {
                cursor_visible = !cursor_visible;
                cursor_timer = Instant::now();
            }

            // Filter items
            let filtered: Vec<(usize, &SelectItem)> = items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    if filter.is_empty() {
                        true
                    } else {
                        item.name.to_lowercase().contains(&filter.to_lowercase())
                            || item
                                .description
                                .to_lowercase()
                                .contains(&filter.to_lowercase())
                    }
                })
                .collect();

            // Clamp selection
            if selected >= filtered.len() {
                selected = filtered.len().saturating_sub(1);
            }

            // Draw UI
            self.terminal.draw(|frame| {
                let area = frame.area();

                // Dialog size
                let dialog_width = area.width.min(70);
                let dialog_height = (items.len() as u16 + 6).min(area.height - 4);
                let dialog_area = Rect {
                    x: (area.width - dialog_width) / 2,
                    y: (area.height - dialog_height) / 2,
                    width: dialog_width,
                    height: dialog_height,
                };

                // Clear background
                frame.render_widget(Clear, dialog_area);

                // Dialog box with themed styling
                let title = format!("{} SELECT ", theme.title);
                let block = Block::default()
                    .title(title)
                    .title_style(
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    )
                    .borders(Borders::ALL)
                    .border_type(theme.border_type())
                    .border_style(Style::default().fg(theme.primary));

                let inner = block.inner(dialog_area);
                frame.render_widget(block, dialog_area);

                // Filter input (themed)
                let filter_prompt = Span::styled(
                    theme.filter_prompt.as_str(),
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                );
                let filter_text = Span::styled(&filter, Style::default().fg(theme.input));
                let cursor_char = if cursor_visible { "█" } else { " " };
                let cursor_span = Span::styled(cursor_char, Style::default().fg(theme.primary));
                let filter_line = Line::from(vec![filter_prompt, filter_text, cursor_span]);

                let filter_para = Paragraph::new(filter_line);
                frame.render_widget(
                    filter_para,
                    Rect {
                        x: inner.x + 1,
                        y: inner.y,
                        width: inner.width.saturating_sub(2),
                        height: 1,
                    },
                );

                // Items
                let items_area = Rect {
                    x: inner.x + 1,
                    y: inner.y + 2,
                    width: inner.width.saturating_sub(2),
                    height: inner.height.saturating_sub(4),
                };

                for (i, (_, item)) in filtered.iter().enumerate() {
                    if i as u16 >= items_area.height {
                        break;
                    }

                    let (style, prefix) = if i == selected {
                        (
                            Style::default()
                                .fg(Color::Black)
                                .bg(theme.primary)
                                .add_modifier(Modifier::BOLD),
                            theme.selected_prefix.as_str(),
                        )
                    } else {
                        (
                            Style::default().fg(theme.text),
                            theme.unselected_prefix.as_str(),
                        )
                    };

                    let text = format!("{}{:<12} {}", prefix, item.name, item.description);
                    let para = Paragraph::new(text).style(style);

                    frame.render_widget(
                        para,
                        Rect {
                            x: items_area.x,
                            y: items_area.y + i as u16,
                            width: items_area.width,
                            height: 1,
                        },
                    );
                }

                // Help text (themed)
                let help_display = format!(" {} ", help_text);
                let help_para = Paragraph::new(help_display)
                    .style(Style::default().fg(theme.dim))
                    .alignment(Alignment::Center);
                frame.render_widget(
                    help_para,
                    Rect {
                        x: inner.x,
                        y: inner.y + inner.height - 1,
                        width: inner.width,
                        height: 1,
                    },
                );
            })?;

            // Handle input from /dev/tty
            if let Some(key) = self.read_key(Duration::from_millis(16))? {
                match key.code {
                    KeyCode::Enter => {
                        if let Some((_, item)) = filtered.get(selected) {
                            return Ok(AppResult::Submit(item.value.clone()));
                        }
                    }
                    KeyCode::Esc => {
                        return Ok(AppResult::Cancel);
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(AppResult::Cancel);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if selected > 0 {
                            selected -= 1;
                        }
                    }
                    KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if selected > 0 {
                            selected -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if selected < filtered.len().saturating_sub(1) {
                            selected += 1;
                        }
                    }
                    KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if selected < filtered.len().saturating_sub(1) {
                            selected += 1;
                        }
                    }
                    KeyCode::Char(c)
                        if !key
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        filter.push(c);
                    }
                    KeyCode::Backspace => {
                        filter.pop();
                    }
                    _ => {}
                }
            }
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

/// An item in the select menu
#[derive(Debug, Clone)]
pub struct SelectItem {
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Value to return when selected
    pub value: String,
    /// Category for grouping
    pub category: String,
}

impl SelectItem {
    pub fn new(name: &str, description: &str, value: &str, category: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            value: value.to_string(),
            category: category.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_to_line_col_single_line() {
        let text = "hello world";
        assert_eq!(cursor_to_line_col(text, 0), (0, 0));
        assert_eq!(cursor_to_line_col(text, 5), (0, 5));
        assert_eq!(cursor_to_line_col(text, 11), (0, 11));
    }

    #[test]
    fn test_cursor_to_line_col_multi_line() {
        let text = "hello\nworld\nfoo";
        // Line 0: "hello" (positions 0-5, newline at 5)
        assert_eq!(cursor_to_line_col(text, 0), (0, 0));
        assert_eq!(cursor_to_line_col(text, 5), (0, 5));
        // Line 1: "world" (positions 6-11, newline at 11)
        assert_eq!(cursor_to_line_col(text, 6), (1, 0));
        assert_eq!(cursor_to_line_col(text, 8), (1, 2));
        // Line 2: "foo" (positions 12-15)
        assert_eq!(cursor_to_line_col(text, 12), (2, 0));
        assert_eq!(cursor_to_line_col(text, 15), (2, 3));
    }

    #[test]
    fn test_line_col_to_cursor_single_line() {
        let text = "hello world";
        assert_eq!(line_col_to_cursor(text, 0, 0), 0);
        assert_eq!(line_col_to_cursor(text, 0, 5), 5);
        assert_eq!(line_col_to_cursor(text, 0, 11), 11);
        // Clamped to line length
        assert_eq!(line_col_to_cursor(text, 0, 100), 11);
    }

    #[test]
    fn test_line_col_to_cursor_multi_line() {
        let text = "hello\nworld\nfoo";
        assert_eq!(line_col_to_cursor(text, 0, 0), 0);
        assert_eq!(line_col_to_cursor(text, 0, 5), 5);
        assert_eq!(line_col_to_cursor(text, 1, 0), 6);
        assert_eq!(line_col_to_cursor(text, 1, 2), 8);
        assert_eq!(line_col_to_cursor(text, 2, 0), 12);
        assert_eq!(line_col_to_cursor(text, 2, 3), 15);
        // Clamped to line length
        assert_eq!(line_col_to_cursor(text, 1, 100), 11); // "world" is 5 chars, so max is position 11
    }

    #[test]
    fn test_get_line_length() {
        let text = "hello\nworld\nfoo";
        assert_eq!(get_line_length(text, 0), 5);
        assert_eq!(get_line_length(text, 1), 5);
        assert_eq!(get_line_length(text, 2), 3);
        assert_eq!(get_line_length(text, 3), 0); // non-existent line
    }

    #[test]
    fn test_count_lines() {
        assert_eq!(count_lines(""), 1);
        assert_eq!(count_lines("hello"), 1);
        assert_eq!(count_lines("hello\n"), 2);
        assert_eq!(count_lines("hello\nworld"), 2);
        assert_eq!(count_lines("hello\nworld\n"), 3);
        assert_eq!(count_lines("a\nb\nc"), 3);
    }

    #[test]
    fn test_find_at_word() {
        assert_eq!(find_at_word("@this", 5), Some((0, "@this")));
        assert_eq!(find_at_word("@this", 3), Some((0, "@th")));
        assert_eq!(find_at_word("hello @this", 11), Some((6, "@this")));
        assert_eq!(find_at_word("hello @", 7), Some((6, "@")));
        assert_eq!(find_at_word("hello", 5), None);
        assert_eq!(find_at_word("@ test", 6), None); // space between @ and cursor
    }

    #[test]
    fn test_filter_placeholders() {
        let placeholders = vec!["@this", "@buffer", "@path", "@selection"];
        assert_eq!(filter_placeholders("@", &placeholders), placeholders);
        assert_eq!(filter_placeholders("@t", &placeholders), vec!["@this"]);
        assert_eq!(filter_placeholders("@b", &placeholders), vec!["@buffer"]);
        assert_eq!(filter_placeholders("@p", &placeholders), vec!["@path"]);
        assert_eq!(filter_placeholders("@s", &placeholders), vec!["@selection"]);
        let empty: Vec<&str> = vec![];
        assert_eq!(filter_placeholders("@x", &placeholders), empty);
    }

    #[test]
    fn test_wrap_text_no_wrap_needed() {
        let text = "hello";
        let wrapped = wrap_text(text, 20, 2);
        assert_eq!(wrapped.len(), 1);
        assert_eq!(wrapped[0].text, "hello");
        assert_eq!(wrapped[0].logical_line, 0);
        assert!(wrapped[0].is_first);
    }

    #[test]
    fn test_wrap_text_single_wrap() {
        let text = "hello world foo bar";
        let wrapped = wrap_text(text, 12, 2); // effective width = 10
        assert_eq!(wrapped.len(), 2);
        assert_eq!(wrapped[0].text, "hello worl");
        assert!(wrapped[0].is_first);
        assert_eq!(wrapped[1].text, "d foo bar");
        assert!(!wrapped[1].is_first);
    }

    #[test]
    fn test_wrap_text_with_newlines() {
        let text = "hello\nworld";
        let wrapped = wrap_text(text, 20, 2);
        assert_eq!(wrapped.len(), 2);
        assert_eq!(wrapped[0].text, "hello");
        assert_eq!(wrapped[0].logical_line, 0);
        assert_eq!(wrapped[1].text, "world");
        assert_eq!(wrapped[1].logical_line, 1);
    }

    #[test]
    fn test_cursor_to_visual_pos_no_wrap() {
        let text = "hello";
        let (row, col) = cursor_to_visual_pos(text, 3, 20, 2);
        assert_eq!(row, 0);
        assert_eq!(col, 3);
    }

    #[test]
    fn test_cursor_to_visual_pos_with_wrap() {
        let text = "hello world foo bar";
        // With width=12, prefix=2, effective=10, wraps to:
        // Line 0: "hello worl" (pos 0-10)
        // Line 1: "d foo bar" (pos 10-19)
        let (row, col) = cursor_to_visual_pos(text, 12, 12, 2);
        assert_eq!(row, 1);
        assert_eq!(col, 2); // "d " = 2 chars into the wrapped line
    }
}
