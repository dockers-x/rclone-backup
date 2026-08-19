# Rclone Backup

Rclone Backup v2 is a Rust backup service with an embedded bilingual Web UI. It backs up directories to any storage supported by rclone, keeps run history in SQLite, and stays online while storage is being configured.

## Highlights

- Directory backups only, with multiple source directories and destinations.
- Standard archives that are easy to restore:
  - **7z**: recommended; an optional password enables AES-256 and filename encryption.
  - **ZIP**: optional password uses broadly compatible ZipCrypto, with a security warning in the UI.
  - **Native directory**: no archive; restore through rclone.
- Friendly daily, weekly, monthly, and interval schedules with an advanced Cron mode, IANA timezones, configurable retries, retention by age/count, and shared global notifications.
- Storage account wizard backed by rclone's provider schema. Every account can be tested.
- Chinese/English, light/dark/system themes, and a responsive mobile layout.
- Public authentication is off by default. Optional Basic Auth is available through environment variables.
- OpenAPI at `/api/openapi.json` and API help at `/api/docs`.

Provider credentials and rclone passwords are written only to `/config/rclone/rclone.conf`. Backup-plan documents and global notification settings, including archive and delivery secrets, are AES-256-GCM encrypted before SQLite storage. The local encryption key is `/config/.rclone-backup.key` by default.

## Start

```bash
docker run -d \
  --name rclone-backup \
  --restart unless-stopped \
  -p 127.0.0.1:8080:8080 \
  -v rclone-backup-data:/config \
  -v /path/to/backup:/data:ro \
  czyt/rclone-backup:2.2.2
```

Open `http://127.0.0.1:8080`. On a fresh installation the service displays the storage wizard and remains running. Scheduled and manual backups stay locked until at least one working rclone alias exists.

The included [`docker-compose.yml`](docker-compose.yml) is a minimal Compose example.

## Restore

For 7z or ZIP plans, download the archive from the destination and open it with a compatible archive application. One-source archives extract directly to that directory's contents. Multi-source archives contain one clearly named top-level directory per source.

Examples:

```bash
7z x backup-20260813.7z
unzip backup-20260813.zip
```

For native-directory plans or advanced `crypt`/`compress` aliases, restore through the configured rclone chain:

```bash
rclone copy MyAlias:/RcloneBackup/backup-20260813 ./restore
```

`rclone crypt` may require its main password, optional salt/password2, and filename/directory encryption settings. Back up `rclone.conf`; a crypt remote cannot be reconstructed from only one archive password. `rclone compress` has no password of its own.

## Storage accounts

The Web UI gets provider options from the bundled rclone API and writes credentials directly to rclone's configuration file. The application database stores only remote aliases referenced by plans. The remote editor returns non-secret configuration values and credential identifiers such as usernames or access-key IDs; passwords, secret keys, and tokens are never returned, only their configured state.

Each plan has separate limits for destination checks and uploads. Destination checks default to 4 concurrent jobs (range 1–32); uploads default to serial execution and can be set from 1–8. For roughly ten destinations, start with 2–3 uploads and tune from observed disk, memory, and network usage. Native rclone tuning such as `--transfers`, `--checkers`, `--buffer-size`, and retry flags remains available through the plan's global rclone flags field.

The private rclone RC service listens only on `127.0.0.1:5572`, uses random per-process credentials, and is not exposed by the public API. The Web server invokes allow-listed rclone operations through this private API.

For providers with browser OAuth, you can also configure rclone using the existing container command path and refresh the UI:

```bash
docker exec -it rclone-backup rclone config
```

## Notifications

The Web UI has one global notification module shared by every manual and scheduled backup. Ping, Email, ServerChan, and ntfy targets can be added more than once, each with start, success, and failure event choices and an independent test action. Notification secrets are encrypted in SQLite and masked in API responses.

The separate **Notification templates** module stores reusable plain-text message templates in the same encrypted SQLite settings document. Create or duplicate a template, customize its start, success, and failure titles and bodies, then select it from any notification target. Multiple targets can share one template, and a template in use cannot be deleted until those targets select another one. Existing targets use the immutable built-in English template by default.

Each custom template has a Chinese or English language setting. It controls localized event names, runner-generated `{{content}}`, and unavailable-value labels; user-authored title and body text are never translated automatically. Templates support `{{plan_name}}`, `{{event}}`, `{{content}}`, `{{time}}`, and `{{backup_size}}`. The content value is generated by the backup runner for the current event, while time and backup size are provided separately. Backup size is unavailable before an archive or native-directory payload exists. Placeholders are expanded once at delivery time; conditions, HTML, environment variables, and credential interpolation are not supported.

