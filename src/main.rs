//! opencode-helix: External TUI for integrating opencode AI with Helix editor

mod cli;
mod config;
mod context;
mod server;
mod tui;

use anyhow::{Context, Result};
use cli::{Cli, Command};
use context::Context as EditorContext;
use tui::app::{App, AppResult, SelectItem};

const DEBUG_LOG_PATH: &str = "/tmp/opencode-helix-debug.log";

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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse_args();
    let debug = cli.debug;
    let cwd = cli.working_directory();
    let ctx = EditorContext::from_cli(&cli);

    if debug {
        // Clear previous debug log
        let _ = std::fs::write(DEBUG_LOG_PATH, "");
        debug_log(debug, &format!("CLI args: {:?}", cli));
        debug_log(debug, &format!("Context: {:?}", ctx));
        debug_log(debug, &format!("CWD: {:?}", cwd));
    }

    // Discover the opencode server
    let server = server::discover_server(&cwd, cli.port)
        .await
        .context("Failed to find opencode server")?;

    debug_log(
        debug,
        &format!("Server found: port={}, cwd={:?}", server.port, server.cwd),
    );

    let client = server::Client::new(server.port);

    match cli.command {
        Command::Ask { initial } => {
            run_ask(&client, &ctx, &initial, debug).await?;
        }
        Command::Select => {
            run_select(&client, &ctx, debug).await?;
        }
        Command::Prompt { text, submit } => {
            run_prompt(&client, &ctx, &text, submit, debug).await?;
        }
        Command::Status => {
            run_status(&server).await?;
        }
    }

    Ok(())
}

/// Run the ask (input) mode
async fn run_ask(
    client: &server::Client,
    ctx: &EditorContext,
    initial: &str,
    debug: bool,
) -> Result<()> {
    debug_log(debug, "run_ask: starting");
    let mut app = App::new(debug)?;

    // Build context hint
    let context_hint = ctx.format_this();

    // Run the TUI with context for placeholder display
    let result = app.run_ask(initial, context_hint.as_deref(), Some(ctx))?;
    debug_log(debug, &format!("run_ask: TUI result = {:?}", result));

    // Clean up terminal before any async operations
    app.restore()?;
    drop(app);

    match result {
        AppResult::Submit(input) => {
            // Expand context placeholders
            let expanded = ctx.expand(&input);
            debug_log(debug, &format!("run_ask: expanded = {}", expanded));

            // Send to opencode
            client.send_prompt(&expanded, false, true).await?;

            // Print confirmation (will be captured by Helix but that's ok)
            eprintln!("Sent: {}", truncate(&expanded, 50));
        }
        AppResult::Cancel => {
            debug_log(debug, "run_ask: cancelled");
            eprintln!("Cancelled");
        }
    }

    Ok(())
}

/// Run the select (menu) mode
async fn run_select(client: &server::Client, ctx: &EditorContext, debug: bool) -> Result<()> {
    debug_log(debug, "run_select: starting");

    // Fetch agents and commands from server
    let agents = client.get_agents().await.unwrap_or_default();
    let commands = client.get_commands().await.unwrap_or_default();
    debug_log(
        debug,
        &format!(
            "run_select: {} agents, {} commands",
            agents.len(),
            commands.len()
        ),
    );

    // Build menu items
    let mut items: Vec<SelectItem> = Vec::new();

    // Add prompts
    items.extend(config::prompts_to_select_items());

    // Add server commands
    items.extend(config::commands_to_select_items(&commands));

    // Add agents
    items.extend(config::agents_to_select_items(&agents));

    let mut app = App::new(debug)?;
    let result = app.run_select(&items)?;
    debug_log(debug, &format!("run_select: TUI result = {:?}", result));

    // Clean up terminal
    app.restore()?;
    drop(app);

    match result {
        AppResult::Submit(value) => {
            // Expand context placeholders
            let expanded = ctx.expand(&value);
            debug_log(debug, &format!("run_select: expanded = {}", expanded));

            // Send to opencode
            client.send_prompt(&expanded, false, true).await?;

            eprintln!("Sent: {}", truncate(&expanded, 50));
        }
        AppResult::Cancel => {
            debug_log(debug, "run_select: cancelled");
            eprintln!("Cancelled");
        }
    }

    Ok(())
}

/// Run the prompt command (non-interactive)
async fn run_prompt(
    client: &server::Client,
    ctx: &EditorContext,
    text: &str,
    submit: bool,
    debug: bool,
) -> Result<()> {
    debug_log(
        debug,
        &format!("run_prompt: text='{}', submit={}", text, submit),
    );

    // Check if text is a prompt name
    let prompt_text = config::get_prompt(text).map(|p| p.prompt).unwrap_or(text);

    // Expand context
    let expanded = ctx.expand(prompt_text);
    debug_log(debug, &format!("run_prompt: expanded = {}", expanded));

    // Send to opencode
    client.send_prompt(&expanded, false, submit).await?;

    eprintln!("Sent: {}", truncate(&expanded, 50));

    Ok(())
}

/// Show server status
async fn run_status(server: &server::Server) -> Result<()> {
    println!("opencode server:");
    println!("  Port: {}", server.port);
    println!("  CWD:  {}", server.cwd.display());
    if server.pid > 0 {
        println!("  PID:  {}", server.pid);
    }
    Ok(())
}

/// Truncate a string for display
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
