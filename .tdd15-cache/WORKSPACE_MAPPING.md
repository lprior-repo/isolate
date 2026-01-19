# TDD15 Workspace-to-Bead Mapping

| Workspace | Bead ID | Title | Complexity | Phases |
|-----------|---------|-------|------------|--------|
| config-subs | zjj-viue | Config command subcommands | MEDIUM | 0→1→2→4→5→6→7→9→11→15 |
| error-codes | zjj-t283 | Error code semantic mapping | MEDIUM | 0→1→2→4→5→6→7→9→11→15 |
| silent-flag | zjj-o14q | Add --silent flag to all commands | MEDIUM | 0→1→2→4→5→6→7→9→11→15 |
| filter-flags | zjj-a50v | Standardize filter flag naming | SIMPLE | 0→4→5→6→14→15 |
| cue-schema | zjj-gm9a | Create CUE schema for JSON | COMPLEX | All 16 phases |
| dryrun-output | zjj-jwwd | Normalize dry-run output | MEDIUM | 0→1→2→4→5→6→7→9→11→15 |
| batch-output | zjj-3l0p | Standardize batch operation output | SIMPLE | 0→4→5→6→14→15 |
| jjz-refs | zjj-h6di | Update 'jjz' references in help | SIMPLE | 0→4→5→6→14→15 |
| git-tag | zjj-jxzp | Create git tag for v0.2.0 | SIMPLE | 0→4→5→6→14→15 |
| help-text | zjj-ffz9 | Update help text for v0.2.0 | MEDIUM | 0→1→2→4→5→6→7→9→11→15 |

## Complexity Breakdown

**SIMPLE (4 beads)**: filter-flags, batch-output, jjz-refs, git-tag
- Phases: 0→4→5→6→14→15 (6 phases, ~60% time savings)

**MEDIUM (5 beads)**: config-subs, error-codes, silent-flag, dryrun-output, help-text  
- Phases: 0→1→2→4→5→6→7→9→11→15 (10 phases, ~35% time savings)

**COMPLEX (1 bead)**: cue-schema
- Phases: All 16 phases (0% time savings)

## Workspace Locations

All workspaces are in: `/home/lewis/src/zjj__workspaces/<name>/`

## TDD15 Cache

All phase data in: `/home/lewis/src/zjj/.tdd15-cache/<bead-id>/`

## Next Steps

1. Execute phases in parallel using subagents
2. Each workspace works independently
3. Sync changes back to main when complete
