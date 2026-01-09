// jjz Zellij Layout Templates
// Defines KDL layout templates with variable substitution
package jjz

// ═══════════════════════════════════════════════════════════════════════════
// LAYOUT TEMPLATE SCHEMA
// ═══════════════════════════════════════════════════════════════════════════

#LayoutTemplate: {
    name:        string & !=""
    description: string & !=""
    kdl:         string & !=""
    variables:   [...string]
}

// ═══════════════════════════════════════════════════════════════════════════
// BUILT-IN TEMPLATES
// ═══════════════════════════════════════════════════════════════════════════

templates: [...#LayoutTemplate] & [
    {
        name:        "minimal"
        description: "Single pane with Claude Code only"
        variables:   ["{session_name}", "{workspace_path}"]
        kdl: """
            layout {
                tab name="{session_name}" cwd="{workspace_path}" {
                    pane command="claude"
                }
            }
            """
    },
    {
        name:        "standard"
        description: "Claude Code main pane with beads viewer and status sidebar"
        variables:   ["{session_name}", "{workspace_path}"]
        kdl: """
            layout {
                default_tab_template {
                    pane size=1 borderless=true {
                        plugin location="zellij:tab-bar"
                    }
                    children
                    pane size=2 borderless=true {
                        plugin location="zellij:status-bar"
                    }
                }

                tab name="{session_name}" cwd="{workspace_path}" {
                    pane split_direction="vertical" {
                        pane command="claude" size="70%" name="claude"
                        pane split_direction="horizontal" size="30%" {
                            pane command="bv" name="beads"
                            pane command="jjz" {
                                args "status" "--watch" "{session_name}"
                            }
                        }
                    }
                }
            }
            """
    },
    {
        name:        "full"
        description: "Full layout with floating pane and JJ log"
        variables:   ["{session_name}", "{workspace_path}"]
        kdl: """
            layout {
                default_tab_template {
                    pane size=1 borderless=true {
                        plugin location="zellij:tab-bar"
                    }
                    children
                    pane size=2 borderless=true {
                        plugin location="zellij:status-bar"
                    }
                }

                tab name="{session_name}" cwd="{workspace_path}" {
                    pane split_direction="vertical" {
                        pane command="claude" size="60%" name="claude"
                        pane split_direction="horizontal" size="40%" {
                            pane command="bv" name="beads" size="40%"
                            pane split_direction="vertical" size="60%" {
                                pane command="jjz" {
                                    args "status" "--watch" "{session_name}"
                                }
                                pane command="jj" {
                                    args "log" "--no-pager" "-r" "::@"
                                }
                            }
                        }
                    }
                    floating_panes {
                        pane command="" start_suspended=true x="10%" y="20%" width="80%" height="60%"
                    }
                }
            }
            """
    },
    {
        name:        "split"
        description: "Two Claude instances side by side for comparison"
        variables:   ["{session_name}", "{workspace_path}"]
        kdl: """
            layout {
                tab name="{session_name}" cwd="{workspace_path}" {
                    pane split_direction="vertical" {
                        pane command="claude" size="50%" name="claude-1"
                        pane command="claude" size="50%" name="claude-2"
                    }
                }
            }
            """
    },
    {
        name:        "review"
        description: "Layout for code review with diff and beads focus"
        variables:   ["{session_name}", "{workspace_path}"]
        kdl: """
            layout {
                tab name="{session_name}" cwd="{workspace_path}" {
                    pane split_direction="horizontal" {
                        pane split_direction="vertical" size="60%" {
                            pane command="jj" {
                                args "diff" "--git"
                            } name="diff" size="70%"
                            pane command="jj" {
                                args "log" "--no-pager" "-r" "::@"
                            } name="log" size="30%"
                        }
                        pane split_direction="vertical" size="40%" {
                            pane command="bv" name="beads" size="50%"
                            pane command="claude" name="claude" size="50%"
                        }
                    }
                }
            }
            """
    },
]

// ═══════════════════════════════════════════════════════════════════════════
// TEMPLATE RENDERING
// ═══════════════════════════════════════════════════════════════════════════

#RenderContext: {
    session_name:   string & =~"^[a-zA-Z0-9_-]+$"
    workspace_path: string & !=""
    repo_name:      string & !=""
    branch:         string | *""
    main_branch:    string | *"main"
}

// Example render context
_example_context: #RenderContext & {
    session_name:   "feature-auth"
    workspace_path: "/home/user/project__workspaces/feature-auth"
    repo_name:      "my-project"
    branch:         "feature-auth"
    main_branch:    "main"
}
