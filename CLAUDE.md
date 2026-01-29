# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
cargo build              # Development build
cargo build --release    # Release build
cargo run -- <args>      # Run with arguments
cargo test               # Run tests
```

## Architecture

Maokai is a CLI tool for managing git worktrees with AI agent integration. It enables parallel development by creating isolated worktrees and optionally launching AI agents within them.

### Core Modules

- **cli** (`src/cli/mod.rs`): Clap-based CLI definitions. Commands: `create`, `ls`, `remove`, `status`, `path`, `workspace`
- **worktree** (`src/worktree/mod.rs`): Git worktree management via `WorktreeManager`. Handles creation, listing, and removal. Metadata stored centrally in `~/.maokai/worktrees.json`
- **agent** (`src/agent/mod.rs`): Agent trait and implementations (ClaudeAgent, GeminiAgent). Agents are spawned in worktree directories
- **workspace** (`src/workspace/mod.rs`): Multi-repo workspace management. Creates worktrees across multiple projects simultaneously
- **config** (`src/config/mod.rs`): Path helpers for `~/.maokai/` directory structure
- **prompt** (`src/prompt/mod.rs`): System prompt loading from `$HOME/maokai-prompts/`

### Data Flow

1. `main.rs` parses CLI args and delegates to appropriate handler
2. `WorktreeManager` creates git worktrees using `git worktree add` commands
3. Worktree metadata is written to central registry (`~/.maokai/worktrees.json`)
4. If agent specified, `get_agent()` returns appropriate `Agent` impl which spawns the CLI process

### Key Data Structures

- `WorktreeInfo`: Core metadata (id, branch, path, project_root, agent, status, timestamps)
- `WorktreeRegistry`: JSON wrapper for `Vec<WorktreeInfo>` stored in registry file
- `WorkspaceInfo`: Multi-repo workspace with list of project paths

### Directory Structure

```
~/.maokai/
├── worktrees.json     # Central worktree registry
├── worktrees/         # Worktree directories
├── workspaces/        # Workspace metadata and directories
└── alias/             # Workspace alias configs
```
