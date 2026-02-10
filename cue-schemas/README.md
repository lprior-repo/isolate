# CUE Schemas

This directory contains all CUE (Configure, Unite, Execute) schema files for the zjj project.

## Schema Files

### Core Architecture & Design
- **architecture.cue** - System components, data flow, and technology stack
- **requirements.cue** - Feature requirements and specifications
- **research.cue** - Research notes and exploratory design

### Command Specifications
- **commands.cue** - CLI command definitions and interfaces
- **config.cue** - Configuration structure and defaults
- **layouts.cue** - Zellij layout templates and specifications

### Development & Operations
- **cicd.cue** - CI/CD pipeline definitions and configurations
- **spec-intent.cue** - Intent specifications for command behaviors

### Protocol Definitions
- **zjj_protocol.cue** - JSON Schema definitions for zjj CLI protocol

## Usage

### Validate CUE schemas
```bash
cue vet cue-schemas/*.cue
```

### Export to JSON
```bash
cue export cue-schemas/zjj_protocol.cue --out json
```

### Export specific fields
```bash
cue export cue-schemas/architecture.cue --out json -e components
```

## Integration

The schema files are used by:
- Test validation (see `crates/zjj/tests/schema_tests.rs`)
- Documentation generation
- Type-safe contract definitions

## See Also

- [CUE Language](https://cuelang.org/)
- [CUE Documentation](https://cuelang.org/docs/)
