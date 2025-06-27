use crate::prompt::PromptManager;
use crate::worktree::WorktreeInfo;
use anyhow::{Context, Result};
use std::process::{Command, Stdio};

pub fn start_claude_agent(
    worktree_info: &WorktreeInfo,
    system_prompt: Option<&str>,
    agent_args: &[String],
) -> Result<()> {
    println!("Starting Claude agent for branch: {}", worktree_info.branch);
    println!("Worktree path: {}", worktree_info.path.display());

    let mut cmd = Command::new("claude");

    // Add forwarded agent arguments
    cmd.args(agent_args);

    if let Some(prompt_name) = system_prompt {
        let prompt_manager = PromptManager::new()?;
        let prompt_content = prompt_manager
            .load_prompt(prompt_name)
            .with_context(|| format!("Failed to load system prompt: {}", prompt_name))?;

        println!("Using system prompt: {}", prompt_name);
        cmd.arg("--system-prompt").arg(prompt_content);
    }

    cmd.current_dir(&worktree_info.path);
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let status = cmd.status().context("Failed to start Claude agent")?;

    if !status.success() {
        anyhow::bail!("Claude agent exited with error");
    }

    Ok(())
}

pub fn start_gemini_agent(
    worktree_info: &WorktreeInfo,
    agent_args: &[String],
) -> Result<()> {
    println!("Starting Gemini agent for branch: {}", worktree_info.branch);
    println!("Worktree path: {}", worktree_info.path.display());

    let mut cmd = Command::new("gemini");

    // Add forwarded agent arguments
    cmd.args(agent_args);

    cmd.current_dir(&worktree_info.path);
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let status = cmd.status().context("Failed to start Gemini agent")?;

    if !status.success() {
        anyhow::bail!("Gemini agent exited with error");
    }

    Ok(())
}

