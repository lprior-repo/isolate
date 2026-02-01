# Zellij Integration Guide

Complete reference for Zellij terminal multiplexer integration with zjj.

**Read time**: 25 minutes
**Prerequisites**: Basic terminal knowledge, understanding of workspaces

---

## Table of Contents

1. [What is Zellij?](#what-is-zellij)
2. [Why Zellij for zjj?](#why-zellij-for-zjj)
3. [KDL Layout Language](#kdl-layout-language)
4. [Layout Templates](#layout-templates)
5. [Pane Configuration](#pane-configuration)
6. [Tab Management](#tab-management)
7. [Zellij Actions](#zellij-actions)
8. [zjj Integration](#zjj-integration)
9. [Advanced Features](#advanced-features)
10. [Troubleshooting](#troubleshooting)

---

## What is Zellij?

Zellij is a terminal multiplexer (like tmux/screen) with modern features:

- **Tabs and Panes**: Organize terminal sessions visually
- **Layouts**: Define workspace structure declaratively
- **Plugins**: Extensible with WebAssembly
- **Session Management**: Detach/attach to sessions
- **KDL Configuration**: Human-readable config language

**Official Documentation**: [zellij.dev](https://zellij.dev/)

---

## Why Zellij for zjj?

zjj uses Zellij to create isolated, visual workspaces for each development session:

| Feature | Benefit |
|---------|---------|
| **Tabs** | One tab per zjj session, easy switching |
| **Layouts** | Pre-configured panes (Claude, beads, jj log) |
| **Programmable** | Create/close/focus tabs via CLI |
| **Session Aware** | zjj runs inside Zellij, detects `$ZELLIJ` |
| **No Nesting** | Uses tabs within current session (no sub-sessions) |

**Design Decision**: zjj creates **tabs** within your current Zellij session, not new sessions. This avoids nested multiplexers and follows the "already in Zellij" pattern.

---

## KDL Layout Language

Zellij layouts are defined in KDL (KDL Document Language), a readable format similar to JSON/YAML.

### Basic Syntax

```kdl
// Single pane layout
layout {
    pane {
        command "htop"
    }
}
```

### Properties

KDL nodes can have:
- **Arguments on same line**: `pane command="htop"`
- **Arguments in braces**: `pane { command "htop" }`
- **Child nodes**: Nested panes, tabs, etc.

### Comments

```kdl
// Single-line comment
/* Multi-line
   comment */
```

---

## Layout Templates

zjj provides 5 pre-built templates (see `crates/zjj-core/src/zellij.rs`):

### 1. Minimal

Single Claude pane, full screen.

```kdl
layout {
    pane {
        command "claude"
        cwd "/workspace"
        focus true
    }
}
```

**Use case**: Focused coding with no distractions.

---

### 2. Standard (Default)

70% Claude + 30% sidebar (beads + jj log).

```kdl
layout {
    pane split_direction="horizontal" {
        pane {
            command "claude"
            cwd "/workspace"
            focus true
            size "70%"
        }
        pane split_direction="vertical" {
            pane {
                command "bv"
                cwd "/workspace"
                size "50%"
            }
            pane {
                command "jj"
                args "log" "--limit" "20"
                cwd "/workspace"
                size "50%"
            }
        }
    }
}
```

**Pane breakdown**:
- **Left 70%**: Claude Code editor (focused)
- **Top-right 15%**: `bv` (beads triage TUI)
- **Bottom-right 15%**: `jj log` (commit history)

**Use case**: Standard development workflow with issue tracking and version control visibility.

---

### 3. Full

Standard + floating status pane.

```kdl
layout {
    pane split_direction="horizontal" {
        pane {
            command "claude"
            cwd "/workspace"
            focus true
            size "70%"
        }
        pane split_direction="vertical" {
            pane {
                command "bv"
                cwd "/workspace"
                size "50%"
            }
            pane {
                command "jj"
                args "log" "--limit" "20"
                cwd "/workspace"
                size "50%"
            }
        }
    }
    floating_panes {
        pane {
            command "jj"
            args "status"
            cwd "/workspace"
            x "20%"
            y "20%"
            width "60%"
            height "60%"
        }
    }
}
```

**Additional feature**: Floating `jj status` pane (toggle with `Ctrl+p + w`).

**Use case**: When you need quick access to full status without switching panes.

---

### 4. Split

Two Claude instances side-by-side (50/50).

```kdl
layout {
    pane split_direction="horizontal" {
        pane {
            command "claude"
            cwd "/workspace"
            focus true
            size "50%"
        }
        pane {
            command "claude"
            cwd "/workspace"
            size "50%"
        }
    }
}
```

**Use case**: Pair programming, comparing implementations, or working on related files.

---

### 5. Review

Diff view (50%) + beads (25%) + Claude (25%).

```kdl
layout {
    pane split_direction="horizontal" {
        pane {
            command "jj"
            args "diff"
            cwd "/workspace"
            focus true
            size "50%"
        }
        pane {
            command "bv"
            cwd "/workspace"
            size "25%"
        }
        pane {
            command "claude"
            cwd "/workspace"
            size "25%"
        }
    }
}
```

**Use case**: Code review, analyzing diffs, or preparing commits.

---

## Pane Configuration

### Core Properties

| Property | Type | Description | Example |
|----------|------|-------------|---------|
| `command` | String | Command to run | `"claude"` |
| `args` | String... | Command arguments | `"log" "--limit" "20"` |
| `cwd` | Path | Working directory | `"/workspace"` |
| `focus` | Boolean | Auto-focus this pane | `true` |
| `size` | Percent/Fixed | Pane size | `"70%"` or `100` (cols) |
| `split_direction` | Enum | Child layout | `"horizontal"` or `"vertical"` |
| `name` | String | Pane identifier | `"editor"` |

### split_direction

Controls how child panes are arranged:

```kdl
// Horizontal: panes side-by-side (left/right)
pane split_direction="horizontal" {
    pane { }  // Left
    pane { }  // Right
}

// Vertical: panes stacked (top/bottom)
pane split_direction="vertical" {
    pane { }  // Top
    pane { }  // Bottom
}
```

**Default**: `"horizontal"`

### cwd Inheritance

```kdl
layout {
    tab cwd="/project" {
        pane                  // cwd = "/project"
        pane cwd="src"        // cwd = "/project/src"
        pane cwd="/other"     // cwd = "/other" (absolute)
    }
}
```

**Rules**:
1. Absolute paths override parent cwd
2. Relative paths are joined to parent cwd
3. Tab-level cwd applies to all child panes

### command and args

```kdl
// Single argument
pane command="htop"

// Multiple arguments (space-separated)
pane command="jj" {
    args "log" "--limit" "20"
}

// Complex command
pane command="bash" {
    args "-c" "cd /workspace && npm run dev"
}
```

**Note**: Use `args` for safety (no shell injection). Avoid shell strings unless necessary.

### focus

```kdl
layout {
    pane { }                    // Not focused
    pane { focus true }         // Focused on load
    pane { }                    // Not focused
}
```

**Rule**: Only one pane should have `focus true` per layout.

---

## Tab Management

### Tab Properties

```kdl
layout {
    tab name="My Tab" cwd="/workspace" focus=true {
        pane { }
    }
}
```

| Property | Description |
|----------|-------------|
| `name` | Tab display name |
| `cwd` | Default working directory for all panes |
| `focus` | Whether to focus this tab on load |

### Multiple Tabs

```kdl
layout {
    tab name="Code" focus=true {
        pane command="claude"
    }
    tab name="Logs" {
        pane command="tail" {
            args "-f" "/var/log/app.log"
        }
    }
    tab name="Tests" {
        pane command="moon" {
            args "run" ":test" "--watch"
        }
    }
}
```

**zjj pattern**: zjj creates **one tab per session**, named `zjj:<session-name>`.

---

## Zellij Actions

Zellij provides CLI commands to control running sessions (see `crates/zjj-core/src/zellij.rs`).

### new-tab

Create a new tab with a layout:

```bash
zellij action new-tab --layout /path/to/layout.kdl --name "My Tab"
```

**zjj usage**: `zjj add <name>` generates a layout and calls this command.

**Implementation** (`zellij.rs:345`):
```rust
pub fn tab_open(layout_path: &Path, tab_name: &str) -> Result<()> {
    check_zellij_running()?;

    let output = Command::new("zellij")
        .args(["action", "new-tab"])
        .arg("--layout")
        .arg(layout_path)
        .arg("--name")
        .arg(tab_name)
        .output()?;

    // Error handling...
}
```

---

### close-tab

Close the currently focused tab:

```bash
zellij action close-tab
```

**zjj pattern**: Focus the tab first, then close it.

**Implementation** (`zellij.rs:382`):
```rust
pub fn tab_close(tab_name: &str) -> Result<()> {
    check_zellij_running()?;

    // First focus the tab
    tab_focus(tab_name)?;

    // Then close it
    Command::new("zellij")
        .args(["action", "close-tab"])
        .output()?;

    Ok(())
}
```

**Why focus first?** `close-tab` closes the *active* tab, so we must focus it first.

---

### go-to-tab-name

Switch to a tab by name:

```bash
zellij action go-to-tab-name "zjj:my-feature"
```

**zjj usage**: `zjj focus <name>` uses this to switch sessions.

**Implementation** (`zellij.rs:411`):
```rust
pub fn tab_focus(tab_name: &str) -> Result<()> {
    check_zellij_running()?;

    Command::new("zellij")
        .args(["action", "go-to-tab-name", tab_name])
        .output()?;

    Ok(())
}
```

---

### Other Actions

Zellij supports many actions (not used by zjj currently):

- `split-pane` - Create new pane in current tab
- `resize` - Resize focused pane
- `move-focus` - Navigate between panes
- `toggle-floating-panes` - Show/hide floating panes
- `dump-layout` - Export current layout to KDL

See: `zellij action --help`

---

## zjj Integration

### Architecture

```
┌─────────────────────────────────────────────────────┐
│ Zellij Session (user's terminal)                    │
│                                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────┐ │
│  │ Tab: main    │  │ Tab: zjj:feat│  │ Tab: logs │ │
│  └──────────────┘  └──────────────┘  └───────────┘ │
│                          │                           │
│                          ▼                           │
│              ┌─────────────────────┐                 │
│              │ Layout: Standard    │                 │
│              │ ┌─────────┬───────┐ │                 │
│              │ │ Claude  │ bv    │ │                 │
│              │ │ (70%)   │ jj log│ │                 │
│              │ └─────────┴───────┘ │                 │
│              └─────────────────────┘                 │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
              ┌─────────────────────┐
              │ zjj CLI              │
              │ - add/remove/focus   │
              │ - Layout generation  │
              └─────────────────────┘
                         │
                         ▼
              ┌─────────────────────┐
              │ JJ Workspace         │
              │ - Separate branch    │
              │ - Isolated changes   │
              └─────────────────────┘
```

### Workflow

1. **User runs `zjj add feature-x`**
   - zjj generates KDL layout file
   - Creates JJ workspace for `feature-x`
   - Calls `zellij action new-tab` with layout
   - Tab named `zjj:feature-x` appears

2. **User switches with `zjj focus feature-x`**
   - zjj calls `zellij action go-to-tab-name zjj:feature-x`
   - Zellij switches to that tab
   - All panes have cwd set to workspace dir

3. **User removes with `zjj remove feature-x`**
   - zjj focuses tab with `go-to-tab-name`
   - Calls `zellij action close-tab`
   - Cleans up JJ workspace
   - Removes layout file

### Configuration

Layout config is defined in `zjj-core::zellij::LayoutConfig`:

```rust
pub struct LayoutConfig {
    pub session_name: String,       // "feature-x"
    pub workspace_path: PathBuf,    // "/repo/.jj/workspaces/feature-x"
    pub claude_command: String,     // "claude" (customizable)
    pub beads_command: String,      // "bv" (customizable)
    pub tab_prefix: String,         // "zjj" (customizable)
}

impl LayoutConfig {
    pub fn tab_name(&self) -> String {
        format!("{}:{}", self.tab_prefix, self.session_name)
    }
}
```

**Customization**:
```rust
let config = LayoutConfig::new("my-session".to_string(), workspace_path)
    .with_claude_command("nvim".to_string())
    .with_beads_command("bd list".to_string())
    .with_tab_prefix("dev".to_string());

// Tab name: "dev:my-session"
```

### Layout Generation

**API** (`zellij.rs:115`):
```rust
pub fn layout_generate(
    config: &LayoutConfig,
    template: LayoutTemplate,
    output_dir: &Path,
) -> Result<Layout>
```

**Process**:
1. Generate KDL content from template
2. Validate syntax (balanced braces, required nodes)
3. Write to `{output_dir}/{session_name}.kdl`
4. Return `Layout` with content and path

**Example**:
```rust
let config = LayoutConfig::new(
    "feature-x".to_string(),
    PathBuf::from("/repo/.jj/workspaces/feature-x"),
);

let layout = layout_generate(
    &config,
    LayoutTemplate::Standard,
    Path::new("/tmp/layouts"),
)?;

// layout.file_path: /tmp/layouts/feature-x.kdl
// layout.kdl_content: "layout { ... }"
```

### Validation

KDL validation checks (`zellij.rs:305`):
1. **Balanced braces**: `{` count == `}` count
2. **Required nodes**: Must contain `layout` and `pane`
3. **Syntax**: No empty node names

**Errors**:
```rust
Error::ValidationError("Unbalanced braces: 3 open, 2 close")
Error::ValidationError("KDL must contain 'layout' node")
Error::ValidationError("KDL must contain at least one 'pane' node")
```

### Environment Check

zjj requires Zellij to be running (`zellij.rs:434`):

```rust
pub fn check_zellij_running() -> Result<()> {
    if std::env::var("ZELLIJ").is_err() {
        return Err(Error::Command(
            "Zellij not running. Run zjj inside a Zellij session.".to_string(),
        ));
    }
    Ok(())
}
```

**How to check**:
```bash
echo $ZELLIJ  # Should print session ID, e.g., "1234567890"
```

**Starting Zellij**:
```bash
zellij  # Start new session
zellij attach  # Attach to existing session
```

---

## Advanced Features

### Pane Templates

Reusable pane definitions:

```kdl
layout {
    pane_template name="follow-log" command="tail" {
        args "-f"
    }

    follow-log {
        args "/var/log/app.log"
    }
    follow-log {
        args "/var/log/error.log"
        cwd "/var/log"
    }
}
```

**Use case**: Multiple similar panes with slight variations.

### Tab Templates

Reusable tab definitions:

```kdl
layout {
    tab_template name="dev-tab" {
        pane split_direction="horizontal" {
            pane command="claude" size="70%"
            pane command="bv" size="30%"
        }
    }

    dev-tab name="Frontend" cwd="/app/frontend"
    dev-tab name="Backend" cwd="/app/backend"
}
```

**Use case**: Multiple workspaces with same layout structure.

### Floating Panes

Overlay panes (like modal windows):

```kdl
floating_panes {
    pane {
        command "htop"
        x "10%"       // Left offset
        y "10%"       // Top offset
        width "80%"   // Pane width
        height "80%"  // Pane height
    }
}
```

**Toggle**: `Ctrl+p + w` (default keybinding)

### Session Serialization

Zellij auto-saves layouts on exit:

```bash
~/.cache/zellij/session-layout.kdl
```

**Use case**: Restore exact session state after restart.

**Manual export**:
```bash
zellij action dump-layout > my-layout.kdl
```

---

## Troubleshooting

### Tab not appearing

**Symptom**: `zjj add` succeeds but no tab appears.

**Checks**:
1. Verify Zellij is running: `echo $ZELLIJ`
2. Check layout file exists: `ls ~/.local/share/zjj/layouts/`
3. Test layout manually: `zellij action new-tab --layout /path/to/layout.kdl`
4. Check Zellij logs: `zellij action dump-log`

**Common cause**: Invalid KDL syntax in generated layout.

---

### "Zellij not running" error

**Symptom**: All zjj commands fail with this error.

**Solution**: Start Zellij first:
```bash
zellij
# Then run zjj commands
```

**Why**: zjj uses the `$ZELLIJ` environment variable to detect running sessions.

---

### Tab has wrong cwd

**Symptom**: Panes open in wrong directory.

**Checks**:
1. Verify workspace path: `jj workspace list`
2. Check layout file: `cat ~/.local/share/zjj/layouts/<session>.kdl`
3. Ensure absolute paths in cwd settings

**Fix**: Regenerate layout with correct workspace path.

---

### Command not found in pane

**Symptom**: Pane shows "command not found: claude"

**Checks**:
1. Verify command in PATH: `which claude`
2. Check Zellij inherits shell env: `zellij setup --check`
3. Use absolute paths: `command "/usr/local/bin/claude"`

**Workaround**:
```kdl
pane {
    command "bash"
    args "-c" "source ~/.bashrc && claude"
}
```

---

### Pane layout doesn't match template

**Symptom**: Sizes or splits are wrong.

**Checks**:
1. Verify terminal size is large enough
2. Check size constraints (min pane size is ~10 cols)
3. Use percentages, not fixed sizes

**Example issue**:
```kdl
// Terminal is 100 cols wide
pane size="200"  // Invalid: exceeds terminal width
```

**Fix**:
```kdl
pane size="50%"  // Always valid
```

---

### Multiple tabs with same name

**Symptom**: `zjj focus` switches to wrong tab.

**Cause**: Multiple tabs named `zjj:feature-x`.

**Prevention**: zjj enforces unique session names.

**Manual fix**:
```bash
zellij action rename-tab "zjj:feature-x-2"
```

---

### Floating pane won't close

**Symptom**: Floating pane persists after closing.

**Solution**: Toggle floating panes off:
```bash
zellij action toggle-floating-panes
```

Or close focused pane:
```bash
zellij action close-pane
```

---

### Layout changes not reflected

**Symptom**: Edited layout file, but tab unchanged.

**Why**: Layouts are applied on tab creation, not dynamically.

**Solution**:
1. Close tab: `zjj remove <name>`
2. Recreate: `zjj add <name>`

**Alternative**: Use `zellij action dump-layout` to see current state.

---

## Quick Reference

### Key Files

| Path | Purpose |
|------|---------|
| `crates/zjj-core/src/zellij.rs` | Core layout generation and tab management |
| `~/.local/share/zjj/layouts/` | Generated layout files |
| `~/.cache/zellij/session-layout.kdl` | Auto-saved session state |
| `~/.config/zellij/config.kdl` | Zellij configuration |

### Commands

```bash
# zjj commands
zjj add <name>           # Create session + tab
zjj focus <name>         # Switch to tab
zjj remove <name>        # Close tab + workspace
zjj list                 # Show all sessions

# Zellij actions (manual control)
zellij action new-tab --layout <path> --name <name>
zellij action close-tab
zellij action go-to-tab-name <name>
zellij action dump-layout
zellij action toggle-floating-panes
```

### Layout Template Selection

```rust
use zjj_core::zellij::{LayoutTemplate, LayoutConfig, layout_generate};

let config = LayoutConfig::new("my-session".to_string(), workspace_path);

// Choose template
let template = match use_case {
    "minimal" => LayoutTemplate::Minimal,
    "standard" => LayoutTemplate::Standard,
    "full" => LayoutTemplate::Full,
    "split" => LayoutTemplate::Split,
    "review" => LayoutTemplate::Review,
    _ => LayoutTemplate::Standard,
};

let layout = layout_generate(&config, template, output_dir)?;
```

---

## Best Practices

1. **Use Standard template by default** - Balances editor space with visibility.
2. **Customize commands for your workflow** - Replace `claude` with your editor.
3. **Keep cwd absolute** - Avoids confusion with relative paths.
4. **One focus per layout** - Only one pane should have `focus true`.
5. **Test layouts manually first** - Use `zellij action new-tab` before integrating.
6. **Use percentages for sizes** - More robust than fixed widths.
7. **Avoid shell strings in commands** - Use `args` for security.
8. **Document custom layouts** - Add comments in KDL files.

---

## Next Steps

- **Moon Build System**: [02_MOON_BUILD.md](02_MOON_BUILD.md)
- **JJ Workspaces**: [09_JUJUTSU.md](09_JUJUTSU.md)
- **Beads Integration**: [08_BEADS.md](08_BEADS.md)
- **zjj Workflow**: [03_WORKFLOW.md](03_WORKFLOW.md)

---

## External Resources

- [Zellij Official Website](https://zellij.dev/)
- [Zellij Documentation](https://zellij.dev/documentation/)
- [KDL Language Spec](https://kdl.dev/)
- [Zellij GitHub Repository](https://github.com/zellij-org/zellij)
- [Layout Examples](https://github.com/zellij-org/zellij/tree/main/example/layouts)
- [Zellij Layout Tutorial](https://zellij.dev/tutorials/layouts/)
- [Creating Layouts Guide](https://zellij.dev/documentation/creating-a-layout.html)

**Sources**:
- [GitHub - zellij-org/zellij](https://github.com/zellij-org/zellij)
- [Zellij Layout Documentation](https://zellij.dev/documentation/layouts.html)
- [Zellij 0.32.0 Release Notes](https://zellij.dev/news/config-command-layouts/)
- [Layout and Configuration System](https://deepwiki.com/zellij-org/zellij/2.3-layout-and-configuration-system)
- [Zellij Layout Examples](https://zellij.dev/documentation/layout-examples.html)

---

**Remember**: zjj manages Zellij tabs, JJ manages workspaces. Together they provide isolated, visual development environments.
