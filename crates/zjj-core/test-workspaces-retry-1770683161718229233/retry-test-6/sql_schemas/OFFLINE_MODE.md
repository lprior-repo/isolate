# SQLx Offline Mode

SQLx provides compile-time query checking. For offline/CI mode:

## Development (Online Mode)

```bash
export DATABASE_URL="sqlite:zjj.db"
cargo build
```

In online mode, SQLx connects to the database at compile time to verify queries.

## CI/Offline Mode

```bash
export SQLX_OFFLINE=true
cargo build
```

In offline mode, SQLx uses the pre-generated metadata in `.sqlx/sqlx-data.json` for compile-time verification.

## Regenerating Metadata

After modifying SQL queries, regenerate the metadata:

```bash
cargo install sqlx-cli
cargo sqlx prepare
```

This updates `.sqlx/sqlx-data.json` with the latest query information.

## Files

- `.sqlx/sqlx-data.json` - Pre-generated query metadata for offline compilation
- `.sqlx/.gitkeep` - Ensures the directory is tracked in git
- `.env.example` - Example environment configuration

## CI Configuration

The moon.yml tasks automatically set `SQLX_OFFLINE=true` for CI builds.
