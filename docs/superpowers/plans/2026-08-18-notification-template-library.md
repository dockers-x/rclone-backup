# Notification Template Library Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a reusable database-backed notification template library and let every notification target select one template.

**Architecture:** Store custom templates and target template references in the existing encrypted `NotificationConfig` document, with an implicit immutable built-in template for backward compatibility. Resolve the selected template per target at delivery time. Expose the library as a dedicated SPA route while continuing to save the complete notification aggregate atomically through the existing API.

**Tech Stack:** Rust 2024, Serde, Axum, Tokio, SQLite/SQLx, plain JavaScript, HTML, CSS.

## Global Constraints

- No new Rust or frontend dependency.
- Existing v2.0.9 notification documents must deserialize and deliver unchanged.
- Custom templates and references must remain inside AES-256-GCM encrypted storage.
- Templates are plain text and support only `{{plan_name}}`, `{{event}}`, and `{{content}}`.
- A referenced template cannot be deleted or left dangling.
- The UI is bilingual, keyboard accessible, and verified at 1280 px and 375 px.
- Release version is `2.1.0`, tag is `v2.1.0`.

---

### Task 1: Template library model and validation

**Files:**
- Modify: `crates/core/src/model.rs`

**Interfaces:**
- Produces: `NotificationTemplate`, `NotificationEventTemplate`, `NotificationConfig::template_for`, and `NotificationTarget::template_id`.
- Consumes: existing `NotificationConfig::validate` and Serde defaults.

- [ ] Add a failing compatibility test that deserializes a v2.0.9 target without `template_id` or `templates` and asserts built-in fallback.
- [ ] Run `cargo test -p rclone-backup-core notification_template -- --nocapture` and confirm failure.
- [ ] Add defaulted template/reference fields and the immutable built-in effective values.
- [ ] Add failing tests for duplicate/invalid IDs, malformed or unknown placeholders, oversize values, dangling references, and referenced-template detection.
- [ ] Implement validation with maximum 32 templates, 80-character names, 200-character titles, 8,000-character bodies, and 64 KiB total library size.
- [ ] Run `cargo test -p rclone-backup-core notification_template -- --nocapture` and confirm all model tests pass.

### Task 2: Target-specific rendering and delivery

**Files:**
- Modify: `crates/notifications/src/lib.rs`
- Modify: `crates/runner/src/lib.rs`

**Interfaces:**
- Consumes: `NotificationConfig::template_for(template_id)` and event template models.
- Produces: deterministic plain-text rendering used by Ping, Email, ServerChan, and ntfy.

- [ ] Add failing notification tests for built-in output, Unicode custom output, repeated placeholders, and two targets selecting different templates.
- [ ] Run `cargo test -p rclone-backup-notifications -- --nocapture` and confirm failure.
- [ ] Resolve and render title/body inside the target loop before calling channel adapters.
- [ ] Keep `deliver(plan_name, config, event, content)` as the runner-facing seam and preserve test-notification inputs.
- [ ] Run notification and runner tests and confirm exact v2.0.9 default messages remain unchanged.

### Task 3: Aggregate API and encrypted persistence

**Files:**
- Modify: `crates/api/src/lib.rs`
- Modify: `crates/store/src/lib.rs`

**Interfaces:**
- Consumes: extended `NotificationConfig` through existing GET/PUT/test endpoints.
- Produces: atomic persistence of templates, references, targets, and existing secrets, plus a template-only update endpoint that preserves confirmation state.

- [ ] Add API tests that save templates and target references, reject dangling references, and preserve redacted credentials while templates change.
- [ ] Add a store test that reloads templates/references from an `enc:v1:` settings document.
- [ ] Run focused API/store tests and confirm they fail before implementation adjustments.
- [ ] Update redacted merge and migration behavior only where the new defaulted fields require it.
- [ ] Run `cargo test -p rclone-backup-api` and `cargo test -p rclone-backup-store`.

### Task 4: Dedicated template manager and target selector

**Files:**
- Modify: `crates/api/src/lib.rs`
- Modify: `web/index.html`
- Modify: `web/app.js`
- Modify: `web/app.css`

**Interfaces:**
- Consumes: `notifications.config.templates` and each target's optional `template_id`.
- Produces: `/templates` SPA route, CRUD/duplicate/preview interactions, and notification-target selectors.

- [ ] Add `/templates` to the frontend route allowlist and navigation.
- [ ] Add bilingual strings for template library, event editor, reference counts, validation, and blocked deletion.
- [ ] Render built-in default first, then custom templates in a master-detail surface.
- [ ] Implement create-from-default, duplicate, rename, edit, preview, and guarded delete using the existing notification aggregate save call.
- [ ] Add a template selector and selected-template summary to every notification target.
- [ ] Reuse existing tokens and radius scale; use explicit transform/opacity transitions under 200 ms and reduced-motion overrides.
- [ ] Run `node --check web/app.js`, frontend route tests, and `git diff --check`.

### Task 5: Documentation and release version

**Files:**
- Modify: `README.md`
- Modify: `docs/spec-notification-templates.md`
- Modify: `Cargo.toml`
- Modify: `crates/*/Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `tasks/plan.md`
- Modify: `tasks/todo.md`

**Interfaces:**
- Produces: user workflow, placeholder reference, completed task record, and version `2.1.0`.

- [ ] Document template creation, channel selection, placeholders, built-in fallback, and deletion protection.
- [ ] Update all workspace package versions to `2.1.0` and refresh the lockfile mechanically.
- [ ] Run `rg 'version = "2\\.0\\.9"' --glob 'Cargo.toml' --glob 'Cargo.lock'` and confirm no workspace package remains at the old version.

### Task 6: Runtime verification and release

**Files:**
- Verify all changed files.

**Interfaces:**
- Consumes: the completed feature.
- Produces: release evidence, commit, pushed branch, and pushed `v2.1.0` tag.

- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --all-targets --all-features`.
- [ ] Run `node --check web/app.js` and `git diff --check`.
- [ ] Start the app with an isolated temporary database/configuration and verify `/templates` at 1280 px and 375 px in Chinese and English.
- [ ] Verify template creation, selection, persistence after reload, preview, and blocked deletion in the real browser.
- [ ] Inspect `git diff` and confirm no unrelated files or secrets are included.
- [ ] Commit with `feat: add notification template library`.
- [ ] Create annotated tag `v2.1.0`, push `main`, and push the tag to `origin`.
