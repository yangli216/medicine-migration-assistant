# Design QA

## Comparison target

- Source visual truth: `/Users/yangl/.codex/generated_images/019fc5df-924a-76d2-913a-44436dc6e468/exec-dab41673-dc55-4ff7-bfdb-5132afeb10c6.png`
- Browser-rendered implementation: `/Users/yangl/.codex/.chatgpt-projects/g-p-69f1adb0b3708191ab2d7b1fa8819ad3/guided-migration-prototype/implementation-final.png`
- Full-view combined comparison: `/Users/yangl/.codex/.chatgpt-projects/g-p-69f1adb0b3708191ab2d7b1fa8819ad3/guided-migration-prototype/design-comparison-final.png`
- Focused stepper comparison: `/Users/yangl/.codex/.chatgpt-projects/g-p-69f1adb0b3708191ab2d7b1fa8819ad3/guided-migration-prototype/comparison-focus-stepper.png`
- Focused mapping-workspace comparison: `/Users/yangl/.codex/.chatgpt-projects/g-p-69f1adb0b3708191ab2d7b1fa8819ad3/guided-migration-prototype/comparison-focus-workspace.png`
- Additional interaction evidence: `/Users/yangl/.codex/.chatgpt-projects/g-p-69f1adb0b3708191ab2d7b1fa8819ad3/guided-migration-prototype/implementation-issue-dialog.png`
- State: initial guided mapping screen with `DRUG_CODE` selected.

## Viewport and normalization

- Source pixels: 1487 × 1058.
- Implementation pixels: 1440 × 1024.
- CSS viewport: 1440 × 1024.
- Device scale factor: 1.
- Density normalization: source was proportionally normalized to 1440 × 1024 before side-by-side comparison; implementation was captured natively at 1440 × 1024.
- Additional responsive check: 1280 × 800. The fixed primary action remained visible, with no horizontal overflow.

## Browser verification

- Local route opened successfully in the Codex in-app browser.
- Primary interactions tested:
  - select an alternate suggested source field;
  - open and resolve the data-quality dialog;
  - open and close professional mapping mode;
  - confirm the current mapping and advance to the next core field;
  - return to the prior field;
  - verify progress and recommendation state updates.
- Console errors and warnings checked after initial render and after interactions: none.
- Layout check at 1440 × 1024: body and viewport both 1440 × 1024; fixed footer and primary controls remain visible; no page overflow.
- Production build passed.
- Sites packaging tests passed: 4/4.

## Full-view comparison evidence

- Overall composition matches the selected direction: thin product header, five-step guided progress, large single-decision workspace, right-aligned completion indicator, intelligent suggestion rows, inline quality warning, preview table, and fixed action bar.
- Major-region proportions, viewport fill, horizontal margins, workspace width, and footer placement are aligned with the source.
- The implementation intentionally localizes the generated English product name to `数据迁移助手`; this is a product-copy adaptation, not visual drift.

## Focused comparison evidence

- Stepper: numbered nodes, completed check indicators, active teal step, connector spacing, and five-stage hierarchy now match the source anatomy.
- Mapping workspace: heading hierarchy, source-table context, professional-mode control, three recommendation rows, selected-row treatment, warning strip, and preview density match the source closely.
- Interaction state: the quality dialog uses the currently selected source field and no longer shows `DRUG_CODE`-specific guidance after a different suggestion is selected.

## Required fidelity surfaces

- Fonts and typography: uses PingFang SC with Microsoft YaHei and Noto Sans CJK SC fallbacks. Heading, body, metadata, and database-field hierarchy match the source. Remaining cross-renderer weight differences are minor.
- Spacing and layout rhythm: frame size, 28 px outer margin, section sequence, table rhythm, border radii, and fixed footer align with the source. No actionable overflow remains.
- Colors and visual tokens: pale-mint base, white work surface, navy text, teal primary/selected states, blue secondary confidence, and amber warning treatment match the source.
- Image quality and asset fidelity: the source contains no photography or illustration. Product and action icons use Phosphor Icons rather than hand-drawn assets; rendering is sharp at device scale factor 1.
- Copy and content: visible content uses realistic Chinese drug-migration data and preserves the selected screen's intent. Labels, samples, progress, and CTA wording fit without clipping.

## Findings

- No actionable P0, P1, or P2 findings remain.

## Comparison history

1. Initial attempt was blocked because the in-app browser security policy could not be verified; no implementation screenshot was available.
2. First successful browser comparison found three P2 issues: completed steps lost their numbered nodes, an extra field counter displaced the question hierarchy, and the page had 6 px of vertical overflow. Fixes: restored numbered nodes plus separate completion checks, removed the extra field counter while rebalancing heading spacing, and reduced shell bottom padding to fit the target viewport. Post-fix evidence: `implementation-02.png`, focused stepper comparison, and a 1440 × 1024 layout measurement with no overflow.
3. Interaction testing found one P2 issue: selecting `YPDM` still opened `DRUG_CODE`-specific quality guidance. Fix: derived warning and dialog copy from the active suggestion and added a generic resolution state for alternate fields. Post-fix evidence: `implementation-issue-dialog.png` and successful dialog text verification for `YPDM`.
4. Final comparison found no actionable P0/P1/P2 differences.

## Implementation checklist

- [x] Match the selected visual structure and hierarchy.
- [x] Test the main guided mapping path.
- [x] Test duplicate/quality issue handling.
- [x] Test professional-mode escape hatch.
- [x] Check console errors.
- [x] Verify the target viewport and a smaller desktop viewport.
- [x] Run production build and packaging checks.

## Follow-up polish

- [P3] The localized Chinese product title and standard medical-kit icon differ from the generated mock's English title and abstract cross mark; they are intentional product-fit substitutions.
- [P3] Small helper and warning text may render one optical weight lighter than the generated image because the browser uses the local Chinese system font.

final result: passed
