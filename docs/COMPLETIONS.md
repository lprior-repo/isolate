# Shell Completions for zjj

The `zjj` CLI supports shell completions for Bash, Zsh, and Fish shells. Completions provide tab-completion for commands, subcommands, and options.

## Generating Completions

Use the `zjj completions` command to generate completion scripts:

```bash
zjj completions <shell>
```

Supported shells:
- `bash`
- `zsh`
- `fish`

### With Installation Instructions

To see installation instructions along with the generated completions:

```bash
zjj completions <shell> --instructions
```

## Installation

### Bash

#### Linux

```bash
# Create completion directory if needed
mkdir -p ~/.local/share/bash-completion/completions

# Generate and install completions
zjj completions bash > ~/.local/share/bash-completion/completions/zjj
```

#### macOS (with Homebrew)

```bash
# Generate and install completions
zjj completions bash > $(brew --prefix)/etc/bash_completion.d/zjj
```

#### Alternative: Source in .bashrc

Add to your `~/.bashrc`:

```bash
source <(zjj completions bash)
```

Then reload your shell:

```bash
source ~/.bashrc
```

### Zsh

#### Create Completion Directory

```bash
# Create completions directory if needed
mkdir -p ~/.zsh/completions
```

#### Generate Completion File

```bash
zjj completions zsh > ~/.zsh/completions/_zjj
```

#### Configure Zsh

Add to your `~/.zshrc` (if not already present):

```zsh
fpath=(~/.zsh/completions $fpath)
autoload -Uz compinit && compinit
```

Then reload your shell:

```bash
source ~/.zshrc
```

### Fish

Fish automatically loads completions from `~/.config/fish/completions/`:

```bash
# Generate and install completions
zjj completions fish > ~/.config/fish/completions/zjj.fish
```

Completions will be available immediately in new Fish shells.

## Testing Completions

After installation, test completions by typing:

```bash
zjj <TAB>
```

You should see available subcommands. Try:

```bash
zjj add <TAB>           # Shows options for the add command
zjj config <TAB>        # Shows config options
zjj completions <TAB>   # Shows available shells
```

## Troubleshooting

### Bash

**Completions not working?**

1. Verify bash-completion is installed:
   ```bash
   # Ubuntu/Debian
   sudo apt install bash-completion

   # macOS with Homebrew
   brew install bash-completion@2
   ```

2. Check that bash-completion is sourced in your `~/.bashrc`:
   ```bash
   if [ -f /etc/bash_completion ]; then
       . /etc/bash_completion
   fi
   ```

3. Verify the completion file exists:
   ```bash
   ls -la ~/.local/share/bash-completion/completions/zjj
   ```

4. Reload your shell:
   ```bash
   source ~/.bashrc
   ```

### Zsh

**Completions not working?**

1. Verify the completion file exists:
   ```bash
   ls -la ~/.zsh/completions/_zjj
   ```

2. Check that `fpath` includes your completions directory:
   ```zsh
   echo $fpath
   ```

3. Ensure `compinit` is called after adding to `fpath` in your `~/.zshrc`

4. Rebuild the completion cache:
   ```zsh
   rm -f ~/.zcompdump
   compinit
   ```

5. Reload your shell:
   ```bash
   source ~/.zshrc
   ```

### Fish

**Completions not working?**

1. Verify the completion file exists:
   ```bash
   ls -la ~/.config/fish/completions/zjj.fish
   ```

2. Fish automatically loads completions, but you can force a reload:
   ```fish
   complete -e zjj
   source ~/.config/fish/completions/zjj.fish
   ```

3. Open a new Fish shell to ensure completions are loaded

## CI/CD Integration

For automated testing and distribution, you can generate completions as part of your build process:

```bash
# Generate all completions
zjj completions bash > completions/zjj.bash
zjj completions zsh > completions/_zjj
zjj completions fish > completions/zjj.fish
```

These can then be packaged with your distribution or installer.

## Development

The completions are generated using [clap_complete](https://docs.rs/clap_complete/), which automatically generates completions based on the CLI definition in `src/main.rs`.

To add new commands or options, simply update the CLI definition, and the completions will be automatically updated when regenerated.

## See Also

- [clap_complete documentation](https://docs.rs/clap_complete/)
- [Bash Completion Guide](https://github.com/scop/bash-completion)
- [Zsh Completion System](https://zsh.sourceforge.io/Doc/Release/Completion-System.html)
- [Fish Completions](https://fishshell.com/docs/current/completions.html)
