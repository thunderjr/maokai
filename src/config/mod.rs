use std::path::PathBuf;

pub fn get_worktree_base_path() -> PathBuf {
    if let Ok(path) = std::env::var("MAOKAI_WORKTREE_PATH") {
        PathBuf::from(path)
    } else {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join("maokai-branches")
    }
}
