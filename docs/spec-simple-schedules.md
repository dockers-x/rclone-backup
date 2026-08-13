# Spec: Simple schedules

## Objective

Let administrators configure common backup schedules without writing Cron while
retaining the existing Cron field for advanced and migrated plans. Simple
schedules compile to the existing schedule string, so the API, database, and
scheduler remain backward compatible.

## Tech stack and commands

- Embedded HTML, CSS, and plain JavaScript over the existing Rust/Axum service.
- Verify with `cargo fmt --all -- --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo test --all-targets --all-features`, `node --check web/app.js`,
  `actionlint`, and `git diff --check`.

## Interaction and mapping

- Schedule modes: Simple (default for recognized rules) and Cron.
- Simple types:
  - Daily at `HH:MM` -> `0 MM HH * * *`
  - Weekly on a selected weekday at `HH:MM` -> `0 MM HH * * DAY`
  - Monthly on day 1-31 at `HH:MM` -> `0 MM HH DAY * *`
  - Every N seconds, 1-59 -> `0/N * * * * *`
  - Every N minutes, 1-59 -> `0 0/N * * * *`
  - Every N hours, 1-23 -> `0 0 0/N * * *`
- Interval rules align to natural clock boundaries in the plan timezone.
- The scheduler checks once per second and skips schedule ticks while the same
  plan is still running, so high-frequency rules never overlap or build a
  backlog of catch-up runs.
- Automatic runs are capped at two concurrent plans; due ticks are skipped when
  that capacity is full rather than queued. Cron input is capped at 256 bytes.
- Monthly dates absent from a month are skipped.
- Existing five-, six-, or seven-field Cron that cannot be recognized exactly
  stays in Cron mode and is never rewritten by merely opening the editor.

## UI and style

- Match the current warm, flat, border-led app shell.
- Use the existing plain-CSS strategy and 44px controls.
- Reveal only fields needed by the selected simple type and show a readable
  summary plus the generated Cron expression.
- Keep the timezone adjacent to both modes because all rules use it.

## Testing strategy

- Rust scheduler tests prove every generated expression parses and fires in the
  expected timezone slot.
- JavaScript syntax and browser checks cover recognition, generation, editing,
  conditional inputs, bilingual copy, and 375px layout.

## Boundaries

- Always: preserve an unrecognized existing Cron expression byte-for-byte.
- Ask first: change interval alignment or add calendar exceptions.
- Never: add a second persisted scheduling model or a frontend dependency.

## Success criteria

- Users can configure daily, weekly, monthly, and N-second/minute/hour rules
  without knowing Cron.
- Saved plans continue to contain a valid schedule string accepted by the
  existing backend parser.
- Cron mode remains available and compatible with every current plan.

## Open questions

None. Interval limits and alignment follow the assumptions stated during
implementation.
