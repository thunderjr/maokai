use clap::{Parser, Subcommand, ValueEnum};
use std::string::ToString;

#[derive(Parser)]
#[command(name = "maokai")]
#[command(about = "Manage git worktrees with AI agents for parallel development")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Create a new worktree with optional custom command (use -- to separate)")]
    Create {
        #[arg(help = "Branch name for the worktree")]
        branch: String,
        #[arg(
            long,
            help = "Agent to use (ignored if custom command provided)",
            value_enum,
            default_value_t = Agents::Claude
        )]
        agent: Agents,
        #[arg(long, help = "Name of system prompt file in $HOME/maokai-prompts")]
        system_prompt: Option<String>,
        #[arg(
            long,
            help = "Base branch to create the new branch from (defaults to current branch)"
        )]
        base_branch: Option<String>,
        #[arg(last = true, help = "Custom command to run instead of agent (use -- to separate)")]
        custom_command: Vec<String>,
    },
    #[command(about = "List and select a worktree to switch to")]
    Ls,
    #[command(about = "Remove a worktree")]
    Remove {
        #[arg(help = "Branch name of the worktree to remove")]
        branch: Option<String>,
    },
    #[command(about = "Show status of all worktrees")]
    Status,
    #[command(about = "Get path for a specific worktree by branch name")]
    Path {
        #[arg(help = "Branch name of the worktree")]
        branch: String,
    },
    #[command(about = "Manage workspaces (groups of worktrees across multiple repos)")]
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommands,
    },
}

#[derive(Subcommand)]
pub enum WorkspaceCommands {
    #[command(about = "List all workspaces")]
    Ls,
    #[command(about = "Create a new workspace")]
    Create {
        #[arg(help = "Branch name for the workspace")]
        name: String,
        #[arg(long, help = "Alias to use for project list")]
        alias: Option<String>,
    },
    #[command(about = "Remove a workspace", alias = "rm")]
    Remove {
        #[arg(help = "Name of the workspace to remove")]
        name: String,
    },
    #[command(about = "Manage workspace aliases")]
    Alias {
        #[command(subcommand)]
        command: AliasCommands,
    },
}

#[derive(Subcommand)]
pub enum AliasCommands {
    #[command(about = "Create a new alias")]
    New {
        #[arg(help = "Name of the alias")]
        alias_name: String,
    },
    #[command(about = "Remove an alias")]
    Rm {
        #[arg(help = "Name of the alias to remove")]
        alias_name: String,
    },
    #[command(about = "List all aliases")]
    Ls,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum Agents {
    Claude,
    Gemini,
}

impl ToString for Agents {
    fn to_string(&self) -> String {
        match self {
            Agents::Claude => "claude".to_string(),
            Agents::Gemini => "gemini".to_string(),
        }
    }
}
