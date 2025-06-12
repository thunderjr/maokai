use anyhow::{Context, Result};
use std::path::PathBuf;

pub struct PromptManager {
    prompts_dir: PathBuf,
}

impl PromptManager {
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().context("Failed to get home directory")?;
        let prompts_dir = home.join("maokai-prompts");

        std::fs::create_dir_all(&prompts_dir).context("Failed to create prompts directory")?;

        Ok(Self { prompts_dir })
    }

    pub fn get_prompt_path(&self, prompt_name: &str) -> PathBuf {
        let filename = if prompt_name.ends_with(".md") {
            prompt_name.to_string()
        } else {
            format!("{}.md", prompt_name)
        };
        self.prompts_dir.join(filename)
    }

    pub fn load_prompt(&self, prompt_name: &str) -> Result<String> {
        let prompt_path = self.get_prompt_path(prompt_name);

        if !prompt_path.exists() {
            anyhow::bail!(
                "Prompt file '{}' not found at {}",
                prompt_name,
                prompt_path.display()
            );
        }

        std::fs::read_to_string(&prompt_path)
            .with_context(|| format!("Failed to read prompt file: {}", prompt_path.display()))
    }

    pub fn list_prompts(&self) -> Result<Vec<String>> {
        let mut prompts = Vec::new();

        if !self.prompts_dir.exists() {
            return Ok(prompts);
        }

        let entries =
            std::fs::read_dir(&self.prompts_dir).context("Failed to read prompts directory")?;

        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                if let Some(stem) = path.file_stem() {
                    if let Some(name) = stem.to_str() {
                        prompts.push(name.to_string());
                    }
                }
            }
        }

        prompts.sort();
        Ok(prompts)
    }

    pub fn prompts_dir(&self) -> &PathBuf {
        &self.prompts_dir
    }
}
