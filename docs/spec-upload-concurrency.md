# Spec: bounded multi-destination uploads

## Objective

Allow a backup plan to upload its single prepared archive or staging directory to several rclone destinations concurrently. Existing plans and new plans remain serial by default. Users with many destinations can opt into a bounded value without accepting hidden transfer, memory, or retry defaults.

## Tech stack

- Rust 2024, Tokio `JoinSet`, the existing private rclone RC API, Serde models, and encrypted plan storage.
- Native HTML/CSS/JavaScript with the locally embedded sashimi UI CSS and Lucide SVG subset.
- No new runtime or build dependency.

## Commands

- Format: `cargo fmt --all -- --check`
- Lint: `cargo clippy --all-targets --all-features -- -D warnings`
- Test: `cargo test --all-targets --all-features`
- Frontend syntax: `node --check web/app.js`
- Local review: `cargo run -- serve`, followed by agent-browser desktop and 375px checks.

## Project structure

- `crates/core/src/model.rs`: plan field, defaults, bounds, and compatibility tests.
- `crates/runner/src/lib.rs`: bounded upload queue and truthful target checkpoints.
- `web/`: bilingual plan control and runtime state presentation.
- `docs/`: durable behavior and acceptance criteria.

## Code style

Use the existing typed plan model and Tokio task style. Start work only when a slot is available, and update the log checkpoint at the same boundary:

```rust
while uploads.len() < concurrency {
    let Some(remote) = pending.next() else { break };
    start_remote_upload(&mut uploads, remote, /* existing values */);
}
```

Do not use shell execution, detached tasks, global mutable state, or error-string retry classification.

## Testing strategy

- Model tests prove old JSON defaults to `1` and values outside `1..=8` fail validation.
- Runner tests cover the queue helper/state events where practical; the full workspace suite protects existing archive, notification, and retry behavior.
- Frontend syntax, axe WCAG A/AA, desktop, mobile, dark theme, and reduced-motion checks run before release.

## Boundaries

- Always: default to `1`, checkpoint pending/active/completed states, wait for all started uploads, aggregate destination failures, preserve one-archive-many-destinations behavior.
- Ask first: changing the default above `1`, adding byte/speed metrics, setting automatic rclone transfer flags, or introducing a total upload timeout.
- Never: hard-code provider-specific throughput flags, cancel healthy sibling uploads after one failure, pass arguments through a shell, or treat a queued target as active.

## Success criteria

- Existing serialized plans load with `upload_concurrency = 1`.
- API and UI accept only `1..=8` and preserve the chosen value.
- At most the configured number of `copy` jobs are active simultaneously.
- Targets without a slot remain `pending`; only active jobs become `uploading`.
- A failed destination does not cancel sibling jobs, and the final run reports the complete failure set.
- Default value `1` retains serial upload behavior.
- All project and browser release gates pass.

## Open questions

None for this release. Byte-level progress and adaptive stall detection remain a separate feature.
