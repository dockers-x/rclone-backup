# React workspace, v2.3.0

The five application pages are React components built with Vite. The API and stored backup configuration remain compatible with v2.2.6. Compiled HTML, JS and CSS under `web/dist` are embedded in the Rust executable; neither Node nor an external asset CDN is required at runtime.

## Interface

- Backup plans: complete-history 24-hour metric, recent-run success rate, enabled/disabled filter, search, create/edit/delete, manual run, sources, multiple destinations, simple and advanced schedules, archive password preservation, retention, retries and concurrency settings.
- Storage accounts: all rclone providers, required and advanced options, provider examples, encrypted secret preservation, interactive configuration, editing, testing and deleting.
- Notifications: multiple targets of each supported type, template selection, event toggles, testing and saving. Polling must not overwrite a draft.
- Templates: built-in read-only English template, creating, duplicating, editing, reference-aware deletion, and localized live previews.
- History: latest 50 runs without full logs, details fetched on demand, sequential polling, cancellation, target progress and technical logs. Closed details must not accept late responses.

## Design

Use the upstream Ovio Liquid Tabs, Toast and ToyKey components. Offer Minimal, Craft, Retro and Toy worlds with Toy as the default. World selection and light/dark/system theme are independent and persist locally. Provide complete Chinese and English UI and persist language selection. Keep visual tokens synchronized with the selected world across the workspace, forms and portals. Maintain visible keyboard focus, native form validation, accessible dialogs, narrow-screen navigation, reduced-motion support and stable data displays.

## Verification and delivery

Run React interaction and data-compatibility tests, a production frontend build, Rust fmt/Clippy/tests, npm production audit and RustSec audit. Exercise the rendered UI at desktop and phone widths, all four worlds, both languages and dark/light themes. Verify a real local rclone backup. Commit generated assets with source; CI must rebuild them and detect drift. Docker builds the frontend before compiling Rust. Publish v2.3.0 through the existing two-registry, amd64/arm64 release workflow and read back its final state.
