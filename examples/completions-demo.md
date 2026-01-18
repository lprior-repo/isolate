# Shell Completions Demo

This document demonstrates the shell completions feature for `zjj`.

## Quick Start

### Generate Bash Completions

```bash
$ zjj completions bash
# Output: Full bash completion script (stdout)
```

### Generate with Instructions

```bash
$ zjj completions bash --instructions
# Bash completion installation:
# Linux:
# zjj completions bash > ~/.local/share/bash-completion/completions/zjj
#
# macOS (with Homebrew):
# zjj completions bash > $(brew --prefix)/etc/bash_completion.d/zjj
#
# Or add to ~/.bashrc:
# source <(zjj completions bash)

Generating bash completions...

# [completion script output follows]
```

## All Supported Shells

### Bash

```bash
# Linux installation
zjj completions bash > ~/.local/share/bash-completion/completions/zjj

# macOS installation
zjj completions bash > $(brew --prefix)/etc/bash_completion.d/zjj

# Direct sourcing
echo 'source <(zjj completions bash)' >> ~/.bashrc
source ~/.bashrc
```

### Zsh

```bash
# Create directory
mkdir -p ~/.zsh/completions

# Generate completion
zjj completions zsh > ~/.zsh/completions/_zjj

# Add to ~/.zshrc
echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc
echo 'autoload -Uz compinit && compinit' >> ~/.zshrc
source ~/.zshrc
```

### Fish

```bash
# Fish auto-loads from this directory
zjj completions fish > ~/.config/fish/completions/zjj.fish

# Completions active immediately in new shells
```

## Tab Completion Examples

Once installed, you can use tab completion:

```bash
# List all commands
$ zjj <TAB>
add          config       diff         init         query        sync
completions  dashboard    doctor       list         remove

# Complete subcommand options
$ zjj add --<TAB>
--help       --json       --no-hooks   --no-open    --template

# Complete shell names
$ zjj completions <TAB>
bash    fish    zsh

# Command help
$ zjj completions --<TAB>
--help         --instructions

# Specific shell completion
$ zjj add -<TAB>
-h (show help)
-t (template)

# Show template options
$ zjj add feature-auth --template <TAB>
minimal    standard    full
```

## Error Handling

### Invalid Shell

```bash
$ zjj completions powershell
Error: Unsupported shell: powershell
Supported shells: bash, zsh, fish
```

### Case Insensitive

```bash
$ zjj completions BASH    # Works!
$ zjj completions Zsh     # Works!
$ zjj completions FiSh    # Works!
```

## Advanced Usage

### Piping to Installation Script

```bash
# Install all shells at once
for shell in bash zsh fish; do
    case $shell in
        bash)
            zjj completions bash > ~/.local/share/bash-completion/completions/zjj
            ;;
        zsh)
            mkdir -p ~/.zsh/completions
            zjj completions zsh > ~/.zsh/completions/_zjj
            ;;
        fish)
            zjj completions fish > ~/.config/fish/completions/zjj.fish
            ;;
    esac
done
```

### Distribution Packaging

```bash
# Generate completions for packaging
mkdir -p dist/completions
zjj completions bash > dist/completions/zjj.bash
zjj completions zsh > dist/completions/_zjj
zjj completions fish > dist/completions/zjj.fish

# Package structure:
# dist/
#   bin/zjj
#   completions/
#     zjj.bash
#     _zjj
#     zjj.fish
```

### CI/CD Integration

```yaml
# GitHub Actions example
- name: Generate completions
  run: |
    mkdir -p artifacts/completions
    ./target/release/zjj completions bash > artifacts/completions/zjj.bash
    ./target/release/zjj completions zsh > artifacts/completions/_zjj
    ./target/release/zjj completions fish > artifacts/completions/zjj.fish

- name: Upload completions
  uses: actions/upload-artifact@v3
  with:
    name: shell-completions
    path: artifacts/completions/
```

## Testing

### Verify Installation

```bash
# Bash
complete -p zjj

# Zsh
which _zjj

# Fish
complete -c zjj
```

### Manual Testing

```bash
# Create test session
zjj init
zjj add test-session

# Test completions
zjj <TAB>           # Should show commands
zjj add <TAB>       # Should show options
zjj list --<TAB>    # Should show flags
```

## Completion Coverage

The completions support:

- ✅ All commands (add, list, remove, focus, etc.)
- ✅ All subcommands
- ✅ All options and flags
- ✅ Short flags (-f, -t, -i, etc.)
- ✅ Long flags (--force, --template, --instructions, etc.)
- ✅ Help text for each option
- ✅ Multi-word argument names
- ✅ Command aliases (dashboard/dash, doctor/check, config/cfg)

## Troubleshooting

See [COMPLETIONS.md](/home/lewis/src/zjj/docs/COMPLETIONS.md) for detailed troubleshooting steps.

### Quick Checks

```bash
# Verify zjj is in PATH
which zjj

# Test completion generation
zjj completions bash | head -5

# Check shell-specific configs
echo $BASH_VERSION  # Bash
echo $ZSH_VERSION   # Zsh
echo $FISH_VERSION  # Fish
```

## References

- [Full Documentation](/home/lewis/src/zjj/docs/COMPLETIONS.md)
- [Implementation Details](/home/lewis/src/zjj/COMPLETIONS_IMPLEMENTATION.md)
- [clap_complete](https://docs.rs/clap_complete/)
