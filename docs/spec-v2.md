# Rclone Backup v2 specification

## Scope

Rclone Backup v2 is a long-running Rust Web service for directory backups. It does not export databases. The embedded Web UI is bilingual, theme-aware, mobile responsive, and remains usable before rclone is configured.

## Lifecycle

- `/api/health` reports process liveness.
- Web UI, API, and application SQLite start even when no rclone alias exists.
- Readiness is unlocked when at least one alias appears in `rclone.conf`; no restart is required.
- Scheduled work waits for readiness. Manual work returns HTTP 409 while not ready.
- rclone RC is private to `127.0.0.1:5572` with random process credentials.

## Data boundaries

- `/config/rclone/rclone.conf`: providers, endpoints, account credentials, OAuth tokens, crypt/compress settings.
- Application SQLite: backup plans, alias references, scheduling, retry/retention settings, encrypted global notifications, and run history.
- Plan documents are AES-256-GCM encrypted before SQLite storage. The key is separate from the database and defaults to `/config/.rclone-backup.key` mode 0600.
- Account responses contain only alias and provider type.

## Backup and restore

- One source in a standard archive extracts directly to the directory contents.
- Multiple sources extract to one top-level directory per named source.
- 7z with a password uses AES-256 and filename encryption.
- ZIP with a password uses ZipCrypto for compatibility and is labelled as weaker.
- An empty password produces an ordinary unencrypted archive.
- Native directory and rclone virtual remotes are advanced paths whose restore depends on rclone.
- Retention by age/count is allowed for archive modes and is scoped to the plan filename prefix.

## Accounts

- Provider schema comes from rclone `config/providers`.
- Create/update/delete/test use the private rclone API.
- Create and update test automatically; every account card also exposes Test.
- Deletion is rejected while a plan references the alias.
- Plans select aliases obtained from the API; free-form aliases are rejected server-side.

## Compatibility

- Legacy process env, `_FILE`, and `/.env` settings are imported exactly once.
- Numbered directory and remote variables remain supported.
- `RCLONE_BACKUP_PLANS` can seed multiple plans once.
- Database-export environment variables do not create backup work in v2.
- The default command starts the service; `backup`, global notification tests, and command passthrough remain available.

## Release gates

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```
