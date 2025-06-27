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
    #[command(about = "Create a new worktree with an AI agent")]
    Create {
        #[arg(help = "Branch name for the worktree")]
        branch: String,
        #[arg(
            long,
            help = "Agent to use",
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
        #[arg(last = true, help = "Additional flags to pass to the agent")]
        agent_args: Vec<String>,
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