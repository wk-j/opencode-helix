//! HTTP client for opencode server API
//!
//! Communicates with the opencode server via HTTP/JSON.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// HTTP client for opencode server
#[derive(Debug, Clone)]
pub struct Client {
    port: u16,
    http: reqwest::Client,
}

/// Response from /path endpoint
#[derive(Debug, Deserialize)]
pub struct PathResponse {
    pub directory: Option<String>,
    pub worktree: Option<String>,
}

/// Agent information
#[derive(Debug, Clone, Deserialize)]
pub struct Agent {
    pub name: String,
    pub description: String,
    pub mode: String, // "primary" or "subagent"
}

/// Custom command from opencode
#[derive(Debug, Clone, Deserialize)]
pub struct Command {
    pub name: String,
    pub description: String,
    pub template: String,
    pub agent: Option<String>,
}

/// TUI publish request
#[derive(Debug, Serialize)]
struct TuiPublishRequest {
    #[serde(rename = "type")]
    event_type: String,
    properties: serde_json::Value,
}

impl Client {
    /// Create a new client for the given port
    pub fn new(port: u16) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        Self { port, http }
    }

    /// Base URL for the server
    fn base_url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    /// GET /path - Get server working directory
    pub async fn get_path(&self) -> Result<PathResponse> {
        let url = format!("{}/path", self.base_url());
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("Failed to connect to opencode server")?;

        response
            .json()
            .await
            .context("Failed to parse path response")
    }

    /// GET /agent - List available agents
    pub async fn get_agents(&self) -> Result<Vec<Agent>> {
        let url = format!("{}/agent", self.base_url());
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("Failed to fetch agents")?;

        response
            .json()
            .await
            .context("Failed to parse agents response")
    }

    /// GET /command - List custom commands
    pub async fn get_commands(&self) -> Result<Vec<Command>> {
        let url = format!("{}/command", self.base_url());
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("Failed to fetch commands")?;

        response
            .json()
            .await
            .context("Failed to parse commands response")
    }

    /// POST /tui/publish - Append text to prompt
    pub async fn tui_append_prompt(&self, text: &str) -> Result<()> {
        let url = format!("{}/tui/publish", self.base_url());
        let request = TuiPublishRequest {
            event_type: "tui.prompt.append".to_string(),
            properties: serde_json::json!({ "text": text }),
        };

        self.http
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to append prompt")?;

        Ok(())
    }

    /// POST /tui/publish - Execute a TUI command
    pub async fn tui_execute_command(&self, command: &str) -> Result<()> {
        let url = format!("{}/tui/publish", self.base_url());
        let request = TuiPublishRequest {
            event_type: "tui.command.execute".to_string(),
            properties: serde_json::json!({ "command": command }),
        };

        self.http
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to execute command")?;

        Ok(())
    }

    /// Clear the prompt input
    pub async fn clear_prompt(&self) -> Result<()> {
        self.tui_execute_command("prompt.clear").await
    }

    /// Submit the current prompt
    pub async fn submit_prompt(&self) -> Result<()> {
        self.tui_execute_command("prompt.submit").await
    }

    /// Send a prompt: optionally clear, append text, optionally submit
    pub async fn send_prompt(&self, text: &str, clear: bool, submit: bool) -> Result<()> {
        if clear {
            self.clear_prompt().await?;
        }

        self.tui_append_prompt(text).await?;

        if submit {
            self.submit_prompt().await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_url() {
        let client = Client::new(12345);
        assert_eq!(client.base_url(), "http://localhost:12345");
    }
}
