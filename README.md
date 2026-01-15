# opencode-helix

External TUI for integrating [opencode](https://github.com/sst/opencode) AI assistant with the [Helix](https://helix-editor.com/) editor.

## Features

- **Ask mode**: Open an input prompt to type messages to opencode
- **Select mode**: Choose from predefined prompts, commands, and agents
- **Context support**: Pass file path, cursor position, and selection from Helix
- **Auto-discovery**: Automatically finds running opencode server in your project
- **Lightweight**: ~2.6MB single binary, fast startup

## Installation

### From source

```bash
cargo install --path .
```

### Requirements

- [opencode](https://github.com/sst/opencode) must be running (e.g., in a tmux pane or separate terminal)
- Helix editor

## Helix Configuration

Add keybindings to your `~/.config/helix/config.toml`:

> **Important**: Context variables like `%{buffer_name}` must be captured **before** `:new` 
> because `:new` creates a scratch buffer which loses the original context. We use `:sh` 
> to save values to cache files, then read them back with `%sh{cat ...}`.

### Recommended Setup

The keybindings use `;` as a prefix. Before using, create the cache directory:

```bash
mkdir -p ~/.cache/helix
```

```toml
# opencode-helix AI integration
# Normal mode: ;i (ask), ;s (select menu) - cursor position context
# Select mode: ;e (explain), ;r (review) - with selection content

[keys.normal.";"]
# ;i = Ask - Opens input prompt to type a custom question to AI
i = [
    ":sh echo '%{buffer_name}' > ~/.cache/helix/opencode_file && echo '%{cursor_line}' > ~/.cache/helix/opencode_line && echo '%{cursor_column}' > ~/.cache/helix/opencode_col && echo '%{language}' > ~/.cache/helix/opencode_lang && echo '%{workspace_directory}' > ~/.cache/helix/opencode_cwd",
    ":new",
    ":insert-output opencode-helix ask -f %sh{cat ~/.cache/helix/opencode_file} -l %sh{cat ~/.cache/helix/opencode_line} -c %sh{cat ~/.cache/helix/opencode_col} --cwd %sh{cat ~/.cache/helix/opencode_cwd} --language %sh{cat ~/.cache/helix/opencode_lang}",
    ":buffer-close!",
    ":redraw",
]
# ;s = Select - Opens menu to choose from predefined prompts, commands, and agents
s = [
    ":sh echo '%{buffer_name}' > ~/.cache/helix/opencode_file && echo '%{cursor_line}' > ~/.cache/helix/opencode_line && echo '%{cursor_column}' > ~/.cache/helix/opencode_col && echo '%{language}' > ~/.cache/helix/opencode_lang && echo '%{workspace_directory}' > ~/.cache/helix/opencode_cwd",
    ":new",
    ":insert-output opencode-helix select -f %sh{cat ~/.cache/helix/opencode_file} -l %sh{cat ~/.cache/helix/opencode_line} -c %sh{cat ~/.cache/helix/opencode_col} --cwd %sh{cat ~/.cache/helix/opencode_cwd} --language %sh{cat ~/.cache/helix/opencode_lang}",
    ":buffer-close!",
    ":redraw",
]

# Select mode: uses :pipe-to for robust selection capture (handles quotes, newlines, special chars)
[keys.select.";"]
# ;e = Explain selection
e = [
    ":sh echo '%{buffer_name}' > ~/.cache/helix/opencode_file && echo '%{selection_line_start}' > ~/.cache/helix/opencode_sel_start && echo '%{selection_line_end}' > ~/.cache/helix/opencode_sel_end && echo '%{language}' > ~/.cache/helix/opencode_lang && echo '%{workspace_directory}' > ~/.cache/helix/opencode_cwd",
    ":pipe-to cat > ~/.cache/helix/opencode_selection.tmp",
    ":new",
    ":insert-output opencode-helix prompt explain -f %sh{cat ~/.cache/helix/opencode_file} --selection-file ~/.cache/helix/opencode_selection.tmp --selection-start %sh{cat ~/.cache/helix/opencode_sel_start} --selection-end %sh{cat ~/.cache/helix/opencode_sel_end} --cwd %sh{cat ~/.cache/helix/opencode_cwd} --language %sh{cat ~/.cache/helix/opencode_lang}",
    ":buffer-close!",
    ":redraw",
]
# ;r = Review selection
r = [
    ":sh echo '%{buffer_name}' > ~/.cache/helix/opencode_file && echo '%{selection_line_start}' > ~/.cache/helix/opencode_sel_start && echo '%{selection_line_end}' > ~/.cache/helix/opencode_sel_end && echo '%{language}' > ~/.cache/helix/opencode_lang && echo '%{workspace_directory}' > ~/.cache/helix/opencode_cwd",
    ":pipe-to cat > ~/.cache/helix/opencode_selection.tmp",
    ":new",
    ":insert-output opencode-helix prompt review -f %sh{cat ~/.cache/helix/opencode_file} --selection-file ~/.cache/helix/opencode_selection.tmp --selection-start %sh{cat ~/.cache/helix/opencode_sel_start} --selection-end %sh{cat ~/.cache/helix/opencode_sel_end} --cwd %sh{cat ~/.cache/helix/opencode_cwd} --language %sh{cat ~/.cache/helix/opencode_lang}",
    ":buffer-close!",
    ":redraw",
]
```

### Keybinding Summary

| Mode | Key | Action |
|------|-----|--------|
| Normal | `;i` | Ask - open input prompt |
| Normal | `;s` | Select - open menu |
| Select | `;e` | Explain selected code |
| Select | `;r` | Review selected code |

To use selection-based commands, first select text with `x` (line), `v` (char), or `V` (extend), then press the keybinding.

## Usage

### Commands

```bash
# Open input prompt
opencode-helix ask

# Open selection menu
opencode-helix select

# Send a predefined prompt
opencode-helix prompt explain

# Send raw text
opencode-helix prompt "Fix the bug in this function"

# Check server status
opencode-helix status
```

### Context Placeholders

In prompts, use these placeholders to include editor context:

| Placeholder | Description |
|-------------|-------------|
| `@this` | Current file + cursor/selection position |
| `@buffer` | Current file reference (relative path) |
| `@path` | Absolute file path |
| `@selection` | Selection with content |
| `@diff` | Git diff output |

**Tip:** Press `?` in the ask prompt to toggle a panel showing all placeholders and their current values.

### Predefined Prompts

| Name | Description |
|------|-------------|
| `explain` | Explain how the code works |
| `review` | Code review with suggestions |
| `fix` | Fix issues in the code |
| `implement` | Implement based on context |
| `tests` | Generate tests |
| `docs` | Add documentation |
| `refactor` | Refactor for maintainability |
| `optimize` | Optimize performance |

## How It Works

1. **Keybinding triggers**: Helix runs `opencode-helix` via `:insert-output`
2. **Server discovery**: Finds running opencode server matching your project
3. **TUI renders**: Shows input prompt or selection menu
4. **Context expansion**: Replaces `@this`, `@buffer`, etc. with actual values
5. **Send to opencode**: Posts prompt via HTTP to opencode's TUI API
6. **Clean exit**: Returns control to Helix

The opencode TUI (running in another terminal/tmux pane) will show the response.

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Helix Editor   │────▶│  opencode-helix  │────▶│  opencode CLI   │
│                 │     │  (External TUI)  │     │  (HTTP Server)  │
│  - Keybindings  │     │                  │     │                 │
│  - :insert-output│◀────│  - Input prompt  │◀────│  - /tui/publish │
│                 │     │  - Selection menu│     │  - /command     │
└─────────────────┘     └──────────────────┘     └─────────────────┘
```

## Development

```bash
# Check code
cargo check

# Run tests
cargo test

# Build debug
cargo build

# Build release
cargo build --release
```

## License

MIT
