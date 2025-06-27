use crate::prompt::PromptManager;
use crate::worktree::WorktreeInfo;
use anyhow::{Context, Result};
use std::process::{Command, Stdio};

pub trait Agent {
    fn name(&self) -> &str;
    fn command(&self) -> &str;
    fn start(
        &self,
        worktree_info: &WorktreeInfo,
        system_prompt: Option<&str>,
        agent_args: &[String],
    ) -> Result<()>;
}

pub struct ClaudeAgent;

impl Agent for ClaudeAgent {
    fn name(&self) -> &str {
        "claude"
    }

    fn command(&self) -> &str {
        "claude"
    }

    fn start(
        &self,
        worktree_info: &WorktreeInfo,
        system_prompt: Option<&str>,
        agent_args: &[String],
    ) -> Result<()> {
        println!("Starting Claude agent for branch: {}", worktree_info.branch);
        println!("Worktree path: {}", worktree_info.path.display());

        let mut cmd = Command::new(self.command());

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
}

pub struct GeminiAgent;

impl Agent for GeminiAgent {
    fn name(&self) -> &str {
        "gemini"
    }

    fn command(&self) -> &str {
        "gemini"
    }

    fn start(
        &self,
        worktree_info: &WorktreeInfo,
        system_prompt: Option<&str>,
        agent_args: &[String],
    ) -> Result<()> {
        println!("Starting Gemini agent for branch: {}", worktree_info.branch);
        println!("Worktree path: {}", worktree_info.path.display());

        if system_prompt.is_some() {
            anyhow::bail!("Gemini agent does not support system prompts");
        }

        let mut cmd = Command::new(self.command());

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
}

pub fn get_agent(agent_type: &str) -> Result<Box<dyn Agent>> {
    match agent_type {
        "claude" => Ok(Box::new(ClaudeAgent)),
        "gemini" => Ok(Box::new(GeminiAgent)),
        _ => anyhow::bail!("Unknown agent type: {}", agent_type),
    }
}

