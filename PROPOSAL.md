# opencode-helix: Proposal Document

## Overview

This document proposes the design for `opencode-helix`, a standalone TUI application that integrates the [opencode](https://github.com/sst/opencode) AI assistant with the [Helix](https://helix-editor.com/) editor.

### Motivation

The existing `opencode.nvim` plugin provides deep Neovim integration with the opencode AI assistant. Helix, being a modal editor written in Rust, doesn't support traditional plugins but allows integration via:
- External commands with `:insert-output`
- Shell pipes with `|` (pipe selection to command)
- Custom keybindings in `config.toml`

This proposal outlines a standalone TUI that bridges Helix and opencode using these mechanisms.

---

## Architecture

### High-Level Design

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Helix Editor   │────▶│  opencode-helix  │────▶│  opencode CLI   │
│                 │     │  (External TUI)  │     │  (HTTP Server)  │
│  - Keybindings  │     │                  │     │                 │
│  - :insert-output│◀────│  - Input prompt  │◀────│  - /tui/publish │
│  - Pipe (|)     │     │  - Selection menu│     │  - /event (SSE) │
└─────────────────┘     └──────────────────┘     └─────────────────┘
```

### Integration Pattern

Helix integration follows the yazi file manager pattern:

```toml
# ~/.config/helix/config.toml
[keys.normal]
C-o = [":new", ":insert-output opencode-helix ask", ":buffer-close!", ":redraw"]
```

This pattern:
1. Opens a new scratch buffer (`:new`)
2. Runs the external TUI and captures output (`:insert-output`)
3. Closes the scratch buffer (`:buffer-close!`)
4. Redraws the screen (`:redraw`)

---

## Components

### 1. CLI Interface

```
opencode-helix [OPTIONS] <COMMAND>

Commands:
  ask       Open input prompt to type a message
  select    Open menu to select from prompts/commands
  prompt    Send a prompt directly (non-interactive)
  status    Show current opencode status

Options:
  -p, --port <PORT>           Connect to specific port
  -f, --file <PATH>           Current file path (for @this context)
  -l, --line <NUM>            Cursor line number (1-based)
  -c, --column <NUM>          Cursor column number (1-based)
  --selection-file <PATH>     Read selection from temp file (file is deleted after reading)
  --selection-start <NUM>     Selection start line (1-based)
  --selection-end <NUM>       Selection end line (1-based)
  --cwd <PATH>                Working directory (for server discovery)
  --language <LANG>           File language (e.g., "rust", "python")
```

### Helix Variables (Available Context)

Helix provides these variables for command expansion:

| Variable | Description |
|----------|-------------|
| `%{buffer_name}` | Relative path of current file |
| `%{file_path_absolute}` | Absolute path of current file |
| `%{cursor_line}` | Cursor line number (1-based) |
| `%{cursor_column}` | Cursor column (grapheme clusters, 1-based) |
| `%{selection}` | Contents of primary selection |
| `%{selection_line_start}` | Selection start line (1-based) |
| `%{selection_line_end}` | Selection end line (1-based) |
| `%{language}` | Language name of current file |
| `%{current_working_directory}` | Current working directory |
| `%{workspace_directory}` | Workspace root (.git, .helix, etc.) |

### 2. Server Discovery

Discover running opencode servers using the same logic as the Neovim plugin:

1. **Fixed port**: If `--port` specified, use it directly
2. **Process discovery**: 
   - Find processes matching `opencode.*--port`
   - Query each on `/path` endpoint to validate
   - Match against current working directory
3. **Auto-start**: Optionally start opencode if not running

```rust
// Pseudo-code
fn discover_server(cwd: &Path, port: Option<u16>) -> Result<Server> {
    if let Some(p) = port {
        return validate_server(p);
    }
    
    for process in find_opencode_processes()? {
        if let Ok(server) = validate_server(process.port) {
            if server.cwd.starts_with(cwd) {
                return Ok(server);
            }
        }
    }
    
    Err(Error::NoServerFound)
}
```

### 3. HTTP Client

Communicate with opencode server via HTTP:

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/path` | GET | Get server working directory |
| `/tui/publish` | POST | Send TUI commands |
| `/event` | GET | SSE subscription |
| `/agent` | GET | List available agents |
| `/command` | GET | List custom commands |

#### TUI Commands

```rust
// Append text to prompt
POST /tui/publish
{
  "type": "tui.prompt.append",
  "properties": { "text": "..." }
}

// Execute command
POST /tui/publish
{
  "type": "tui.command.execute",
  "properties": { "command": "prompt.submit" }
}
```

### 4. TUI Modes

#### Ask Mode (Input Prompt)

A minimal input prompt for typing messages:

```
┌─ opencode ──────────────────────────────────┐
│                                             │
│  > Explain this code @this                  │
│    ▊                                        │
│                                             │
│  [Enter] Send  [Esc] Cancel  [Tab] Complete │
└─────────────────────────────────────────────┘
```

Features:
- Single-line or multi-line input
- Context placeholder expansion (`@this`, `@buffer`, etc.)
- Tab completion for contexts and agents
- Vim-like keybindings (optional)

#### Select Mode (Menu)

A selection menu for predefined prompts and commands:

```
┌─ opencode ──────────────────────────────────┐
│  PROMPTS                                    │
│  > Explain    Explain how this code works   │
│    Review     Review this code              │
│    Fix        Fix the issue in this code    │
│    Implement  Implement based on context    │
│                                             │
│  COMMANDS                                   │
│    /compact   Summarize conversation        │
│    /clear     Clear conversation            │
│                                             │
│  [↑↓] Navigate  [Enter] Select  [Esc] Cancel│
└─────────────────────────────────────────────┘
```

Features:
- Fuzzy filtering with `/` search
- Grouped sections (prompts, commands, agents)
- Preview pane showing expanded prompt

### 5. Context System

Map Helix variables to opencode format:

| Context | Helix Source | opencode Format |
|---------|--------------|-----------------|
| `@this` | `--file`, `--line`, `--column` | `@file.rs L42:C10` |
| `@selection` | `--selection-file`, `--selection-start/end` | `@file.rs L10-L20` + selection text |
| `@buffer` | `--file` | `@file.rs` |
| `@diff` | `git diff` (executed) | Git diff output |

#### Context Expansion

When `@this` is used in a prompt:
- If selection is provided: expands to file + selection range
- Otherwise: expands to file + cursor position

Example:
```
Input:  "Explain @this"
Output: "Explain @src/main.rs L42:C10-L50:C1"
```

#### Helix Integration for Context

```toml
# Full context with cursor position (normal mode)
C-o = [
  ":new",
  ":insert-output opencode-helix ask -f %{buffer_name} -l %{cursor_line} -c %{cursor_column} --cwd %{workspace_directory} --language %{language}",
  ":buffer-close!",
  ":redraw"
]

# With selection (select mode) - uses temp file
C-S-o = [
  ":write ~/.cache/opencode-selection.tmp",
  ":new",
  ":insert-output opencode-helix ask -f %{buffer_name} --selection-file ~/.cache/opencode-selection.tmp --selection-start %{selection_line_start} --selection-end %{selection_line_end} --cwd %{workspace_directory}",
  ":buffer-close!",
  ":redraw"
]
```
Input:  "Explain @this"
Output: "Explain @src/main.rs L42:C10-L50:C1"
```

#### Helix Integration for Context

```toml
# Full context with cursor position
C-o = [
  ":new",
  ":insert-output opencode-helix ask -f %{buffer_name} -l %{cursor_line} -c %{cursor_column} --cwd %{workspace_directory}",
  ":buffer-close!",
  ":redraw"
]

# With selection
C-S-o = [
  ":new",
  ":insert-output opencode-helix ask -f %{buffer_name} -s %{selection} --selection-start %{selection_line_start} --selection-end %{selection_line_end} --cwd %{workspace_directory}",
  ":buffer-close!",
  ":redraw"
]
```

---

## Data Flow

### Ask Flow

```
1. User presses keybinding in Helix
2. Helix runs: opencode-helix ask -f current_file.rs -l 42
3. opencode-helix:
   a. Discovers opencode server
   b. Renders input TUI
   c. User types: "Explain @this"
   d. Expands context: "Explain @current_file.rs L42:C1"
   e. POST /tui/publish with prompt
   f. Exits cleanly
4. Helix closes scratch buffer, redraws
5. opencode TUI (running separately) shows the response
```

### Select Flow

```
1. User presses keybinding in Helix
2. Helix runs: opencode-helix select -f current_file.rs
3. opencode-helix:
   a. Discovers opencode server
   b. GET /command to fetch custom commands
   c. Renders selection TUI
   d. User selects "Review"
   e. Expands prompt with context
   f. POST /tui/publish with prompt
   g. Exits cleanly
4. Helix closes scratch buffer, redraws
```

---

## Module Structure

```
helix/
├── Cargo.toml
├── src/
│   ├── main.rs           # Entry point, CLI parsing
│   ├── cli.rs            # Clap argument definitions
│   ├── server/
│   │   ├── mod.rs
│   │   ├── discovery.rs  # Find opencode processes
│   │   └── client.rs     # HTTP client for opencode API
│   ├── context.rs        # Context placeholder handling
│   ├── tui/
│   │   ├── mod.rs
│   │   ├── app.rs        # Main TUI application state
│   │   ├── ask.rs        # Ask mode (input prompt)
│   │   ├── select.rs     # Select mode (menu)
│   │   └── widgets.rs    # Custom ratatui widgets
│   └── config.rs         # Configuration and prompts
```

---

## Configuration

### Default Prompts

Built-in prompts matching opencode.nvim defaults:

```rust
const DEFAULT_PROMPTS: &[Prompt] = &[
    Prompt {
        name: "explain",
        prompt: "Explain how this code works: @this",
        description: "Explain the selected code",
    },
    Prompt {
        name: "review", 
        prompt: "Review this code and suggest improvements: @this",
        description: "Code review",
    },
    Prompt {
        name: "fix",
        prompt: "Fix the issue in this code: @this",
        description: "Fix code issues",
    },
    Prompt {
        name: "implement",
        prompt: "Implement based on the context: @this",
        description: "Implement code",
    },
];
```

### User Configuration (Optional)

```toml
# ~/.config/opencode-helix/config.toml
[prompts]
test = { prompt = "Write tests for: @this", description = "Generate tests" }

[contexts]
# Custom context mappings

[settings]
auto_submit = true  # Submit prompt immediately after selection
```

---

## Helix Keybinding Examples

### Basic Setup (No Context)

```toml
# ~/.config/helix/config.toml

[keys.normal]
# Open ask prompt (no context)
C-o = [":new", ":insert-output opencode-helix ask", ":buffer-close!", ":redraw"]

# Open selection menu
C-S-o = [":new", ":insert-output opencode-helix select", ":buffer-close!", ":redraw"]
```

### Full Context Setup (Recommended)

```toml
# ~/.config/helix/config.toml

[keys.normal]
# Ask with cursor context
C-o = [
  ":new",
  ":insert-output opencode-helix ask -f %{buffer_name} -l %{cursor_line} -c %{cursor_column} --cwd %{workspace_directory} --language %{language}",
  ":buffer-close!",
  ":redraw"
]

# Select with cursor context
C-S-o = [
  ":new",
  ":insert-output opencode-helix select -f %{buffer_name} -l %{cursor_line} -c %{cursor_column} --cwd %{workspace_directory} --language %{language}",
  ":buffer-close!",
  ":redraw"
]

[keys.select]
# Ask with selection context (from visual/select mode)
# Writes selection to temp file first, then invokes opencode-helix
C-o = [
  ":write ~/.cache/opencode-selection.tmp",
  ":new",
  ":insert-output opencode-helix ask -f %{buffer_name} --selection-file ~/.cache/opencode-selection.tmp --selection-start %{selection_line_start} --selection-end %{selection_line_end} --cwd %{workspace_directory} --language %{language}",
  ":buffer-close!",
  ":redraw"
]

# Select menu with selection context
C-S-o = [
  ":write ~/.cache/opencode-selection.tmp",
  ":new",
  ":insert-output opencode-helix select -f %{buffer_name} --selection-file ~/.cache/opencode-selection.tmp --selection-start %{selection_line_start} --selection-end %{selection_line_end} --cwd %{workspace_directory} --language %{language}",
  ":buffer-close!",
  ":redraw"
]

[keys.normal.space.a]  # Space-a menu for AI
i = [":new", ":insert-output opencode-helix ask -f %{buffer_name} -l %{cursor_line} --cwd %{workspace_directory}", ":buffer-close!", ":redraw"]
s = [":new", ":insert-output opencode-helix select -f %{buffer_name} -l %{cursor_line} --cwd %{workspace_directory}", ":buffer-close!", ":redraw"]
e = [":new", ":insert-output opencode-helix prompt explain -f %{buffer_name} -l %{cursor_line} --cwd %{workspace_directory}", ":buffer-close!", ":redraw"]
r = [":new", ":insert-output opencode-helix prompt review -f %{buffer_name} -l %{cursor_line} --cwd %{workspace_directory}", ":buffer-close!", ":redraw"]
```

---

## Implementation Phases

### Phase 1: Core Foundation
- [ ] CLI argument parsing with clap
- [ ] Server discovery (find opencode processes)
- [ ] HTTP client for opencode API
- [ ] Basic `prompt` command (non-interactive)

### Phase 2: TUI Implementation
- [ ] Ratatui app scaffolding
- [ ] Ask mode with input field
- [ ] Select mode with menu
- [ ] Keyboard navigation

### Phase 3: Context & Polish
- [ ] Context placeholder expansion (@this, @buffer, @selection)
- [ ] Selection text handling (escaping, large selections)
- [ ] Tab completion for contexts and agents
- [ ] Configuration file support
- [ ] Error handling and user feedback

### Phase 4: Documentation
- [ ] README with installation instructions
- [ ] Helix configuration examples
- [ ] Troubleshooting guide

---

## Comparison with opencode.nvim

| Feature | opencode.nvim | opencode-helix |
|---------|---------------|----------------|
| Integration | Native plugin | External TUI |
| Context | Full editor state | Helix variables via CLI |
| Selection | Visual mode marks | `%{selection}` variable |
| SSE Events | Auto-reload, status | Not in initial version |
| Completion | blink.cmp integration | Built-in basic |
| Prompts | Configurable | Built-in + config file |
| Providers | terminal, tmux, etc. | N/A (separate process) |

---

## Design Decisions

1. **Selection handling**: Use temporary files for passing selection data.
   - Helix writes selection to temp file, opencode-helix reads it
   - Avoids shell escaping issues with special characters, quotes, newlines
   - Works reliably for large selections
   - Temp file is cleaned up after reading

2. **Auto-start opencode**: No (initial version)
   - User must have opencode running separately
   - Keeps implementation simple and predictable

3. **SSE for status**: No (initial version)
   - Keep it simple for v1
   - Future: Could show status in a persistent panel

### Selection via Temporary File

Instead of passing `%{selection}` directly (which has escaping issues), use a wrapper approach:

```toml
# Helix keybinding that writes selection to temp file
[keys.select]
C-o = [
  ":write %{config_dir}/opencode-selection.tmp",  # Write selection to temp
  ":new",
  ":insert-output opencode-helix ask -f %{buffer_name} --selection-file %{config_dir}/opencode-selection.tmp --selection-start %{selection_line_start} --selection-end %{selection_line_end}",
  ":buffer-close!",
  ":redraw"
]
```

Or use shell to create temp file:

```toml
[keys.select]
C-o = [
  ":new",
  ":insert-output sh -c 'echo %{selection} > /tmp/opencode-sel.tmp && opencode-helix ask -f %{buffer_name} --selection-file /tmp/opencode-sel.tmp'",
  ":buffer-close!",
  ":redraw"
]
```

**CLI flag addition:**
```
--selection-file <PATH>    Read selection from file (cleaned up after reading)
```

---

## Success Criteria

1. User can send prompts to opencode from Helix with 2-3 keypresses
2. Context (current file, line) is correctly passed
3. Clean exit without leaving artifacts
4. Works alongside opencode running in a terminal split/tmux pane
5. Response time < 100ms for TUI to appear

---

## Next Steps

1. Review and approve this proposal
2. Implement Phase 1 (core foundation)
3. Test with actual opencode server
4. Iterate on TUI design based on feedback
