# Phase A — Audit every `dirIcon` / navigation-icon call site

## Status: pending

Goal: confirm the Setup Wizard "التالي" button is the *only* currently-broken call site, and that no
other button (AppButton or bare markup) has made the same DOM-order mistake. This is a reading task —
most files should need no change. Don't fix anything that isn't actually broken.

## Tasks

- [ ] Re-grep `dirIcon\.(next|forward|open|back|prev)` across `src/` (confirm the list below is still
      current — files move).
- [ ] For each `AppButton` call site with a visible label (slot content) **and** an `icon` prop using
      `dirIcon.next`/`.forward`/`.open`: confirm the icon currently renders leading (it will, since
      that's `AppButton`'s only behavior today) and flag it as needing `icon-position="end"` in Phase B.
      Known case: [`SetupWizardPage.vue:209`](../../../src/modules/setup/pages/SetupWizardPage.vue#L209).
  - [ ] Re-check there isn't a second one introduced since this plan was written (e.g. a wizard-like
        flow in `plans/pending/23-subscription-platform` work in progress, or anywhere under
        `apps/dashboard` that reuses this `AppButton` — check if `apps/dashboard` has its own copy of
        `AppButton` or imports the desktop one; if it has its own copy, note it as a **separate**
        follow-up, not silently fixed here, since this plan's scope is the desktop app).
- [ ] For each `AppButton` call site with `dirIcon.back`/`.prev` and a visible label: confirm it's
      correct as-is (icon leading = correct for back). Known cases: `VoucherPrintPage.vue:47`,
      `PosPage.vue:582`, `InvoicePrintPage.vue:105`, `PurchasePrintPage.vue:48`,
      `SetupWizardPage.vue:206`, `DevUiPage.vue:425`. No change expected.
- [ ] For each **icon-only** `AppButton` (no slot content, `aria-label` instead) using any `dirIcon`:
      confirm no fix needed — there's no label to be on the wrong side of. Known cases:
      `PdfPreview.vue:47,51` (page nav), and any `ZoomIn`/`ZoomOut` buttons (not `dirIcon`, unaffected).
- [ ] For each **bare `<button>` + `<DirIcon>`** (not `AppButton`) call site, confirm the surrounding
      markup places the icon on the correct side of any adjacent label in its own template (these don't
      go through `AppButton`'s DOM order, so each is independent):
  - [ ] `DataTable.vue:418-436` (pager prev/next, icon-only, no label — unaffected)
  - [ ] `DataTable.vue:435` expand chevron (`dirIcon.open`, rotates in place — unaffected, not a
        leading/trailing position question)
  - [ ] `JournalListPage.vue:421`, `ChartOfAccountsPage.vue:274` (expand/collapse chevrons — same,
        rotation not position)
  - [ ] `SetupChecklistCard.vue:42`, `KpiCard.vue:34` (trailing decorative chevron already placed after
        text in template — confirm, don't assume)
  - [ ] `DevUiPage.vue:414,418` (gallery pager, icon-only — unaffected)
- [ ] Write the final list of "needs fix" (should be exactly 1 file) into Phase B's task list before
      starting Phase B — if the audit finds more than the known case, update Phase B's checklist first.

## Gate

- [ ] Audit list above fully checked off, findings (if any beyond the known case) written into
      Phase B before that phase starts.
