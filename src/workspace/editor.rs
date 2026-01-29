use anyhow::Result;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::process::{Command, Stdio};

pub fn get_editor() -> String {
    std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string())
}

pub fn is_vim_like(editor: &str) -> bool {
    let basename = Path::new(editor)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(editor);
    matches!(basename, "vim" | "nvim" | "vi")
}

pub fn open_in_editor(path: &Path) -> Result<()> {
    let editor = get_editor();
    let vim_like = is_vim_like(&editor);

    let status = Command::new(&editor)
        .arg(path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        anyhow::bail!("Editor exited with non-zero status");
    }

    if !vim_like {
        eprint!("Press Enter to continue...");
        io::stderr().flush()?;
        let stdin = io::stdin();
        let _ = stdin.lock().lines().next();
    }

    Ok(())
}
