pub mod alias;
pub mod editor;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::config::{get_worktree_base_path, workspaces_dir};
use crate::WorktreeManager;

use self::alias::AliasManager;
use self::editor::open_in_editor;

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub name: String,
    pub safe_name: String,
    pub projects: Vec<PathBuf>,
    pub alias: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | ' ' => '-',
            _ => c,
        })
        .collect()
}

pub struct WorkspaceManager;

impl WorkspaceManager {
    pub fn new() -> Self {
        Self
    }

    pub fn create(&self, name: &str, alias_name: Option<&str>) -> Result<()> {
        let safe_name = sanitize_name(name);
        let workspace_path = workspaces_dir().join(format!("{}.json", safe_name));

        if workspace_path.exists() {
            anyhow::bail!("Workspace '{}' already exists", name);
        }

        let projects = match alias_name {
            Some(alias) => {
                let alias_manager = AliasManager::new();
                let config = alias_manager.load(alias)?;
                config.projects
            }
            None => self.get_projects_from_editor(&safe_name)?,
        };

        if projects.is_empty() {
            anyhow::bail!("No projects specified for workspace");
        }

        std::fs::create_dir_all(workspaces_dir())?;

        let worktree_base = get_worktree_base_path();
        let mut created_worktrees = Vec::new();

        for project in &projects {
            let manager = WorktreeManager::new(project.clone(), worktree_base.clone());
            match manager.create_worktree(name, "none", None) {
                Ok(info) => {
                    eprintln!(
                        "Created worktree for {} at {}",
                        project.display(),
                        info.path.display()
                    );
                    created_worktrees.push(project.clone());
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to create worktree for {}: {}",
                        project.display(),
                        e
                    );
                    // Continue with other projects
                }
            }
        }

        if created_worktrees.is_empty() {
            anyhow::bail!("Failed to create any worktrees");
        }

        let workspace_info = WorkspaceInfo {
            name: name.to_string(),
            safe_name: safe_name.clone(),
            projects: created_worktrees,
            alias: alias_name.map(String::from),
            created_at: Utc::now(),
        };

        let content = serde_json::to_string_pretty(&workspace_info)?;
        std::fs::write(&workspace_path, content)?;

        eprintln!("Workspace '{}' created.", name);
        Ok(())
    }

    pub fn remove(&self, name: &str) -> Result<()> {
        let safe_name = sanitize_name(name);
        let workspace_path = workspaces_dir().join(format!("{}.json", safe_name));

        if !workspace_path.exists() {
            anyhow::bail!("Workspace '{}' not found", name);
        }

        let content = std::fs::read_to_string(&workspace_path)?;
        let workspace_info: WorkspaceInfo = serde_json::from_str(&content)?;

        let worktree_base = get_worktree_base_path();
        let mut had_errors = false;

        for project in &workspace_info.projects {
            let manager = WorktreeManager::new(project.clone(), worktree_base.clone());
            match manager.remove_worktree(&workspace_info.name) {
                Ok(_) => {
                    eprintln!("Removed worktree for {}", project.display());
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to remove worktree for {}: {}",
                        project.display(),
                        e
                    );
                    had_errors = true;
                }
            }
        }

        std::fs::remove_file(&workspace_path)?;

        if had_errors {
            eprintln!(
                "Workspace '{}' removed with some errors (see warnings above).",
                name
            );
        } else {
            eprintln!("Workspace '{}' removed.", name);
        }

        Ok(())
    }

    pub fn list(&self) -> Result<Vec<WorkspaceInfo>> {
        let dir = workspaces_dir();
        if !dir.exists() {
            return Ok(vec![]);
        }

        let mut workspaces = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                let content = std::fs::read_to_string(&path)?;
                if let Ok(info) = serde_json::from_str::<WorkspaceInfo>(&content) {
                    workspaces.push(info);
                }
            }
        }

        workspaces.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(workspaces)
    }

    fn get_projects_from_editor(&self, safe_name: &str) -> Result<Vec<PathBuf>> {
        let temp_dir = tempfile::tempdir()?;
        let temp_file = temp_dir.path().join(format!("{}.yml", safe_name));

        let template = r#"# Maokai Workspace
# Add the full paths to the git repositories for this workspace.

projects:
#  - /path/to/your/first/project
#  - /path/to/your/second/project
"#;

        std::fs::write(&temp_file, template)?;
        open_in_editor(&temp_file)?;

        let content =
            std::fs::read_to_string(&temp_file).context("Failed to read workspace config")?;

        #[derive(Deserialize)]
        struct TempConfig {
            projects: Vec<PathBuf>,
        }

        let config: TempConfig =
            serde_yaml::from_str(&content).context("Failed to parse workspace config")?;

        // Validate projects
        for project in &config.projects {
            if !project.exists() {
                anyhow::bail!("Project path does not exist: {}", project.display());
            }
            let git_path = project.join(".git");
            if !git_path.exists() {
                anyhow::bail!(
                    "Project path is not a git repository: {}",
                    project.display()
                );
            }
        }

        Ok(config.projects)
    }
}
