#!/bin/bash

# Maokai shell integration for automatic directory change
# Add this to your ~/.bashrc, ~/.zshrc, or equivalent shell profile:
#   source /path/to/maokai-cd.sh

maokai() {
    local output
    local exit_code
    
    # Check if this is a create command
    if [[ "$1" == "create" ]]; then
        # Capture output and exit code
        output=$(command maokai "$@" 2>&1)
        exit_code=$?
        
        if [[ $exit_code -eq 0 ]]; then
            # Extract the first line which should be the path
            local path=$(echo "$output" | head -n1)
            
            # Change to the directory if it exists
            if [[ -d "$path" ]]; then
                echo "Created worktree at: $path" >&2
                builtin cd "$path"
            else
                echo "$output"
            fi
        else
            echo "$output" >&2
            return $exit_code
        fi
    else
        # For non-create commands, just pass through
        command maokai "$@"
    fi
}

# Zsh compatibility
if [[ -n "${ZSH_VERSION:-}" ]]; then
    # Enable bash-style array handling for zsh
    setopt KSH_ARRAYS 2>/dev/null || true
fi