# MaokAI

A Rust CLI tool for managing git worktrees with AI agents to enable parallel development workflows.

## Overview

Maokai simplifies the process of creating isolated git worktrees and launching AI agents (Claude Code as default) within them, allowing you to work on multiple features or experiments simultaneously without context switching between branches.

## Features

- **Git Worktree Management**: Create, list, and remove git worktrees with automatic branch creation
- **AI Agent Integration**: Launch Claude agents with optional system prompts in each worktree
- **Context-Aware Listing**: Shows project-specific worktrees when inside a git repo, all worktrees globally when outside
- **Safe Folder Naming**: Automatically sanitizes branch names for filesystem compatibility
- **Metadata Tracking**: Stores worktree information with timestamps and status
- **Shell Integration**: Designed to work with external UI tools like `gum`

## Installation

```bash
# Install from source
cargo install --path .

# Or build locally
cargo build --release
```

## Quick Start

```bash
# Create a new worktree and launch Claude
maokai create feature/auth

# Create with a system prompt
maokai create feature/auth --system-prompt my-prompt

# Create from a specific base branch
maokai create feature/auth --base-branch main

# List all worktrees
maokai

# Remove a worktree
maokai remove feature/auth

# Get path to a specific worktree
maokai path feature/auth
```

## Commands

### `create <branch> [options] [-- agent-args]`
Creates a new git branch and worktree, then launches Claude agent.

**Options:**
- `--system-prompt <name>`: Use system prompt from `$HOME/maokai-prompts/<name>.md`
- `--base-branch <branch>`: Create branch from specified base (defaults to current branch)

**Examples:**
```bash
maokai create feature/auth
maokai create feature/auth --system-prompt backend-dev -- code --verbose
maokai create hotfix/bug-123 --base-branch main
```

### `ls` or default
Lists worktrees with context-aware behavior:
- Inside git repo: Shows only current project's worktrees
- Outside git repo: Shows all worktrees from all projects

### `remove [branch]`
Removes a worktree and its associated branch.
- With branch name: Removes specific worktree
- Without arguments: Shows available worktrees to remove

### `status`
Shows detailed status of all worktrees including paths, agents, and creation times.

### `path <branch>`
Returns the filesystem path to the specified worktree.

## Configuration

Maokai uses environment variables for configuration:

- `MAOKAI_WORKTREE_PATH`: Base directory for worktrees (default: `$HOME/maokai-branches`)

## System Prompts

Store system prompts as markdown files in `$HOME/maokai-prompts/`:

```bash
mkdir -p ~/maokai-prompts
echo "You are a backend developer..." > ~/maokai-prompts/backend-dev.md
maokai create api/users --system-prompt backend-dev
```

## Shell Integration

Recommended shell function for interactive worktree switching using [gum](https://github.com/charmbracelet/gum):

```bash
mcd() {
    local selected=$(maokai | gum choose)
    if [ -n "$selected" ]; then
        local branch=$(echo "$selected" | sed 's/.* - \([^(]*\) .*/\1/' | xargs)
        local path=$(maokai path "$branch")
        if [ -n "$path" ]; then
            cd "$path"
        fi
    fi
}
```

**Note**: [gum](https://github.com/charmbracelet/gum) is an optional CLI tool for creating interactive terminal UIs. Install it with:
```bash
# macOS
brew install gum

# Linux
yay -S gum # using arch, btw

# Or download from releases: https://github.com/charmbracelet/gum/releases
```

## How It Works

1. **Worktree Creation**: Creates git branches without prefixes and worktrees in `$HOME/maokai-branches`
2. **Naming Convention**: Uses `${project-name}-${safe-branch-name}` format with character sanitization
3. **Metadata Storage**: Each worktree contains `.maokai-info.json` with tracking information
4. **Agent Integration**: Launches `claude` command with flag forwarding and optional system prompts
5. **Context Detection**: Automatically detects if you're inside a git repository for intelligent listing

## Directory Structure

```
$HOME/maokai-branches/
├── myproject-feature_auth/          # Worktree for feature/auth branch
│   ├── .maokai-info.json           # Metadata
│   └── ...                         # Project files
└── myproject-hotfix_bug-123/       # Worktree for hotfix/bug-123 branch
    ├── .maokai-info.json
    └── ...

$HOME/maokai-prompts/
├── backend-dev.md                  # System prompt for backend development
├── frontend.md                     # System prompt for frontend work
└── ...
```

## Requirements

- Rust
- Git
- Claude Code CLI (default agent)
