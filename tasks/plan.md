# Notification template library plan

1. Extend the encrypted notification aggregate with reusable templates and
   stable per-target references while preserving v2.0.9 defaults.
2. Resolve and render each target's chosen template at delivery time.
3. Add a dedicated bilingual template-management route and template selectors
   to notification targets.
4. Document the workflow, update every workspace crate to v2.1.0, and run all
   Rust, frontend, responsive-browser, and release checks.
5. Commit, tag v2.1.0, and push main plus the tag to origin.

## Risks and mitigation

- Dangling references: validate the whole aggregate atomically and block deletion
  while a target still uses the template.
- Backward compatibility: absent fields select an implicit built-in template
  whose output matches v2.0.9 byte for byte.
- Lost consistency during a run: keep the existing run-start configuration
  snapshot so one run never mixes template revisions.
- UI density: manage templates on a dedicated master-detail route and expose only
  a selector inside each notification target.
