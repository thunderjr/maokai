use anyhow::Result;
use clap::Parser;
use std::env;

use maokai::agent::get_agent;
use maokai::cli::Commands;
use maokai::config::get_worktree_base_path;
use maokai::{Cli, WorktreeManager};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let project_root = env::current_dir()?;
    let worktree_base_path = get_worktree_base_path();
    let worktree_manager = WorktreeManager::new(project_root.clone(), worktree_base_path.clone());

    match cli.command {
        Some(Commands::Create {
            branch,
            agent,
            system_prompt,
            base_branch,
            agent_args,
        }) => {
            let worktree_info = worktree_manager.create_worktree(
                &branch,
                &agent.to_string(),
                base_branch.as_deref(),
            )?;
            println!(
                "Created worktree for branch '{}' at: {}",
                branch,
                worktree_info.path.display()
            );

            let agent_impl = get_agent(&agent.to_string())?;
            agent_impl.start(&worktree_info, system_prompt.as_deref(), &agent_args)?;
        }
        Some(Commands::Ls) => {
            let worktrees = if worktree_manager.is_git_repo() {
                // Inside a git repo - show project-specific worktrees
                worktree_manager.list_worktrees()?
            } else {
                // Outside git repo - show all worktrees from all projects
                worktree_manager.list_all_worktrees()?
            };

            if worktrees.is_empty() {
                eprintln!("No active worktrees found.");
                std::process::exit(1);
            }

            for wt in worktrees {
                println!("{} - {} ({})", wt.project_name, wt.branch, wt.agent);
            }
        }
        Some(Commands::Remove { branch }) => match branch {
            Some(branch_name) => {
                worktree_manager.remove_worktree(&branch_name)?;
                println!("Removed worktree for branch '{}'", branch_name);
            }
            _ => {
                let worktrees = if worktree_manager.is_git_repo() {
                    worktree_manager.list_worktrees()?
                } else {
                    worktree_manager.list_all_worktrees()?
                };

                if worktrees.is_empty() {
                    eprintln!("No active worktrees found to remove.");
                    std::process::exit(1);
                }

                eprintln!("Please specify a branch name to remove. Available worktrees:");
                for wt in worktrees {
                    eprintln!("  {}", wt.branch);
                }
                std::process::exit(1);
            }
        },
        Some(Commands::Status) => {
            let worktrees = worktree_manager.list_worktrees()?;
            println!("Worktree Status:");
            for wt in worktrees {
                println!("  Branch: {}", wt.branch);
                println!("    Path: {}", wt.path.display());
                println!("    Agent: {}", wt.agent);
                println!("    Status: {:?}", wt.status);
                println!(
                    "    Created: {}",
                    wt.created_at.format("%Y-%m-%d %H:%M:%S UTC")
                );
                println!();
            }
        }
        Some(Commands::Path { branch }) => {
            let worktrees = if worktree_manager.is_git_repo() {
                worktree_manager.list_worktrees()?
            } else {
                worktree_manager.list_all_worktrees()?
            };

            for wt in worktrees {
                if wt.branch == branch {
                    println!("{}", wt.path.display());
                    return Ok(());
                }
            }
            eprintln!("Worktree for branch '{}' not found", branch);
            std::process::exit(1);
        }
        _ => {
            // Default to listing worktrees
            let worktrees = if worktree_manager.is_git_repo() {
                // Inside a git repo - show project-specific worktrees
                worktree_manager.list_worktrees()?
            } else {
                // Outside git repo - show all worktrees from all projects
                worktree_manager.list_all_worktrees()?
            };

            if worktrees.is_empty() {
                eprintln!("No active worktrees found.");
                std::process::exit(1);
            }

            for wt in worktrees {
                println!("{} - {} ({})", wt.project_name, wt.branch, wt.agent);
            }
        }
    }

    Ok(())
}