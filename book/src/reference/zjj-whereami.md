# zjj whereami

Show current workspace.

```bash
zjj whereami
```

## Examples

```bash
zjj whereami
# workspace:feature-auth

# Or in main:
zjj whereami
# main
```

## JSON Output

```bash
zjj whereami --json
# {"location":"workspace","name":"feature-auth"}
```

## Use in Scripts

```bash
if [[ $(zjj whereami) == "main" ]]; then
  echo "In main workspace"
fi
```
