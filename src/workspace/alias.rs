use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::config::alias_dir;

use super::editor::open_in_editor;

#[derive(Debug, Serialize, Deserialize)]
pub struct AliasConfig {
    pub name: String,
    pub projects: Vec<PathBuf>,
}

pub struct AliasManager;

impl AliasManager {
    pub fn new() -> Self {
        Self
    }

    pub fn create(&self, alias_name: &str) -> Result<()> {
        let alias_path = alias_dir().join(format!("{}.yml", alias_name));
        std::fs::create_dir_all(alias_dir())?;

        let template = format!(
            r#"# Maokai Workspace Alias
# Add the full paths to the git repositories for this alias.

name: {}
projects:
#  - /path/to/your/first/project
#  - /path/to/your/second/project
"#,
            alias_name
        );

        std::fs::write(&alias_path, &template)?;
        open_in_editor(&alias_path)?;

        match self.validate_alias_file(&alias_path) {
            Ok(_) => {
                eprintln!("Alias '{}' created successfully.", alias_name);
                Ok(())
            }
            Err(e) => {
                std::fs::remove_file(&alias_path)?;
                Err(e)
            }
        }
    }

    pub fn load(&self, alias_name: &str) -> Result<AliasConfig> {
        let alias_path = alias_dir().join(format!("{}.yml", alias_name));
        let content = std::fs::read_to_string(&alias_path)
            .with_context(|| format!("Failed to read alias '{}'", alias_name))?;
        let config: AliasConfig = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse alias '{}'", alias_name))?;

        self.validate_projects(&config.projects)?;
        Ok(config)
    }

    pub fn remove(&self, alias_name: &str) -> Result<()> {
        let alias_path = alias_dir().join(format!("{}.yml", alias_name));
        if !alias_path.exists() {
            anyhow::bail!("Alias '{}' not found", alias_name);
        }
        std::fs::remove_file(&alias_path)?;
        eprintln!("Alias '{}' removed.", alias_name);
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<String>> {
        let dir = alias_dir();
        if !dir.exists() {
            return Ok(vec![]);
        }

        let mut aliases = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "yml").unwrap_or(false) {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    aliases.push(stem.to_string());
                }
            }
        }
        aliases.sort();
        Ok(aliases)
    }

    fn validate_alias_file(&self, path: &PathBuf) -> Result<()> {
        let content = std::fs::read_to_string(path).context("Failed to read alias file")?;
        let config: AliasConfig =
            serde_yaml::from_str(&content).context("Failed to parse alias file")?;

        if config.projects.is_empty() {
            anyhow::bail!("Alias must have at least one project");
        }

        self.validate_projects(&config.projects)
    }

    fn validate_projects(&self, projects: &[PathBuf]) -> Result<()> {
        for project in projects {
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
        Ok(())
    }
}
