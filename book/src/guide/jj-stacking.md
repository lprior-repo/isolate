# Stacking Changes

Work with dependent changes.

## What is Stacking?

Multiple commits that build on each other:
```
main → commit-1 → commit-2 → commit-3
```

## In ZJJ

ZJJ workspaces handle stacking automatically:

```bash
# Each workspace is a stack
zjj add feature-part-1 --bead BD-101
# Work...
zjj done

zjj add feature-part-2 --bead BD-102
# Builds on part-1 (via main)
zjj done
```

## JJ Stacking

Within a workspace, stack with JJ:

```bash
# Create stack
jj commit -m "Part 1: API changes"
jj commit -m "Part 2: Frontend updates"
jj commit -m "Part 3: Documentation"

# View stack
jj log
# @  Part 3
# ○  Part 2
# ○  Part 1
# ○  main
```

## Landing Stacks

```bash
# Land entire stack
zjj done --message "Feature complete"

# Or split:
jj squash --into main  # Part 1
zjj done

# New workspace for Part 2
zjj add part-2 --bead BD-102
```

## Dependencies

If commit-2 depends on commit-1:

```bash
# Both in same workspace
jj commit -m "Part 1"
jj commit -m "Part 2"
zjj done  # Lands both
```

## Best Practices

**Keep stacks small:**
- 2-3 commits per stack
- Easy to review
- Fast to land

**Clear dependencies:**
```bash
# Good: Each commit makes sense alone
jj commit -m "Add API endpoint"
jj commit -m "Add tests"
jj commit -m "Add docs"
```
