# Shell Completions Demo

This document demonstrates the shell completions feature for `jjz`.

## Quick Start

### Generate Bash Completions

```bash
$ jjz completions bash
# Output: Full bash completion script (stdout)
```

### Generate with Instructions

```bash
$ jjz completions bash --instructions
# Bash completion installation:
# Linux:
# jjz completions bash > ~/.local/share/bash-completion/completions/jjz
#
# macOS (with Homebrew):
# jjz completions bash > $(brew --prefix)/etc/bash_completion.d/jjz
#
# Or add to ~/.bashrc:
# source <(jjz completions bash)

Generating bash completions...

# [completion script output follows]
```

## All Supported Shells

### Bash

```bash
# Linux installation
jjz completions bash > ~/.local/share/bash-completion/completions/jjz

# macOS installation
jjz completions bash > $(brew --prefix)/etc/bash_completion.d/jjz

# Direct sourcing
echo 'source <(jjz completions bash)' >> ~/.bashrc
source ~/.bashrc
```

### Zsh

```bash
# Create directory
mkdir -p ~/.zsh/completions

# Generate completion
jjz completions zsh > ~/.zsh/completions/_jjz

# Add to ~/.zshrc
echo 'fpath=(~/.zsh/completions $fpath)' >> ~/.zshrc
echo 'autoload -Uz compinit && compinit' >> ~/.zshrc
source ~/.zshrc
```

### Fish

```bash
# Fish auto-loads from this directory
jjz completions fish > ~/.config/fish/completions/jjz.fish

# Completions active immediately in new shells
```

## Tab Completion Examples

Once installed, you can use tab completion:

```bash
# List all commands
$ jjz <TAB>
add          config       diff         init         query        sync
completions  dashboard    doctor       list         remove

# Complete subcommand options
$ jjz add --<TAB>
--help       --json       --no-hooks   --no-open    --template

# Complete shell names
$ jjz completions <TAB>
bash    fish    zsh

# Command help
$ jjz completions --<TAB>
--help         --instructions

# Specific shell completion
$ jjz add -<TAB>
-h (show help)
-t (template)

# Show template options
$ jjz add feature-auth --template <TAB>
minimal    standard    full
```

## Error Handling

### Invalid Shell

```bash
$ jjz completions powershell
Error: Unsupported shell: powershell
Supported shells: bash, zsh, fish
```

### Case Insensitive

```bash
$ jjz completions BASH    # Works!
$ jjz completions Zsh     # Works!
$ jjz completions FiSh    # Works!
```

## Advanced Usage

### Piping to Installation Script

```bash
# Install all shells at once
for shell in bash zsh fish; do
    case $shell in
        bash)
            jjz completions bash > ~/.local/share/bash-completion/completions/jjz
            ;;
        zsh)
            mkdir -p ~/.zsh/completions
            jjz completions zsh > ~/.zsh/completions/_jjz
            ;;
        fish)
            jjz completions fish > ~/.config/fish/completions/jjz.fish
            ;;
    esac
done
```

### Distribution Packaging

```bash
# Generate completions for packaging
mkdir -p dist/completions
jjz completions bash > dist/completions/jjz.bash
jjz completions zsh > dist/completions/_jjz
jjz completions fish > dist/completions/jjz.fish

# Package structure:
# dist/
#   bin/jjz
#   completions/
#     jjz.bash
#     _jjz
#     jjz.fish
```

### CI/CD Integration

```yaml
# GitHub Actions example
- name: Generate completions
  run: |
    mkdir -p artifacts/completions
    ./target/release/jjz completions bash > artifacts/completions/jjz.bash
    ./target/release/jjz completions zsh > artifacts/completions/_jjz
    ./target/release/jjz completions fish > artifacts/completions/jjz.fish

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
complete -p jjz

# Zsh
which _jjz

# Fish
complete -c jjz
```

### Manual Testing

```bash
# Create test session
jjz init
jjz add test-session

# Test completions
jjz <TAB>           # Should show commands
jjz add <TAB>       # Should show options
jjz list --<TAB>    # Should show flags
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
# Verify jjz is in PATH
which jjz

# Test completion generation
jjz completions bash | head -5

# Check shell-specific configs
echo $BASH_VERSION  # Bash
echo $ZSH_VERSION   # Zsh
echo $FISH_VERSION  # Fish
```

## References

- [Full Documentation](/home/lewis/src/zjj/docs/COMPLETIONS.md)
- [Implementation Details](/home/lewis/src/zjj/COMPLETIONS_IMPLEMENTATION.md)
- [clap_complete](https://docs.rs/clap_complete/)
