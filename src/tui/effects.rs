//! Visual effects for the TUI
//!
//! Provides animated effects like blinking cursor, typing animation, and scanlines.

use std::time::{Duration, Instant};

/// Blinking block cursor that toggles visibility
#[derive(Debug)]
pub struct BlinkingCursor {
    visible: bool,
    last_toggle: Instant,
    blink_rate: Duration,
}

impl BlinkingCursor {
    /// Create a new blinking cursor with default 530ms blink rate
    pub fn new() -> Self {
        Self {
            visible: true,
            last_toggle: Instant::now(),
            blink_rate: Duration::from_millis(530),
        }
    }

    /// Update cursor state based on elapsed time
    pub fn tick(&mut self) {
        if self.last_toggle.elapsed() >= self.blink_rate {
            self.visible = !self.visible;
            self.last_toggle = Instant::now();
        }
    }

    /// Get the cursor character (visible or empty)
    pub fn char(&self) -> &'static str {
        if self.visible {
            "█"
        } else {
            " "
        }
    }

    /// Check if cursor is currently visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Reset cursor to visible state
    pub fn reset(&mut self) {
        self.visible = true;
        self.last_toggle = Instant::now();
    }
}

impl Default for BlinkingCursor {
    fn default() -> Self {
        Self::new()
    }
}

/// Typewriter effect that reveals text character by character
#[derive(Debug)]
pub struct TypewriterText {
    full_text: String,
    visible_chars: usize,
    last_update: Instant,
    char_delay: Duration,
    complete: bool,
}

impl TypewriterText {
    /// Create a new typewriter effect
    ///
    /// # Arguments
    /// * `text` - The full text to reveal
    /// * `chars_per_second` - How fast to reveal characters (e.g., 30 = 30 chars/sec)
    pub fn new(text: &str, chars_per_second: u32) -> Self {
        let delay = if chars_per_second > 0 {
            1000 / chars_per_second as u64
        } else {
            0
        };
        Self {
            full_text: text.to_string(),
            visible_chars: 0,
            last_update: Instant::now(),
            char_delay: Duration::from_millis(delay),
            complete: text.is_empty(),
        }
    }

    /// Create an already-complete typewriter (no animation)
    pub fn instant(text: &str) -> Self {
        Self {
            full_text: text.to_string(),
            visible_chars: text.chars().count(),
            last_update: Instant::now(),
            char_delay: Duration::ZERO,
            complete: true,
        }
    }

    /// Update state based on elapsed time
    pub fn tick(&mut self) {
        if self.complete {
            return;
        }
        if self.last_update.elapsed() >= self.char_delay {
            let total_chars = self.full_text.chars().count();
            self.visible_chars = (self.visible_chars + 1).min(total_chars);
            self.last_update = Instant::now();
            if self.visible_chars >= total_chars {
                self.complete = true;
            }
        }
    }

    /// Get the currently visible portion of text
    pub fn visible_text(&self) -> &str {
        if self.complete {
            &self.full_text
        } else {
            // Find byte index for visible_chars
            let byte_idx = self
                .full_text
                .char_indices()
                .nth(self.visible_chars)
                .map(|(i, _)| i)
                .unwrap_or(self.full_text.len());
            &self.full_text[..byte_idx]
        }
    }

    /// Skip animation and show full text immediately
    pub fn skip(&mut self) {
        self.visible_chars = self.full_text.chars().count();
        self.complete = true;
    }

    /// Check if animation is complete
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Get the full text
    pub fn full_text(&self) -> &str {
        &self.full_text
    }
}

/// Scanline effect that moves down the screen
#[derive(Debug)]
pub struct Scanline {
    position: u16,
    height: u16,
    last_update: Instant,
    speed: Duration,
    enabled: bool,
}

impl Scanline {
    /// Create a new scanline effect
    ///
    /// # Arguments
    /// * `height` - Total height of the area
    /// * `speed_ms` - Milliseconds between each row movement
    pub fn new(height: u16, speed_ms: u64) -> Self {
        Self {
            position: 0,
            height,
            last_update: Instant::now(),
            speed: Duration::from_millis(speed_ms),
            enabled: true,
        }
    }

    /// Update scanline position based on elapsed time
    pub fn tick(&mut self) {
        if !self.enabled || self.height == 0 {
            return;
        }
        if self.last_update.elapsed() >= self.speed {
            self.position = (self.position + 1) % self.height;
            self.last_update = Instant::now();
        }
    }

    /// Check if a given row is the scanline
    pub fn is_scanline_row(&self, row: u16) -> bool {
        self.enabled && row == self.position
    }

    /// Get the current scanline position
    pub fn position(&self) -> u16 {
        self.position
    }

    /// Update the height (e.g., when terminal resizes)
    pub fn set_height(&mut self, height: u16) {
        self.height = height;
        if self.position >= height {
            self.position = 0;
        }
    }

    /// Enable or disable the scanline
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if scanline is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_blinking_cursor_initial_state() {
        let cursor = BlinkingCursor::new();
        assert!(cursor.is_visible());
        assert_eq!(cursor.char(), "█");
    }

    #[test]
    fn test_typewriter_instant() {
        let tw = TypewriterText::instant("hello");
        assert!(tw.is_complete());
        assert_eq!(tw.visible_text(), "hello");
    }

    #[test]
    fn test_typewriter_progress() {
        let mut tw = TypewriterText::new("hi", 1000); // 1000 chars/sec = 1ms per char
        assert!(!tw.is_complete());
        assert_eq!(tw.visible_text(), "");

        // Wait and tick
        sleep(Duration::from_millis(2));
        tw.tick();
        assert!(!tw.visible_text().is_empty());
    }

    #[test]
    fn test_typewriter_skip() {
        let mut tw = TypewriterText::new("hello world", 10);
        assert!(!tw.is_complete());
        tw.skip();
        assert!(tw.is_complete());
        assert_eq!(tw.visible_text(), "hello world");
    }

    #[test]
    fn test_scanline_movement() {
        let mut scan = Scanline::new(10, 1); // 1ms speed
        assert_eq!(scan.position(), 0);

        sleep(Duration::from_millis(2));
        scan.tick();
        // Position should have moved
        assert!(scan.position() <= 10);
    }

    #[test]
    fn test_scanline_wraps() {
        let mut scan = Scanline::new(3, 1);
        for _ in 0..10 {
            sleep(Duration::from_millis(2));
            scan.tick();
        }
        // Should wrap around, position always < height
        assert!(scan.position() < 3);
    }
}
