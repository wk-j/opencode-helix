//! Configuration and default prompts

use crate::tui::app::SelectItem;

/// A predefined prompt template
#[derive(Debug, Clone)]
pub struct Prompt {
    pub name: &'static str,
    pub prompt: &'static str,
    pub description: &'static str,
}

/// Default prompts matching opencode.nvim
pub const DEFAULT_PROMPTS: &[Prompt] = &[
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
    Prompt {
        name: "tests",
        prompt: "Write tests for this code: @this",
        description: "Generate tests",
    },
    Prompt {
        name: "docs",
        prompt: "Add documentation to this code: @this",
        description: "Add documentation",
    },
    Prompt {
        name: "refactor",
        prompt: "Refactor this code to be cleaner and more maintainable: @this",
        description: "Refactor code",
    },
    Prompt {
        name: "optimize",
        prompt: "Optimize this code for better performance: @this",
        description: "Optimize performance",
    },
];

/// Built-in commands
pub const BUILTIN_COMMANDS: &[(&str, &str)] = &[
    ("prompt.clear", "Clear the prompt input"),
    ("prompt.submit", "Submit the current prompt"),
    ("session.new", "Start a new session"),
    ("session.list", "List all sessions"),
    ("model.list", "List available models"),
];

/// Get prompt by name
pub fn get_prompt(name: &str) -> Option<&'static Prompt> {
    DEFAULT_PROMPTS.iter().find(|p| p.name == name)
}

/// Convert prompts to select items
pub fn prompts_to_select_items() -> Vec<SelectItem> {
    DEFAULT_PROMPTS
        .iter()
        .map(|p| SelectItem::new(p.name, p.description, p.prompt, "PROMPTS"))
        .collect()
}

/// Convert commands to select items
pub fn commands_to_select_items(commands: &[crate::server::client::Command]) -> Vec<SelectItem> {
    commands
        .iter()
        .map(|c| {
            SelectItem::new(
                &format!("/{}", c.name),
                &c.description,
                &c.template,
                "COMMANDS",
            )
        })
        .collect()
}

/// Convert agents to select items
pub fn agents_to_select_items(agents: &[crate::server::client::Agent]) -> Vec<SelectItem> {
    agents
        .iter()
        .filter(|a| a.mode == "subagent")
        .map(|a| {
            SelectItem::new(
                &format!("@{}", a.name),
                &a.description,
                &format!("@{} ", a.name),
                "AGENTS",
            )
        })
        .collect()
}
