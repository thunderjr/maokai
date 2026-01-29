use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorktreeInfo {
    pub id: String,
    pub branch: String,
    pub path: PathBuf,
    pub project_name: String,
    pub agent: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub status: WorktreeStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WorktreeStatus {
    Active,
    Paused,
    Completed,
}

pub struct WorktreeManager {
    project_root: PathBuf,
    base_path: PathBuf,
}

impl WorktreeManager {
    pub fn new(project_root: PathBuf, base_path: PathBuf) -> Self {
        Self {
            project_root,
            base_path,
        }
    }

    pub fn is_git_repo(&self) -> bool {
        self.project_root.join(".git").exists()
    }

    pub fn list_all_worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        let mut all_worktrees = Vec::new();

        if !self.base_path.exists() {
            return Ok(all_worktrees);
        }

        for entry in std::fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                if let Ok(info) = self.load_worktree_info(&path) {
                    all_worktrees.push(info);
                }
            }
        }

        // Sort by creation time (newest first)
        all_worktrees.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(all_worktrees)
    }

    pub fn create_worktree(
        &self,
        branch: &str,
        agent: &str,
        base_branch: Option<&str>,
    ) -> Result<WorktreeInfo> {
        let project_name = self.get_project_name()?;
        let safe_branch_name = self.sanitize_branch_name(branch);
        let worktree_name = format!("{}-{}", project_name, safe_branch_name);
        self.create_worktree_at(&worktree_name, branch, agent, base_branch)
    }

    pub fn create_workspace_worktree(
        &self,
        branch: &str,
        base_branch: Option<&str>,
    ) -> Result<WorktreeInfo> {
        let project_name = self.get_project_name()?;
        self.create_worktree_at(&project_name, branch, "none", base_branch)
    }

    fn create_worktree_at(
        &self,
        worktree_name: &str,
        branch: &str,
        agent: &str,
        base_branch: Option<&str>,
    ) -> Result<WorktreeInfo> {
        let project_name = self.get_project_name()?;
        let worktree_path = self.base_path.join(worktree_name);
        std::fs::create_dir_all(&self.base_path)
            .context("Failed to create base worktree directory")?;

        let base = match base_branch {
            Some(base) => base.to_string(),
            _ => self.get_current_branch()?,
        };

        // Check if branch exists
        let branch_exists = self.branch_exists(branch)?;
        
        let mut args = vec!["worktree", "add"];
        
        if branch_exists {
            // If branch exists, just add the worktree without -b flag
            args.push(worktree_path.to_str().unwrap());
            args.push(branch);
        } else {
            // If branch doesn't exist, create it with -b flag
            args.push("-b");
            args.push(branch);
            args.push(worktree_path.to_str().unwrap());
            args.push(&base);
        }

        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.project_root)
            .output()
            .context("Failed to create git worktree")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to create worktree: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let worktree_info = WorktreeInfo {
            id: Uuid::new_v4().to_string(),
            branch: branch.to_string(),
            path: worktree_path,
            project_name,
            agent: agent.to_string(),
            created_at: chrono::Utc::now(),
            status: WorktreeStatus::Active,
        };

        self.save_worktree_info(&worktree_info)?;
        self.copy_env_files(&worktree_info.path)?;
        Ok(worktree_info)
    }

    fn copy_env_files(&self, worktree_path: &Path) -> Result<()> {
        for entry in std::fs::read_dir(&self.project_root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with(".env") {
                        let dest = worktree_path.join(name);
                        std::fs::copy(&path, &dest)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn list_worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        let output = Command::new("git")
            .args(["worktree", "list", "--porcelain"])
            .current_dir(&self.project_root)
            .output()
            .context("Failed to list git worktrees")?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut worktrees = Vec::new();

        for chunk in output_str.split("\n\n") {
            if chunk.trim().is_empty() {
                continue;
            }

            let lines: Vec<&str> = chunk.lines().collect();
            let mut worktree_path = None;
            let mut branch = None;

            for line in lines {
                if line.starts_with("worktree ") {
                    worktree_path = Some(line.strip_prefix("worktree ").unwrap());
                } else if line.starts_with("branch ") {
                    let branch_full = line.strip_prefix("branch ").unwrap();
                    if branch_full.starts_with("refs/heads/") {
                        branch = Some(branch_full.strip_prefix("refs/heads/").unwrap());
                    }
                }
            }

            if let (Some(path), Some(_br)) = (worktree_path, branch) {
                if let Ok(info) = self.load_worktree_info(Path::new(path)) {
                    worktrees.push(info);
                }
            }
        }

        Ok(worktrees)
    }

    pub fn remove_worktree(&self, branch: &str) -> Result<()> {
        self.remove_worktree_with_options(branch, false)
    }

    pub fn remove_worktree_force(&self, branch: &str) -> Result<()> {
        self.remove_worktree_with_options(branch, true)
    }

    fn remove_worktree_with_options(&self, branch: &str, force: bool) -> Result<()> {
        // Find the worktree by branch name from existing worktrees
        let worktrees = if self.is_git_repo() {
            self.list_worktrees()?
        } else {
            self.list_all_worktrees()?
        };

        let worktree_info = worktrees
            .iter()
            .find(|wt| wt.branch == branch)
            .ok_or_else(|| anyhow::anyhow!("Worktree for branch '{}' not found", branch))?;

        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        args.push(worktree_info.path.to_str().unwrap());

        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.project_root)
            .output()
            .context("Failed to remove git worktree")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to remove worktree: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let _ = Command::new("git")
            .args(["branch", "-D", branch])
            .current_dir(&self.project_root)
            .output();

        self.remove_worktree_info(&worktree_info.path)?;
        Ok(())
    }

    pub fn remove_worktree_at_path(&self, path: &Path, branch: &str, force: bool) -> Result<()> {
        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        args.push(path.to_str().unwrap());

        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.project_root)
            .output()
            .context("Failed to remove git worktree")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to remove worktree: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let _ = Command::new("git")
            .args(["branch", "-D", branch])
            .current_dir(&self.project_root)
            .output();

        self.remove_worktree_info(path)?;
        Ok(())
    }

    fn branch_exists(&self, branch: &str) -> Result<bool> {
        let output = Command::new("git")
            .args(["show-ref", "--verify", "--quiet", &format!("refs/heads/{}", branch)])
            .current_dir(&self.project_root)
            .output()
            .context("Failed to check if branch exists")?;

        Ok(output.status.success())
    }

    fn get_current_branch(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(&self.project_root)
            .output()
            .context("Failed to get current branch")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to get current branch: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if branch.is_empty() {
            anyhow::bail!("No current branch found (detached HEAD?)");
        }

        Ok(branch)
    }

    fn get_project_name(&self) -> Result<String> {
        if let Some(name) = self.project_root.file_name() {
            Ok(name.to_string_lossy().to_string())
        } else {
            Ok("project".to_string())
        }
    }

    fn save_worktree_info(&self, info: &WorktreeInfo) -> Result<()> {
        let info_path = info.path.join(".maokai-info.json");
        let content =
            serde_json::to_string_pretty(info).context("Failed to serialize worktree info")?;
        std::fs::write(&info_path, content).context("Failed to write worktree info")?;
        Ok(())
    }

    fn load_worktree_info(&self, worktree_path: &Path) -> Result<WorktreeInfo> {
        let info_path = worktree_path.join(".maokai-info.json");
        let content =
            std::fs::read_to_string(&info_path).context("Failed to read worktree info")?;
        serde_json::from_str(&content).context("Failed to parse worktree info")
    }

    fn remove_worktree_info(&self, worktree_path: &Path) -> Result<()> {
        let info_path = worktree_path.join(".maokai-info.json");
        if info_path.exists() {
            std::fs::remove_file(&info_path).context("Failed to remove worktree info")?;
        }
        Ok(())
    }

    fn sanitize_branch_name(&self, branch: &str) -> String {
        branch
            .replace('/', "-")
            .replace('\\', "-")
            .replace(':', "-")
            .replace('*', "-")
            .replace('?', "-")
            .replace('"', "-")
            .replace('<', "-")
            .replace('>', "-")
            .replace('|', "-")
            .replace(' ', "-")
    }

    pub fn get_worktree_path(&self, branch: &str) -> PathBuf {
        let project_name = self
            .get_project_name()
            .unwrap_or_else(|_| "project".to_string());
        let safe_branch_name = self.sanitize_branch_name(branch);
        let worktree_name = format!("{}-{}", project_name, safe_branch_name);
        self.base_path.join(&worktree_name)
    }
}
