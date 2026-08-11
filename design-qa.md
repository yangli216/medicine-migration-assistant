# Design QA — 迁移历史日志左侧列表

- Source visual truth: `/var/folders/tq/31b1_m3x7934pqwkcfc3qhcc0000gp/T/codex-clipboard-bdf2e743-c5ad-43e7-bc5a-d75abe0d7ad2.png`
- Implementation screenshot: `/Users/yangl/ClaudeSpace/medicine-migration-assistant/history-layout-after.png`
- Source pixels: 1324 × 866
- Implementation pixels: 1324 × 866
- CSS viewport: 1339 × 876; device pixel ratio: 1
- Density normalization: browser content capture was aligned to the source at 1324 × 866, so no resampling was required.
- State: history modal open, a batch selected, overview tab visible. The source contains 17 persisted desktop batches while the browser verification uses one generated preview batch; the compared component anatomy and selected state are equivalent.

## Full-view comparison evidence

The modal frame, two-column hierarchy, compact healthcare palette, header, filters, selected-card treatment, detail summary cards, tabs, and overview table preserve the existing visual system shown in the source. The implementation intentionally changes only the left list's width containment. No new visual assets were introduced.

## Focused region comparison evidence

The left library is the fidelity-critical region for this task. In the source, the list exposes a horizontal scrollbar at the bottom. In the implementation, the list reports `clientWidth = 315px` and `scrollWidth = 315px`; its selected card reports `clientWidth = 313px` and `scrollWidth = 313px`. Computed overflow is `overflow-x: hidden` and `overflow-y: auto`, so horizontal travel is removed while vertical browsing remains available. At the narrower desktop check, the list also reports equal client and scroll widths (`275px`).

## Required fidelity surfaces

- Fonts and typography: existing Chinese system-font stack, sizes, weights, single-line title truncation, and small metadata hierarchy are preserved. Batch titles now have an explicit shrinkable flex track; status text remains on one line.
- Spacing and layout rhythm: existing 340px/300px desktop sidebar tracks, 12px inset, 7px card gaps, radii, borders, and selected-state inset accent are unchanged. Only width containment and wrapping safeguards were added.
- Colors and visual tokens: no color, border, shadow, or semantic status token changed.
- Image quality and asset fidelity: the screen contains no task-specific raster assets; existing Phosphor icons remain unchanged.
- Copy and content: no user-facing copy changed. The browser batch data differs from the persisted desktop screenshot only because the verification environment uses generated preview data.

## Findings

No actionable P0, P1, or P2 findings remain.

## Comparison history

- Earlier P2: the source screenshot showed a horizontal scrollbar in the left history list, indicating that list/card descendants could exceed the fixed sidebar width.
- Fix: added zero-min-width constraints to the sidebar, filter grid, list, cards, title row, title text, metadata, and count row; constrained cards to the available width; hid horizontal overflow only on the list; allowed count chips to wrap; kept status pills non-shrinking and safely truncated.
- Post-fix evidence: equal client/scroll widths at both 315px and 275px list widths, no browser console warnings or errors, successful search empty-state recovery, and successful batch selection/detail display.

## Implementation checklist

- [x] Remove the left list's horizontal scrolling.
- [x] Preserve vertical scrolling.
- [x] Keep long titles and statuses within the card width.
- [x] Keep large count groups from widening cards.
- [x] Verify search empty/recovery states and batch selection.
- [x] Verify at the source-sized and narrower desktop viewports.

## Follow-up polish

None required for the requested scope.

final result: passed
