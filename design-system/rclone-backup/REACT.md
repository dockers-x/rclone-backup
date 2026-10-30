# React workspace design, v2.3.0

This document supersedes MASTER.md for the rewritten React interface.

- Purpose: a compact backup operations workspace for managing schedules, destinations, notifications and runs.
- Worlds: use upstream Ovio Minimal, Craft, Retro and Toy. Default Toy; remember the selection independently of language and light/dark/system theme.
- Palette: preserve the Ovio world materials. Toy uses warm molded plastic, blue action keys and a red tab indicator; Craft uses paper and warm brown ink; Retro uses a dark green terminal; Minimal uses restrained neutral surfaces. Application status colors always accompany text.
- Typography: local system sans for Minimal/Toy, serif for Craft, monospace for Retro. Paths, cron and logs use monospace. No remote fonts.
- Components: Ovio Liquid Tabs, Ovio Toast and ToyKey; Radix Dialog, AlertDialog and Switch for accessible behavior. Use native inputs/selects for long provider lists and form validation.
- Layout: 240px navigation, compact breadcrumb bar, page title and action, status metrics then working content. No promotional hero. Sidebar becomes an accessible drawer under 900px. Stack forms/cards under 600px.
- Depth: one surface shadow per panel; physical key depth only in Toy; no blur or decorative ambient gradients.
- Motion: Ovio springs for pointer-driven tabs and keys, 160ms press feedback, 220ms modal entry, 250ms narrow-screen sheet. Stable metrics and polling rows. Keyboard actions resolve immediately. Respect reduced-motion and gate hover motion to fine pointers.
- Accessibility: visible focus across Ovio and Radix, meaningful form labels, textual status, no close-on-outside-click data loss, deep links and browser history. Localize application and vendored labels in Chinese and English.
