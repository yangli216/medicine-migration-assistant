# Prototype Instructions

Run the local server yourself and open the preview in the browser available to this environment. Do not give the user server-start instructions when you can run it.

Before making substantial visual changes, use the Product Design plugin's `get-context` skill when the visual source is unclear or no longer matches the current goal. When the user gives durable prototype-specific design feedback, preferences, or decisions, record them in `AGENTS.md`.

When implementing from a selected generated mock, treat that image as the source of truth for layout, component anatomy, density, spacing, color, typography, visible content, and hierarchy.

Build app UI in `src/`. Keep `.openai/hosting.json`, `worker/index.js`, `scripts/prepare-sites-build.mjs`, and `tests/sites-worker.test.mjs` intact so the same local prototype can be handed to Sites. Before a Sites handoff, run `npm run build` and `npm run test:sites`; the build must leave `dist/client/index.html`, `dist/server/index.js`, and `dist/.openai/hosting.json`.

## Product direction

- The selected concept is the third generated direction: a guided migration assistant for non-expert integrators.
- Preserve the plain-language, one-decision-per-screen flow, mint/teal healthcare visual system, smart field recommendations, immediate sample preview, and optional expert-mode escape hatch.
- The core prototype path is: select a suggested legacy field, inspect or resolve the data-quality warning, confirm the mapping, and continue to the next required field.
- The production target is a lightweight Tauri desktop application, not a Java service or browser-only deployment.
- Rust owns source/target database access, row-level transactions, retry orchestration, local SQLite batch history, and audit records.
- Every primary key written to the target database must be a lowercase 24-character MongoDB ObjectId-compatible hex string, matching `new ObjectId().toString()`.
- Database profiles must support MySQL, Oracle, Dameng/DM8, openGauss/GaussDB and KingbaseES/PostgreSQL-family systems. Keep vendor-specific connection details behind the guided connection form instead of exposing dialect complexity to integrators.
