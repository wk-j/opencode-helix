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
use std::time::Duration;

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
}

impl App {
    /// Create a new TUI application
    /// Uses /dev/tty directly to support running via Helix's :insert-output
    pub fn new() -> Result<Self> {
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
    pub fn run_ask(&mut self, initial: &str, context_hint: Option<&str>) -> Result<AppResult> {
        let mut input = initial.to_string();
        let mut cursor_pos = input.len();
        // Focus: 0 = input, 1 = Send button, 2 = Cancel button
        let mut focus: u8 = 0;

        loop {
            // Draw UI
            self.terminal.draw(|frame| {
                let area = frame.area();

                // Center the dialog
                let dialog_width = area.width.min(60);
                let dialog_height = 8; // Increased for button row
                let dialog_area = Rect {
                    x: (area.width - dialog_width) / 2,
                    y: (area.height - dialog_height) / 2,
                    width: dialog_width,
                    height: dialog_height,
                };

                // Clear background
                frame.render_widget(Clear, dialog_area);

                // Dialog box
                let block = Block::default()
                    .title(" opencode ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan));

                let inner = block.inner(dialog_area);
                frame.render_widget(block, dialog_area);

                // Context hint (if any)
                let input_area = if let Some(hint) = context_hint {
                    let hint_para =
                        Paragraph::new(hint).style(Style::default().fg(Color::DarkGray));
                    let hint_area = Rect {
                        x: inner.x,
                        y: inner.y,
                        width: inner.width,
                        height: 1,
                    };
                    frame.render_widget(hint_para, hint_area);

                    Rect {
                        x: inner.x,
                        y: inner.y + 1,
                        width: inner.width,
                        height: inner.height - 1,
                    }
                } else {
                    inner
                };

                // Input field
                let input_style = if focus == 0 {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let display_input = format!("> {}", input);
                let input_para = Paragraph::new(display_input.as_str()).style(input_style);
                frame.render_widget(
                    input_para,
                    Rect {
                        x: input_area.x,
                        y: input_area.y,
                        width: input_area.width,
                        height: 1,
                    },
                );

                // Buttons row
                let button_y = input_area.y + 2;

                // Send button
                let send_style = if focus == 1 {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default().fg(Color::Cyan)
                };
                let send_btn = Paragraph::new(" [Send] ").style(send_style);
                frame.render_widget(
                    send_btn,
                    Rect {
                        x: input_area.x,
                        y: button_y,
                        width: 8,
                        height: 1,
                    },
                );

                // Cancel button
                let cancel_style = if focus == 2 {
                    Style::default().fg(Color::Black).bg(Color::Red)
                } else {
                    Style::default().fg(Color::Red)
                };
                let cancel_btn = Paragraph::new(" [Cancel] ").style(cancel_style);
                frame.render_widget(
                    cancel_btn,
                    Rect {
                        x: input_area.x + 10,
                        y: button_y,
                        width: 10,
                        height: 1,
                    },
                );

                // Help text
                let help = "[Tab] Navigate  [Enter] Select  [Esc] Cancel";
                let help_para = Paragraph::new(help).style(Style::default().fg(Color::DarkGray));
                frame.render_widget(
                    help_para,
                    Rect {
                        x: input_area.x,
                        y: input_area.y + input_area.height - 1,
                        width: input_area.width,
                        height: 1,
                    },
                );

                // Position cursor only when input is focused
                if focus == 0 {
                    frame.set_cursor_position(Position {
                        x: input_area.x + 2 + cursor_pos as u16,
                        y: input_area.y,
                    });
                }
            })?;

            // Handle input from /dev/tty
            if let Some(key) = self.read_key(Duration::from_millis(100))? {
                match key.code {
                    KeyCode::Tab => {
                        // Cycle focus: input -> Send -> Cancel -> input
                        focus = (focus + 1) % 3;
                    }
                    KeyCode::BackTab => {
                        // Reverse cycle
                        focus = if focus == 0 { 2 } else { focus - 1 };
                    }
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
                    // Only handle text input when input field is focused
                    KeyCode::Char(c) if focus == 0 => {
                        input.insert(cursor_pos, c);
                        cursor_pos += 1;
                    }
                    KeyCode::Backspace if focus == 0 => {
                        if cursor_pos > 0 {
                            input.remove(cursor_pos - 1);
                            cursor_pos -= 1;
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
                        }
                    }
                    KeyCode::Right if focus == 0 => {
                        if cursor_pos < input.len() {
                            cursor_pos += 1;
                        }
                    }
                    KeyCode::Home if focus == 0 => {
                        cursor_pos = 0;
                    }
                    KeyCode::End if focus == 0 => {
                        cursor_pos = input.len();
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
    pub fn run_select(&mut self, items: &[SelectItem]) -> Result<AppResult> {
        if items.is_empty() {
            return Ok(AppResult::Cancel);
        }

        let mut selected = 0;
        let mut filter = String::new();

        loop {
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

                // Dialog box
                let block = Block::default()
                    .title(" opencode - Select ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan));

                let inner = block.inner(dialog_area);
                frame.render_widget(block, dialog_area);

                // Filter input
                let filter_display = format!("/ {}", filter);
                let filter_para = Paragraph::new(filter_display.as_str())
                    .style(Style::default().fg(Color::Yellow));
                frame.render_widget(
                    filter_para,
                    Rect {
                        x: inner.x,
                        y: inner.y,
                        width: inner.width,
                        height: 1,
                    },
                );

                // Items
                let items_area = Rect {
                    x: inner.x,
                    y: inner.y + 2,
                    width: inner.width,
                    height: inner.height.saturating_sub(4),
                };

                for (i, (_, item)) in filtered.iter().enumerate() {
                    if i as u16 >= items_area.height {
                        break;
                    }

                    let style = if i == selected {
                        Style::default().fg(Color::Black).bg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    let prefix = if i == selected { "> " } else { "  " };
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

                // Help text
                let help = "[↑↓] Navigate  [Enter] Select  [Esc] Cancel  [/] Filter";
                let help_para = Paragraph::new(help).style(Style::default().fg(Color::DarkGray));
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
            if let Some(key) = self.read_key(Duration::from_millis(100))? {
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
                    KeyCode::Down | KeyCode::Char('j') => {
                        if selected < filtered.len().saturating_sub(1) {
                            selected += 1;
                        }
                    }
                    KeyCode::Char(c) => {
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
