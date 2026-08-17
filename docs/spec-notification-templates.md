# Spec: Notification template library

## Objective

Add an independent notification-template management module. Administrators can
create reusable message templates, then choose one template for each configured
Ping, Email, ServerChan, or ntfy target. Editing one template updates every
target that references it. Existing targets keep the current English messages
through an immutable built-in default template.

## Tech stack

- Rust 2024, Axum 0.8, Tokio, Serde
- Existing AES-256-GCM encrypted global settings document in SQLite
- Embedded HTML, CSS, and plain JavaScript, with no new dependency
- A small built-in placeholder renderer, not a general-purpose template engine

## Commands

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
node --check web/app.js
git diff --check
```

## Project structure

- `crates/core/src/model.rs`: template models, target references, and validation
- `crates/notifications/src/lib.rs`: template selection, rendering, and delivery
- `crates/store/src/lib.rs`: existing encrypted settings persistence
- `crates/api/src/lib.rs`: template-library update API and validation
- `crates/runner/src/lib.rs`: supplies event values to rendering
- `web/index.html`, `web/app.js`, `web/app.css`: template manager and selector UI
- `README.md`: template workflow and placeholder reference

## Data and persistence contract

`NotificationConfig` gains a defaulted `templates: Vec<NotificationTemplate>`.
Each custom template contains a stable ID, name, and title/body pairs for the
`start`, `success`, and `failure` events. `NotificationTarget` gains a defaulted
optional `template_id`. A missing or empty reference selects the built-in
default template, so v2.0.9 documents deserialize without migration.

Templates and target references are serialized inside the existing global
notification settings document. The store continues to encrypt that complete
document with AES-256-GCM before writing it to SQLite. Nothing is stored in
browser local storage or environment variables, and no plaintext schema column
is added.

The dedicated template update endpoint replaces only the template library in
that aggregate and validates all references before one atomic save. It does not
re-resolve unchanged notification network targets or change the notification
confirmation state.

The built-in default template is immutable and implicit, rather than duplicated
in every database document:

```text
Start title:   {{plan_name}} Backup Start
Success title: {{plan_name}} Backup Success
Failure title: {{plan_name}} Backup Failed
All bodies:    {{content}}
```

At most 32 custom templates are allowed. IDs use the same safe stable format as
notification target IDs. Names are required and limited to 80 characters.
Template names do not need to be unique because selectors also preserve stable
IDs. A custom template referenced by any target cannot be deleted until those
targets select another template. A missing non-empty reference is rejected on
save and delivery.

## Rendering contract

Each event title and body supports these literal placeholders:

- `{{plan_name}}`: backup plan name
- `{{event}}`: stable value `start`, `success`, or `failure`
- `{{content}}`: the event detail currently produced by the runner

No conditions, loops, includes, HTML interpretation, environment-variable
access, or secret interpolation are supported. Unknown or malformed
placeholders are rejected before persistence. Rendered messages are plain text.
Each title is limited to 200 characters, each body to 8,000 characters, and the
complete template library to 64 KiB.

Delivery resolves the selected template separately for every target before
sending it. The target test action uses that target's selected template, plan
name `Rclone Backup Test`, success event, and content
`Notification test from Rclone Backup`.

## Interface direction

The existing app shell gains a dedicated `Templates` / `通知模板` navigation
entry beside Notifications. This is a focused management surface, not six more
fields inside every channel card.

The template page uses a quiet master-detail layout:

- A compact template list shows name, customized event count, and how many
  channels reference it.
- The built-in default appears first with a lock indicator and can be previewed
  or duplicated, but not edited or deleted.
- Selecting a custom template opens one editor with Start, Success, and Failure
  event tabs. Each tab has title and multiline body fields, placeholder chips,
  and an escaped plain-text preview.
- Creation starts by duplicating the default, so valid useful content is present
  immediately. Duplicate and delete are explicit secondary actions.
- If deletion is blocked, the UI names the referencing targets and links the
  user back to Notifications instead of silently changing them.

Each expanded notification-target editor gains one required Template selector.
It lists the built-in default first and then custom templates. The target summary
shows the selected template name, making shared behavior visible without opening
the editor.

The visual direction remains the current utility-first app: existing type,
colors, border radii, spacing tokens, and surface hierarchy are reused. Template
selection is immediate. Master-detail selection and preview changes use short,
interruptible opacity/transform transitions under 200 ms, do not delay keyboard
actions, and respect `prefers-reduced-motion`. Full-width and 375 px layouts,
Chinese and English labels, focus order, and contrast are release checks.

## Code style

Use typed defaulted Serde models, stable ID references, explicit referential
validation, and deterministic dependency-free rendering.

```rust
let template = config.template_for(target.template_id.as_deref())?;
let message = template.render(plan_name, event, content)?;
deliver_target(target, &message).await;
```

## Testing strategy

- Core unit tests cover v2.0.9 compatibility, ID/reference validation, size
  limits, blocked deletion prerequisites, malformed placeholders, and unknown
  placeholders.
- Notification unit tests cover built-in fallback, per-target selection,
  multiple targets choosing different templates, repeated placeholders, and
  Unicode rendering.
- Store tests prove template libraries and references round-trip inside the
  encrypted settings document.
- API tests cover invalid references and preserve the existing secret-redaction
  behavior.
- Frontend checks cover template CRUD, duplication, reference counts, blocked
  deletion guidance, target selection, keyboard use, and bilingual responsive
  layouts.

## Boundaries

- Always: encrypt templates and references at rest; escape values in the UI;
  validate before persistence and delivery; preserve built-in defaults; prevent
  dangling references.
- Ask first: add a third-party template engine; add HTML email; expose more
  runtime or secret fields; silently reassign channels during deletion.
- Never: evaluate code or shell syntax; read environment variables from a
  template; interpolate notification credentials; accept unknown placeholders;
  persist templates in browser storage.

## Success criteria

- A dedicated bilingual template module can list, create, duplicate, edit,
  preview, and safely delete reusable templates.
- Each Ping, Email, ServerChan, and ntfy target can select one template, and the
  selected name is visible in its summary.
- Editing a template affects every referencing target on its next delivery.
- Custom templates and target references survive reload through encrypted
  SQLite persistence.
- Existing saved targets and targets selecting the built-in default produce
  byte-for-byte equivalent titles and bodies to v2.0.9.
- Invalid templates, dangling references, and deletion of referenced templates
  produce clear errors and are not persisted.
- All test and release gates pass at desktop and 375 px widths.
- All workspace package versions move from `2.0.9` to `2.1.0`; the change is
  committed, tag `v2.1.0` is created, and the branch plus tag are pushed to
  `origin`.

## Open questions

None, provided the independent template library, per-target selection model,
dedicated navigation entry, and `v2.1.0` release are approved.
