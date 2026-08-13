# Global notification tasks

- [x] Add global settings and conflict-candidate models.
  - Acceptance: models validate channel/event combinations and URLs/options.
  - Verify: `cargo test model::tests`.
  - Files: `src/model.rs`.
- [x] Add encrypted singleton persistence and legacy migration.
  - Acceptance: identical legacy configs auto-confirm; conflicts stay inactive;
    stored documents begin with `enc:v1:`.
  - Verify: `cargo test store::tests`.
  - Files: `src/store.rs`.
- [x] Use the global snapshot for run notifications and CLI tests.
  - Acceptance: plans no longer drive runtime notification delivery.
  - Verify: Runner unit/integration tests and `cargo test runner::tests`.
  - Files: `src/runner.rs`, `src/main.rs`.
- [x] Add masked settings and test-notification APIs.
  - Acceptance: secrets are masked, `REDACTED` preserves stored secrets, invalid
    requests return 422, and tests never echo command arguments.
  - Verify: `cargo test api::tests`.
  - Files: `src/api.rs`.
- [x] Add the independent notification UI and remove per-plan fields.
  - Acceptance: bilingual, responsive, accessible save/test/conflict flows with
    visible pending and completion feedback.
  - Verify: `node --check web/app.js` plus browser checks.
  - Files: `web/index.html`, `web/app.js`, `web/app.css`.
- [x] Document, review, and run release gates.
  - Acceptance: docs describe global settings and all gates pass.
  - Verify: commands in `docs/spec-global-notifications.md`.
  - Files: `README.md`, `tasks/todo.md`.