Every backup run has a 24-hour safety timeout covering destination checks, preparation, uploads, retry waits, and retention. Active runs can be cancelled from the run-log view; cancellation stops local preparation and active rclone jobs, and incomplete history left by a service restart is marked as interrupted on the next service startup.

Email targets use standard SMTP fields for host, port, STARTTLS or TLS, sender, credentials, and recipient. Delivery uses the native Rust `lettre` mail client rather than invoking curl. Direct SMTP connections and HTTP notification endpoints are pinned to the public IP addresses resolved during validation to prevent internal-network requests and DNS rebinding. The legacy `MAIL_SMTP_VARIABLES` environment variable remains supported only for the first environment import and is converted into the standard fields.

When upgrading, identical notification settings from old plans are migrated automatically. If plans contain different settings, notifications stay disabled until an administrator selects one of the listed source plans or saves a new global configuration. The old values remain in encrypted plan documents for rollback, but runtime delivery only uses the confirmed global configuration.

## Optional Web authentication

Authentication is disabled by default. Set both values to enable Basic Auth; a partial configuration stops startup:

```yaml
environment:
  RCLONE_BACKUP_USER: admin
  RCLONE_BACKUP_PASSWORD_FILE: /run/secrets/web-password
```

Bind the default port to loopback or put the service behind a trusted HTTPS reverse proxy. Do not expose the unauthenticated listener to an untrusted network.

## Environment migration

Existing v1 variables, their numbered variants, `/.env`, and `_FILE` secrets are supported as a one-time import. Import happens only when the v2 database has not been initialized. After the marker is written, Web UI/API/SQLite are the source of truth and later environment changes do not overwrite plans.

Common imported settings include:

| Legacy variable | v2 destination |
| --- | --- |
| `DISPLAY_NAME`, `CRON`, `TIMEZONE` | Plan name and schedule |
| `BACKUP_FOLDER_NAME[_N]`, `BACKUP_FOLDER_PATH[_N]` | Directory sources |
| `RCLONE_REMOTE_NAME[_N]`, `RCLONE_REMOTE_DIR[_N]` | Alias references and paths |
| `ZIP_ENABLE`, `ZIP_TYPE`, `ZIP_PASSWORD` | Archive format and password |
| `BACKUP_FILE_*`, `BACKUP_KEEP_*` | Filename and retention |
| `PING_*`, `MAIL_*`, `SERVERCHAN_*` | Initial global notification candidate |

Database-export variables from v1 are intentionally ignored because v2 backs up directories only. Mount a database-generated export directory as a source if another tool produces dumps.

Multiple complete plans can be seeded once with `RCLONE_BACKUP_PLANS` as a JSON array of plan inputs.

## v2 environment variables

| Variable | Default | Purpose |
| --- | --- | --- |
| `RCLONE_BACKUP_SITE_NAME` | `Rclone Backup` | Browser title and site name shown in the Web UI |
| `RCLONE_BACKUP_ADDR` | `0.0.0.0:8080` | Public Web listener |
| `RCLONE_BACKUP_DATABASE_URL` | `sqlite:///config/rclone-backup.db?mode=rwc` | Application SQLite |
| `RCLONE_BACKUP_WORK_DIR` | `/tmp/rclone-backup` | Temporary workspace root; managed runs use a private `.runs` subdirectory and the root must be outside every source directory |
| `RCLONE_CONFIG` | `/config/rclone/rclone.conf` | rclone account configuration |
| `RCLONE_BACKUP_KEY_FILE` | `/config/.rclone-backup.key` | Plan-encryption key path |
| `RCLONE_BACKUP_SECRET_KEY[_FILE]` | unset | Explicit 32-byte unpadded-base64 plan key |
| `RCLONE_BACKUP_USER` | unset | Optional Basic Auth user |
| `RCLONE_BACKUP_PASSWORD[_FILE]` | unset | Optional Basic Auth password |
| `RUST_LOG` | application info | Rust log filter |

Keep the `/config` volume backed up. It contains rclone accounts, backup plans, run history, and the plan-encryption key.

## Development

Requires stable Rust. Runtime backup execution also needs `rclone`, `7z`, `cp`, `curl`, and `mail` for the corresponding features.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo run -- serve
```

The container image bundles rclone 1.75.0. Releases publish `linux/amd64` and `linux/arm64` images to Docker Hub and GHCR.

## License

[MIT](LICENSE)
