# Agent Guidelines for opencode-helix

This document provides guidelines for AI coding agents working on the opencode-helix codebase.

## Project Overview

opencode-helix is a standalone Rust TUI application that integrates the [opencode](https://github.com/sst/opencode) AI assistant with the [Helix](https://helix-editor.com/) editor. It provides an external TUI for sending prompts to opencode from Helix via keybindings.

## Build Commands

```bash
# Check for errors (fast)
cargo check

# Build debug binary
cargo build

# Build release binary (optimized, stripped)
cargo build --release

# Install to ~/.cargo/bin
cargo install --path .

# Format code
cargo fmt

# Lint code
cargo clippy

# Run all tests
cargo test

# Run a single test by name
cargo test test_parse_ask

# Run tests in a specific module
cargo test cli::tests

# Run tests with output
cargo test -- --nocapture
```

## Project Structure

```
src/
├── main.rs           # Entry point, command dispatch, async runtime
├── cli.rs            # CLI argument parsing (clap derive)
├── config.rs         # Predefined prompts and menu item conversion
├── context.rs        # Editor context (placeholders @this, @buffer, etc.)
├── server/
│   ├── mod.rs        # Re-exports
│   ├── discovery.rs  # Find opencode processes via sysinfo
│   └── client.rs     # HTTP client for opencode API
└── tui/
    ├── mod.rs        # Re-exports
    ├── app.rs        # Main TUI app (terminal setup, key handling)
    ├── ask.rs        # Ask mode rendering (unused, logic in app.rs)
    └── select.rs     # Select mode rendering (unused, logic in app.rs)
```

## Code Style Guidelines

### Imports

Order imports in groups separated by blank lines:
1. Standard library (`std::`)
2. External crates (alphabetically)
3. Internal crate modules (`crate::`)

```rust
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cli::Cli;
use crate::context::Context as EditorContext;
```

### Module Documentation

Every module file should start with a `//!` doc comment:

```rust
//! CLI argument parsing for opencode-helix
```

### Function Documentation

Public functions should have `///` doc comments:

```rust
/// Create context from CLI arguments
pub fn from_cli(cli: &Cli) -> Self {
```

### Error Handling

- Use `anyhow::Result` for functions that can fail
- Use `.context()` to add context to errors
- Use `?` operator for propagation
- For optional failures that shouldn't stop execution, use `.ok()` or `.unwrap_or_default()`

```rust
// Propagate with context
let server = server::discover_server(&cwd, cli.port)
    .await
    .context("Failed to find opencode server")?;

// Optional failure (continue if fails)
let agents = client.get_agents().await.unwrap_or_default();
```

### Naming Conventions

- **Types**: PascalCase (`EditorContext`, `SelectItem`)
- **Functions/methods**: snake_case (`format_location`, `run_ask`)
- **Constants**: SCREAMING_SNAKE_CASE (`DEFAULT_PROMPTS`, `DEBUG_LOG_PATH`)
- **Modules**: snake_case (`server`, `cli`)

### Struct Definitions

Use `#[derive()]` for common traits. Order: Debug, Clone, Default, then Serialize/Deserialize:

```rust
#[derive(Debug, Clone, Default)]
pub struct Context {
    /// Current file path (relative)
    pub file: Option<String>,
    // ...
}
```

### Async Code

- Use `tokio` as the async runtime
- Mark async functions with `async fn`
- Use `.await` for async calls
- The `#[tokio::main]` attribute is on `main()`

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // ...
}

async fn run_ask(client: &server::Client, ctx: &EditorContext) -> Result<()> {
    client.send_prompt(&expanded, false, true).await?;
}
```

### TUI Code (ratatui)

- Terminal uses `/dev/tty` directly (required for Helix integration)
- Always restore terminal state before returning
- Use `App::new()` / `App::restore()` pattern

```rust
let mut app = App::new()?;
let result = app.run_ask(initial, hint, ctx)?;
app.restore()?;
drop(app);
// Now safe to do async operations
```

### Testing

- Tests go in a `#[cfg(test)]` module at the end of each file
- Use `Cli::parse_from()` to test CLI parsing
- Test names: `test_<what_is_being_tested>`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_location_cursor() {
        let ctx = Context {
            file: Some("src/main.rs".to_string()),
            line: Some(42),
            ..Default::default()
        };
        assert_eq!(ctx.format_location(), Some("@src/main.rs L42".to_string()));
    }
}
```

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing (derive macro) |
| `ratatui` | Terminal UI framework |
| `crossterm` | Terminal manipulation |
| `tokio` | Async runtime |
| `reqwest` | HTTP client |
| `serde` / `serde_json` | JSON serialization |
| `anyhow` | Error handling |
| `sysinfo` | Process discovery |
| `chrono` | Timestamps for debug logging |

## Debug Mode

Use `--debug` flag to enable logging to `/tmp/opencode-helix-debug.log`:

```bash
opencode-helix ask --debug -f test.rs -l 1 --cwd .
cat /tmp/opencode-helix-debug.log
```

Add debug logging in functions:

```rust
debug_log(debug, &format!("run_ask: expanded = {}", expanded));
```

## Common Patterns

### Context Placeholder Expansion

The `Context` struct handles placeholder expansion:
- `@this` - file + cursor/selection position
- `@buffer` - relative file path
- `@path` - absolute file path
- `@selection` - selection content with code block
- `@diff` - git diff output

### HTTP Client

The `server::Client` communicates with opencode via HTTP:
- `GET /path` - server working directory
- `GET /agent` - list agents
- `GET /command` - list commands
- `POST /tui/publish` - send TUI commands
