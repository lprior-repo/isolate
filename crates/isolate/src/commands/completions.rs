//! Completions command - Generate shell completions
//!
//! Generates shell completion scripts for bash, zsh, fish, etc.

use anyhow::Result;
use isolate_core::{OutputFormat, SchemaEnvelope};
use serde::{Deserialize, Serialize};

/// Options for the completions command
#[derive(Debug, Clone)]
pub struct CompletionsOptions {
    /// Shell type (bash, zsh, fish, powershell)
    pub shell: Shell,
    /// Output format (for JSON mode)
    pub format: OutputFormat,
}

/// Supported shells
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[allow(clippy::enum_variant_names)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

impl std::str::FromStr for Shell {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bash" => Ok(Self::Bash),
            "zsh" => Ok(Self::Zsh),
            "fish" => Ok(Self::Fish),
            "powershell" | "ps" | "pwsh" => Ok(Self::PowerShell),
            "elvish" => Ok(Self::Elvish),
            _ => {
                anyhow::bail!("Unknown shell: {s}. Supported: bash, zsh, fish, powershell, elvish")
            }
        }
    }
}

/// Completions response (for JSON mode)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionsResponse {
    /// Shell type
    pub shell: Shell,
    /// The completion script
    pub script: String,
    /// Installation instructions
    pub install_instructions: String,
}

/// Run the completions command
pub fn run(options: &CompletionsOptions) -> Result<()> {
    let script = generate_completions(options.shell);
    let install = get_install_instructions(options.shell);

    if options.format.is_json() {
        let response = CompletionsResponse {
            shell: options.shell,
            script,
            install_instructions: install,
        };
        let envelope = SchemaEnvelope::new("completions-response", "single", &response);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
    } else {
        println!("{script}");
    }

    Ok(())
}

fn generate_completions(shell: Shell) -> String {
    match shell {
        Shell::Bash => generate_bash_completions(),
        Shell::Zsh => generate_zsh_completions(),
        Shell::Fish => generate_fish_completions(),
        Shell::PowerShell => generate_powershell_completions(),
        Shell::Elvish => generate_elvish_completions(),
    }
}

fn generate_bash_completions() -> String {
    r#"# isolate bash completion
_isolate() {
    local cur prev commands
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    commands="init add list remove focus status sync done undo revert spawn work abort \
              agents ai checkpoint clean config context diff doctor introspect \
              query whereami whoami contract examples validate whatif claim yield events \
              batch completions export import rename pause resume clone"

    if [[ ${COMP_CWORD} -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "${commands}" -- "${cur}") )
        return 0
    fi

    # Session name completion for commands that take session names
    case "${prev}" in
        focus|remove|status|sync|diff|claim|yield|rename|pause|resume|clone)
            local sessions=$(isolate list --json 2>/dev/null | jq -r '.data[].name' 2>/dev/null)
            COMPREPLY=( $(compgen -W "${sessions}" -- "${cur}") )
            return 0
            ;;
        --template|-t)
            COMPREPLY=( $(compgen -W "minimal standard full" -- "${cur}") )
            return 0
            ;;
        --shell)
            COMPREPLY=( $(compgen -W "bash zsh fish powershell elvish" -- "${cur}") )
            return 0
            ;;
    esac

    # Flag completion
    case "${COMP_WORDS[1]}" in
        add|work)
            COMPREPLY=( $(compgen -W "--no-hooks --no-open --template --json --idempotent --dry-run" -- "${cur}") )
            ;;
        remove)
            COMPREPLY=( $(compgen -W "--force --merge --keep-branch --json --idempotent" -- "${cur}") )
            ;;
        done)
            COMPREPLY=( $(compgen -W "--message --keep-workspace --squash --dry-run --no-bead-update --json" -- "${cur}") )
            ;;
        list)
            COMPREPLY=( $(compgen -W "--all --json --bead --agent" -- "${cur}") )
            ;;
        *)
            COMPREPLY=( $(compgen -W "--json --help" -- "${cur}") )
            ;;
    esac
}

complete -F _isolate isolate
"#.to_string()
}

