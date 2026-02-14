# Conflict Resolution

Resolve merge conflicts in JJ.

## When Conflicts Happen

During sync:
```bash
zjj sync
# Warning: Conflicts in src/auth.rs
```

## Resolution Steps

1. **Focus workspace:**
```bash
zjj focus my-feature
```

2. **Check conflicts:**
```bash
jj status
# Shows conflicted files
```

3. **Resolve:**
```bash
jj resolve
# Opens conflicted file in editor
```

Or manually edit conflict markers:
```
<<<<<<<
main version
|||||||
base version
=======
my version
>>>>>>>
```

4. **Commit resolution:**
```bash
jj commit -m "Resolve conflicts"
```

5. **Retry sync:**
```bash
zjj sync
```

## Conflict Markers

JJ uses 3-way markers:
- `<<<<<<<` - Your changes
- `|||||||` - Base (common ancestor)
- `=======` - Separator
- `>>>>>>>` - Other changes

## Prevention

**Sync frequently:**
```bash
zjj sync  # Morning
# ... work ...
zjj sync  # Mid-day
```

**Smaller changes:**
```bash
jj commit -m "WIP"  # Commit often
```

## Tools

Configure merge tool:
```bash
jj config set --user merge-tool vimdiff
```

## See Also

- [JJ Documentation](https://martinvonz.github.io/jj/)
- `jj resolve --help`
