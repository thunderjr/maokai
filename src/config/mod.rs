use std::path::PathBuf;

pub fn base_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".maokai")
}

pub fn workspaces_dir() -> PathBuf {
    base_dir().join("workspaces")
}

pub fn alias_dir() -> PathBuf {
    base_dir().join("alias")
}

pub fn get_worktree_base_path() -> PathBuf {
    if let Ok(path) = std::env::var("MAOKAI_WORKTREE_PATH") {
        PathBuf::from(path)
    } else {
        base_dir().join("worktrees")
    }
}