fn generate_zsh_completions() -> String {
    r#"#compdef isolate

_isolate() {
    local line state

    _arguments -C \
        '1: :->command' \
        '*::arg:->args'

    case $state in
        command)
            _values 'isolate commands' \
                'init[Initialize isolate in a JJ repository]' \
                'add[Create session for manual work]' \
                'list[List all sessions]' \
                'remove[Remove a session]' \
                'focus[Switch to session Zellij tab]' \
                'status[Show detailed session status]' \
                'sync[Sync workspace with main]' \
                'done[Complete work and merge]' \
                'undo[Revert last done operation]' \
                'revert[Revert specific session merge]' \
                'spawn[Create session for automated agent work]' \
                'work[Start working on a task]' \
                'abort[Abandon workspace without merging]' \
                'agents[List and manage agents]' \
                'ai[AI-first entry point]' \
                'checkpoint[Save and restore session state]' \
                'clean[Remove stale sessions]' \
                'config[View or modify configuration]' \
                'context[Show complete environment context]' \

                'diff[Show diff between session and main]' \
                'doctor[Run system health checks]' \
                'introspect[Discover isolate capabilities]' \
                'query[Query system state]' \
                'whereami[Quick location query]' \
                'whoami[Agent identity query]' \
                'contract[Show command contracts]' \
                'examples[Show usage examples]' \
                'validate[Pre-validate inputs]' \
                'whatif[Preview what a command would do]' \
                'claim[Claim a session lock]' \
                'yield[Release a session lock]' \
                'events[Show or stream events]' \
                'batch[Execute multiple commands]' \
                'completions[Generate shell completions]'
            ;;
        args)
            case $line[1] in
                focus|remove|status|sync|diff|claim|yield|rename|pause|resume|clone)
                    _isolate_sessions
                    ;;
                *)
                    _files
                    ;;
            esac
            ;;
    esac
}

_isolate_sessions() {
    local sessions
    sessions=(${(f)"$(isolate list --json 2>/dev/null | jq -r '.data[].name' 2>/dev/null)"})
    _describe 'sessions' sessions
}

_isolate "$@"
"#
    .to_string()
}

fn generate_fish_completions() -> String {
    r#"# isolate fish completion

# Disable file completion by default
complete -c isolate -f

# Commands
complete -c isolate -n "__fish_use_subcommand" -a init -d "Initialize isolate"
complete -c isolate -n "__fish_use_subcommand" -a add -d "Create session"
complete -c isolate -n "__fish_use_subcommand" -a list -d "List sessions"
complete -c isolate -n "__fish_use_subcommand" -a remove -d "Remove session"
complete -c isolate -n "__fish_use_subcommand" -a focus -d "Switch to session"
complete -c isolate -n "__fish_use_subcommand" -a status -d "Show status"
complete -c isolate -n "__fish_use_subcommand" -a sync -d "Sync with main"
complete -c isolate -n "__fish_use_subcommand" -a done -d "Complete and merge"
complete -c isolate -n "__fish_use_subcommand" -a undo -d "Revert last done"
complete -c isolate -n "__fish_use_subcommand" -a revert -d "Revert specific merge"
complete -c isolate -n "__fish_use_subcommand" -a spawn -d "Spawn agent"
complete -c isolate -n "__fish_use_subcommand" -a work -d "Start working"
complete -c isolate -n "__fish_use_subcommand" -a abort -d "Abandon workspace"
complete -c isolate -n "__fish_use_subcommand" -a agents -d "Manage agents"
complete -c isolate -n "__fish_use_subcommand" -a ai -d "AI entry point"
complete -c isolate -n "__fish_use_subcommand" -a checkpoint -d "Manage checkpoints"
complete -c isolate -n "__fish_use_subcommand" -a clean -d "Remove stale sessions"
complete -c isolate -n "__fish_use_subcommand" -a config -d "Manage config"
complete -c isolate -n "__fish_use_subcommand" -a context -d "Show context"
complete -c isolate -n "__fish_use_subcommand" -a diff -d "Show diff"
complete -c isolate -n "__fish_use_subcommand" -a doctor -d "Health checks"
complete -c isolate -n "__fish_use_subcommand" -a introspect -d "Discover capabilities"
complete -c isolate -n "__fish_use_subcommand" -a query -d "Query state"
complete -c isolate -n "__fish_use_subcommand" -a whereami -d "Location query"
complete -c isolate -n "__fish_use_subcommand" -a whoami -d "Identity query"
complete -c isolate -n "__fish_use_subcommand" -a contract -d "Show contracts"
complete -c isolate -n "__fish_use_subcommand" -a examples -d "Show examples"
complete -c isolate -n "__fish_use_subcommand" -a validate -d "Validate inputs"
complete -c isolate -n "__fish_use_subcommand" -a whatif -d "Preview command"
complete -c isolate -n "__fish_use_subcommand" -a claim -d "Claim lock"
complete -c isolate -n "__fish_use_subcommand" -a yield -d "Release lock"
complete -c isolate -n "__fish_use_subcommand" -a events -d "Show events"
complete -c isolate -n "__fish_use_subcommand" -a batch -d "Batch execute"
complete -c isolate -n "__fish_use_subcommand" -a completions -d "Generate completions"

# Session name completion for relevant commands
function __fish_isolate_sessions
    isolate list --json 2>/dev/null | jq -r '.data[].name' 2>/dev/null
end

complete -c isolate -n "__fish_seen_subcommand_from focus remove status sync diff claim yield" -a "(__fish_isolate_sessions)"

# Global flags
complete -c isolate -l json -d "Output as JSON"
complete -c isolate -l help -d "Show help"
"#.to_string()
}

