# Global notification implementation plan

1. Add validated global notification models, channel/event controls, and a
   migration-conflict representation.
2. Add an encrypted singleton settings table and idempotent migration from the
   notification data retained in legacy plan documents.
3. Switch Runner and CLI notification tests to a confirmed global snapshot.
4. Add masked GET/PUT/test APIs with redacted-secret merging and channel tests.
5. Build an independent bilingual notification section and remove notification
   fields from the backup-plan form.
6. Verify store/API/runner behavior, frontend syntax and responsive interaction,
   then run the full release gates.

## Risks and mitigation

- Legacy conflicts: never activate automatically; retain source labels and require
  an explicit selection.
- Secret leakage: encrypt the complete singleton document, mask response values,
  and keep command/error logging free of arguments and URLs.
- Notification SSRF: reject every non-public resolved address and pin the same
  validated IP set into curl, with proxies and redirects disabled.
- Mid-run configuration changes: Runner takes one snapshot at run start so event
  delivery is consistent throughout that run.
- Plain-HTML complexity: reuse the existing form, toast, navigation, and request
  helpers; do not introduce a framework for one settings module.

## Checkpoints

- Store migration and encryption tests pass before Runner integration.
- API masking and update tests pass before UI integration.
- Browser validation covers 375px and desktop, both themes, keyboard focus, and
  reduced motion before release.
