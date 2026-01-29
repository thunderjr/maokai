use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

use crate::config::worktrees_registry_path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorktreeInfo {
    pub id: String,
    pub branch: String,
    pub path: PathBuf,
    pub project_root: PathBuf,
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

#[derive(Debug, Serialize, Deserialize, Default)]
struct WorktreeRegistry {
    worktrees: Vec<WorktreeInfo>,
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

    /// List all worktrees from the central registry.
    /// Optionally filters by project_root matching the current manager's project_root.
    pub fn list_all_worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        let mut all_worktrees = load_registry()?;

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
            project_root: self.project_root.clone(),
            project_name,
            agent: agent.to_string(),
            created_at: chrono::Utc::now(),
            status: WorktreeStatus::Active,
        };

        add_to_registry(&worktree_info)?;
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

    /// List worktrees for this project by cross-referencing git worktree list with the registry.
    /// Returns the intersection (validates worktrees still exist in git).
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
        let mut git_worktree_paths: Vec<PathBuf> = Vec::new();

        for chunk in output_str.split("\n\n") {
            if chunk.trim().is_empty() {
                continue;
            }

            for line in chunk.lines() {
                if line.starts_with("worktree ") {
                    if let Some(path) = line.strip_prefix("worktree ") {
                        git_worktree_paths.push(PathBuf::from(path));
                    }
                }
            }
        }

        // Load registry and filter to worktrees that exist in git and match this project
        let registry = load_registry()?;
        let worktrees: Vec<WorktreeInfo> = registry
            .into_iter()
            .filter(|info| {
                info.project_root == self.project_root
                    && git_worktree_paths.contains(&info.path)
            })
            .collect();

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

        remove_from_registry(&worktree_info.path)?;
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

        remove_from_registry(path)?;
        Ok(())
    }

    fn branch_exists(&self, branch: &str) -> Result<bool> {
        let output = Command::new("git")
            .args([
                "show-ref",
                "--verify",
                "--quiet",
                &format!("refs/heads/{}", branch),
            ])
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

// Registry functions

fn load_registry() -> Result<Vec<WorktreeInfo>> {
    let registry_path = worktrees_registry_path();

    if !registry_path.exists() {
        // Attempt migration from old .maokai-info.json files
        return migrate_old_worktree_info();
    }

    let content = std::fs::read_to_string(&registry_path)
        .context("Failed to read worktrees registry")?;
    let registry: WorktreeRegistry =
        serde_json::from_str(&content).context("Failed to parse worktrees registry")?;

    Ok(registry.worktrees)
}

fn save_registry(worktrees: &[WorktreeInfo]) -> Result<()> {
    let registry_path = worktrees_registry_path();

    // Ensure parent directory exists
    if let Some(parent) = registry_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let registry = WorktreeRegistry {
        worktrees: worktrees.to_vec(),
    };
    let content =
        serde_json::to_string_pretty(&registry).context("Failed to serialize worktrees registry")?;
    std::fs::write(&registry_path, content).context("Failed to write worktrees registry")?;
    Ok(())
}

fn add_to_registry(info: &WorktreeInfo) -> Result<()> {
    let mut worktrees = load_registry().unwrap_or_default();
    worktrees.push(info.clone());
    save_registry(&worktrees)
}

fn remove_from_registry(path: &Path) -> Result<()> {
    let mut worktrees = load_registry().unwrap_or_default();
    worktrees.retain(|wt| wt.path != path);
    save_registry(&worktrees)
}

/// Migrate old .maokai-info.json files from worktrees to the central registry.
fn migrate_old_worktree_info() -> Result<Vec<WorktreeInfo>> {
    use crate::config::get_worktree_base_path;

    let base_path = get_worktree_base_path();
    let mut migrated = Vec::new();

    if !base_path.exists() {
        return Ok(migrated);
    }

    // Scan for .maokai-info.json files in worktree directories
    if let Ok(entries) = std::fs::read_dir(&base_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let info_path = path.join(".maokai-info.json");
                if info_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&info_path) {
                        // Try to parse as old format (without project_root)
                        #[derive(Deserialize)]
                        struct OldWorktreeInfo {
                            id: String,
                            branch: String,
                            path: PathBuf,
                            project_name: String,
                            agent: String,
                            created_at: chrono::DateTime<chrono::Utc>,
                            status: WorktreeStatus,
                        }

                        if let Ok(old_info) = serde_json::from_str::<OldWorktreeInfo>(&content) {
                            // Convert to new format with empty project_root (we don't know it)
                            let new_info = WorktreeInfo {
                                id: old_info.id,
                                branch: old_info.branch,
                                path: old_info.path,
                                project_root: PathBuf::new(), // Unknown for migrated entries
                                project_name: old_info.project_name,
                                agent: old_info.agent,
                                created_at: old_info.created_at,
                                status: old_info.status,
                            };
                            migrated.push(new_info);

                            // Delete the old .maokai-info.json file
                            let _ = std::fs::remove_file(&info_path);
                        }
                    }
                }
            }
        }
    }

    // Also check workspaces directory for old .maokai-info.json files
    let workspaces_dir = crate::config::workspaces_dir();
    if workspaces_dir.exists() {
        if let Ok(workspace_entries) = std::fs::read_dir(&workspaces_dir) {
            for workspace_entry in workspace_entries.flatten() {
                let workspace_path = workspace_entry.path();
                if workspace_path.is_dir() {
                    // Check subdirectories within each workspace
                    if let Ok(project_entries) = std::fs::read_dir(&workspace_path) {
                        for project_entry in project_entries.flatten() {
                            let project_path = project_entry.path();
                            if project_path.is_dir() {
                                let info_path = project_path.join(".maokai-info.json");
                                if info_path.exists() {
                                    if let Ok(content) = std::fs::read_to_string(&info_path) {
                                        #[derive(Deserialize)]
                                        struct OldWorktreeInfo {
                                            id: String,
                                            branch: String,
                                            path: PathBuf,
                                            project_name: String,
                                            agent: String,
                                            created_at: chrono::DateTime<chrono::Utc>,
                                            status: WorktreeStatus,
                                        }

                                        if let Ok(old_info) =
                                            serde_json::from_str::<OldWorktreeInfo>(&content)
                                        {
                                            let new_info = WorktreeInfo {
                                                id: old_info.id,
                                                branch: old_info.branch,
                                                path: old_info.path,
                                                project_root: PathBuf::new(),
                                                project_name: old_info.project_name,
                                                agent: old_info.agent,
                                                created_at: old_info.created_at,
                                                status: old_info.status,
                                            };
                                            migrated.push(new_info);

                                            let _ = std::fs::remove_file(&info_path);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Save migrated entries to the new registry if any were found
    if !migrated.is_empty() {
        save_registry(&migrated)?;
    }

    Ok(migrated)
}
