//! Server discovery for opencode processes
//!
//! Finds running opencode servers by scanning processes and validating via HTTP.

use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use sysinfo::System;

/// A discovered opencode server
#[derive(Debug, Clone)]
pub struct Server {
    /// Process ID
    pub pid: u32,
    /// HTTP server port
    pub port: u16,
    /// Working directory of the server
    pub cwd: PathBuf,
}

/// Find opencode processes listening on ports
fn find_opencode_processes() -> Result<Vec<(u32, String)>> {
    let system = System::new_all();

    let mut processes = Vec::new();

    for (pid, process) in system.processes() {
        let cmd = process.cmd();
        let cmd_str: String = cmd
            .iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(" ");

        // Look for opencode processes with --port flag
        if cmd_str.contains("opencode") && cmd_str.contains("--port") {
            processes.push((pid.as_u32(), cmd_str));
        }
    }

    Ok(processes)
}

/// Extract port number from command line arguments
fn extract_port_from_cmdline(cmdline: &str) -> Option<u16> {
    // Look for --port followed by a number
    let parts: Vec<&str> = cmdline.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == "--port" {
            if let Some(port_str) = parts.get(i + 1) {
                if let Ok(port) = port_str.parse() {
                    return Some(port);
                }
            }
        } else if part.starts_with("--port=") {
            if let Some(port_str) = part.strip_prefix("--port=") {
                if let Ok(port) = port_str.parse() {
                    return Some(port);
                }
            }
        }
    }
    None
}

/// Validate a port is an opencode server and get its working directory
async fn validate_server(port: u16) -> Result<Server> {
    let client = super::client::Client::new(port);
    let path_response = client
        .get_path()
        .await
        .context("Failed to connect to opencode server")?;

    let cwd = path_response
        .directory
        .or(path_response.worktree)
        .ok_or_else(|| anyhow!("Server did not return a working directory"))?;

    Ok(Server {
        pid: 0, // We don't track PID after validation
        port,
        cwd: PathBuf::from(cwd),
    })
}

/// Discover an opencode server for the given working directory
///
/// If `port` is specified, validates and uses that port directly.
/// Otherwise, scans for opencode processes and finds one matching the cwd.
pub async fn discover_server(cwd: &Path, port: Option<u16>) -> Result<Server> {
    // If port is specified, use it directly
    if let Some(p) = port {
        return validate_server(p)
            .await
            .context(format!("No opencode server responding on port {}", p));
    }

    // Find all opencode processes
    let processes = find_opencode_processes()?;
    if processes.is_empty() {
        return Err(anyhow!(
            "No opencode processes found. Start opencode first with: opencode"
        ));
    }

    // Try each process to find one matching our cwd
    let mut last_error = None;
    for (pid, cmdline) in processes {
        if let Some(port) = extract_port_from_cmdline(&cmdline) {
            match validate_server(port).await {
                Ok(mut server) => {
                    server.pid = pid;

                    // Check if server's cwd matches or contains our cwd
                    let server_cwd = server.cwd.canonicalize().unwrap_or(server.cwd.clone());
                    let our_cwd = cwd.canonicalize().unwrap_or(cwd.to_path_buf());

                    if our_cwd.starts_with(&server_cwd) || server_cwd.starts_with(&our_cwd) {
                        return Ok(server);
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| anyhow!("No opencode server found for directory: {}", cwd.display())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_port() {
        assert_eq!(
            extract_port_from_cmdline("opencode --port 12345"),
            Some(12345)
        );
        assert_eq!(
            extract_port_from_cmdline("node opencode.js --port 8080 --other"),
            Some(8080)
        );
        assert_eq!(
            extract_port_from_cmdline("opencode --port=9999"),
            Some(9999)
        );
        assert_eq!(extract_port_from_cmdline("opencode --other"), None);
    }
}
