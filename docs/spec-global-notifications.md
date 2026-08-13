# Spec: Global notifications

## Objective

Move Ping, SMTP, and ServerChan settings out of individual backup plans into one
system-wide notification module. Manual and scheduled runs share the same
confirmed configuration, so administrators configure delivery and event choices
once and can test each channel before relying on it.

## Tech stack

- Rust 2024, Axum 0.8, Tokio, SQLx/SQLite
- AES-256-GCM via the existing `Store` encryption key
- Embedded HTML, CSS, and plain JavaScript; no new frontend dependency

## Commands

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
node --check web/app.js
git diff --check
```

## Project structure

- `src/model.rs`: global notification and migration-candidate models
- `src/store.rs`: encrypted singleton storage and legacy-plan migration
- `src/api.rs`: read, update, confirm, and test endpoints
- `src/runner.rs`: delivery from the confirmed global configuration
- `web/`: independent notification navigation and settings UI
- `docs/`: user-facing and engineering specification

## Code style

Use existing typed models, `Result`-based validation, parameterized SQL, and
small async store/API methods. Secrets are merged server-side when a client sends
`REDACTED`; clients never need the plaintext value after saving it.

```rust
if input.serverchan.send_key == REDACTED {
    input.serverchan.send_key = existing.serverchan.send_key;
}
input.validate()?;
store.save_notification_settings(&input).await?;
```

## Testing strategy

- Unit-test validation and event/channel selection.
- Store integration tests use temporary SQLite databases and encryption keys.
- API tests verify masking, secret preservation, and validation errors.
- Migration tests cover no legacy settings, one/unanimous legacy setting, and
  conflicting settings.
- Browser checks cover navigation, form feedback, keyboard labels, responsive
  layout, test-button feedback, and reduced motion.

## Boundaries

- Always: encrypt global settings and candidates at rest; mask secrets in API
  responses and logs; retain legacy plan notification fields for rollback.
- Ask first: add third-party frontend or notification dependencies; remove legacy
  plan notification data; change the existing notification command semantics.
- Never: silently pick one of several conflicting plan configurations; execute a
  shell; expose a URL, SendKey, or SMTP credential in an API response or log;
  connect to a notification hostname after a separate, unpinned DNS lookup.

## Success criteria

- There is one top-level notification module for Ping, SMTP, and ServerChan.
- Administrators can enable channels, choose start/success/failure events, save,
  and send a test notification with visible loading/success/error feedback.
- New and edited backup plans do not expose or overwrite notification settings.
- All runs load one confirmed global snapshot at start and reuse it for the run.
- Identical non-empty legacy settings migrate automatically and become active.
- Conflicting legacy settings remain inactive; the UI lists their plan sources
  and requires explicit candidate selection or a newly entered configuration.
- Old plan documents retain their notification fields.
- Stored global settings and conflict candidates use `enc:v1:` encryption.
- Secret API fields are masked and `REDACTED` preserves the stored value.

## Open questions

None. The migration and interaction behavior was approved by the user.