fn generate_powershell_completions() -> String {
    r#"# isolate PowerShell completion

$script:isolateCommands = @(
    'init', 'add', 'list', 'remove', 'focus', 'status', 'sync', 'done',
    'undo', 'revert', 'spawn', 'work', 'abort', 'agents', 'ai', 'checkpoint',
    'clean', 'config', 'context', 'diff', 'doctor', 'introspect',
    'query', 'whereami', 'whoami', 'contract', 'examples', 'validate', 'whatif',
    'claim', 'yield', 'events', 'batch', 'completions'
)

Register-ArgumentCompleter -CommandName isolate -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $words = $commandAst.CommandElements

    if ($words.Count -eq 1) {
        # Complete commands
        $script:isolateCommands | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
    }
    elseif ($words.Count -ge 2) {
        $command = $words[1].Extent.Text

        # Complete session names for relevant commands
        if ($command -in @('focus', 'remove', 'status', 'sync', 'diff', 'claim', 'yield')) {
            $sessions = isolate list --json 2>$null | ConvertFrom-Json | Select-Object -ExpandProperty data | Select-Object -ExpandProperty name
            $sessions | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
            }
        }

        # Complete flags
        @('--json', '--help', '--dry-run') | Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
    }
}
"#.to_string()
}

fn generate_elvish_completions() -> String {
    r"# isolate elvish completion

set edit:completion:arg-completer[isolate] = {|@words|
    var commands = [
        init add list remove focus status sync done undo revert spawn work abort
        agents ai checkpoint clean config context diff doctor introspect
        query whereami whoami contract examples validate whatif claim yield events
        batch completions
    ]

    if (eq (count $words) 1) {
        # Complete commands
        for cmd $commands {
            put $cmd
        }
    } elif (eq (count $words) 2) {
        var cmd = $words[1]
        if (has-value [focus remove status sync diff claim yield] $cmd) {
            # Complete session names
            try {
                var sessions = (isolate list --json 2>/dev/null | from-json)[data]
                for sess $sessions {
                    put $sess[name]
                }
            } catch { }
        }
    }
}
"
    .to_string()
}

fn get_install_instructions(shell: Shell) -> String {
    match shell {
        Shell::Bash => "Add to ~/.bashrc:\n  source <(isolate completions bash)".to_string(),
        Shell::Zsh => "Add to ~/.zshrc:\n  source <(isolate completions zsh)\n\nOr save to ~/.zfunc/_isolate".to_string(),
        Shell::Fish => "Save to ~/.config/fish/completions/isolate.fish:\n  isolate completions fish > ~/.config/fish/completions/isolate.fish".to_string(),
        Shell::PowerShell => "Add to $PROFILE:\n  isolate completions powershell | Out-String | Invoke-Expression".to_string(),
        Shell::Elvish => "Save to ~/.elvish/lib/isolate.elv:\n  isolate completions elvish > ~/.elvish/lib/isolate.elv".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_from_str() -> anyhow::Result<()> {
        assert!(matches!("bash".parse::<Shell>()?, Shell::Bash));
        assert!(matches!("zsh".parse::<Shell>()?, Shell::Zsh));
        assert!(matches!("fish".parse::<Shell>()?, Shell::Fish));
        assert!(matches!("powershell".parse::<Shell>()?, Shell::PowerShell));
        Ok(())
    }

    #[test]
    fn test_shell_from_str_case_insensitive() -> anyhow::Result<()> {
        assert!(matches!("BASH".parse::<Shell>()?, Shell::Bash));
        assert!(matches!("ZSH".parse::<Shell>()?, Shell::Zsh));
        Ok(())
    }

    #[test]
    fn test_shell_from_str_invalid() {
        assert!("invalid".parse::<Shell>().is_err());
    }

    #[test]
    fn test_generate_bash_completions() {
        let script = generate_bash_completions();
        assert!(script.contains("_isolate()"));
        assert!(script.contains("complete -F _isolate isolate"));
    }

    #[test]
    fn test_generate_zsh_completions() {
        let script = generate_zsh_completions();
        assert!(script.contains("#compdef isolate"));
        assert!(script.contains("_isolate()"));
    }

    #[test]
    fn test_generate_fish_completions() {
        let script = generate_fish_completions();
        assert!(script.contains("complete -c isolate"));
    }

    #[test]
    fn test_install_instructions() {
        let bash_install = get_install_instructions(Shell::Bash);
        assert!(bash_install.contains("bashrc"));

        let zsh_install = get_install_instructions(Shell::Zsh);
        assert!(zsh_install.contains("zshrc"));
    }
}
