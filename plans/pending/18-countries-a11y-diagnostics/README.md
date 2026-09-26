# 18 — Egypt + Saudi as real countries, address picker, phone/switch fixes, contrast, and a diagnostics system

**Status: in progress** (2026-09-26). A and B are done. F is mostly done (see its file for the two
open items: an automatic post-`mutate()` invariant watcher + `ACC-` issue stub, and the first real
`scripts/verify/cases/*.json` regression case, which only gets added the first time an actual
accounting bug is fixed with F5's rule). F was **not verified against an Egypt seed** (Phase D,
which adds it, is paused behind a concurrent session's plan-20 refactor) and its full e2e suite was
not re-run (dev server contention at the time) — run `python scripts/e2e/run.py` and re-verify
`bun run verify:mocks` once D lands before treating F as fully done. C, D, E, G are still pending.

Triggered by a review of the setup wizard's "بيانات المنشأة" step. It surfaced three kinds of work:

1. **Visible bugs:** the phone field collapses to a sliver, and the RTL switch thumb sits on the wrong side.
2. **Product gaps:** the app only knows Saudi Arabia, but **v1 ships for Egypt first**. Addresses are free
   text. Contrast fails WCAG in several places.
3. **Missing tooling:** there is no structured error / performance / debug / audit record, and nothing
   that helps debug accounting problems, which are the hardest bugs in this app.

Seven phases, in this order. Each one leaves the app working, so you can stop after any of them:

| Phase | File | What | Size | Status |
|---|---|---|---|---|
| A | [phase-a-phone-switch.md](phase-a-phone-switch.md) | **Quick fixes**: phone input (layout, trunk `0`, validation timing, Windows flags), switch "on" = right in RTL | S | done |
| B | [phase-b-diagnostics.md](phase-b-diagnostics.md) | **Diagnostics foundation**: one logger, 5 channels (error, perf, debug, audit, accounting), log files, the "failed to fix" issue ledger | M | done |
| C | [phase-c-contrast.md](phase-c-contrast.md) | **Contrast & accessibility**: token fixes backed by measured ratios, `check-contrast.ts`, focus and field semantics | S–M | pending |
| D | [phase-d-country-profiles.md](phase-d-country-profiles.md) | **Country profiles (EG first, SA)**: one owner file for per-country rules; wizard asks country first; **fix the VAT rate for Egypt** | M | pending |
| E | [phase-e-address-picker.md](phase-e-address-picker.md) | **Address picker**: `eg.json` / `sa.json`, `geoService`, one shared `AddressFields` block used everywhere | M | pending |
| F | [phase-f-accounting-debugger.md](phase-f-accounting-debugger.md) | **Accounting debugger**: posting traces, runtime invariants, inspector page, repro bundles + replay | L | mostly done — see file (auto-watcher + first regression case still open) |
| G | [phase-g-dev-loop.md](phase-g-dev-loop.md) | **Dev loop**: e2e/verify/perf feed the ledger automatically; open issues appear in `AGENT_MEMORY.md` | S–M | pending |

**Why this order.** A is small and user-visible. B comes second so that every later phase is built with
logging already in place, and C–F errors land in the ledger from day one. D must come before E because
the address shape depends on the country. F builds on B's accounting channel.

**Agent instructions** (same rules as [15](../../../docs/v2/15-action-plan.md) and [17](../../../docs/v2/17-ui-system-rtl-themes.md)):
- Still **UI-only against the mock backend**. Pages touch only `modules/*/services`.
- Tick the boxes in each phase file as you go, add a status note at the top of that file when the phase ends,
  and set its **Status** column above to `done`.
- When every phase is done and verified, move this whole folder to `plans/completed/` (CLAUDE.md "Plans").
- One commit per phase (Phase F: one commit per sub-part F1–F5).
- Run the **whole** e2e suite at the end of each phase.
- Phase D and F touch money logic: read [02](../../../docs/v2/02-accounting-review.md) first and keep `bun run verify:mocks`
  fully green, for **both** the SA and the EG seed (added in D).

**Definition of done (every phase):** the gates from [15](../../../docs/v2/15-action-plan.md) and CLAUDE.md: `bun run build`,
`bun run check`, `bun run verify:mocks`, `bun run memory` (structural changes), the full e2e suite with no
console errors, light + dark at 1280/1920 px, RTL + keyboard. Phases B and F add Rust commands, so they
also need `cargo build --manifest-path src-tauri/Cargo.toml` and a real `bun run desktop` check.

### Decisions already made

| # | Decision | Notes |
|---|---|---|
| 1 | **Switch "on" = thumb on the right, in RTL too.** | This reverses doc 17 Phase A and CLAUDE.md rule 18 ("a switch's 'on' thumb sits on the left"). Phase A updates both. Note: iOS and Material mirror the switch in RTL (on = left). This is a deliberate product choice, not an oversight. |
| 2 | **Default country = Egypt.** Only EG and SA are offered. | AE stays in the type union for later but is removed from the picker. |
| 3 | **Business audit ≠ diagnostic logs.** | Audit is business data: it lives in the DB, goes into backups, is append-only, and a real backend owns it. Logs are device-local dev/support data and are not in backups. |
| 4 | Egypt ETA e-invoicing (the government API) is **out of scope** here. | D only makes Egyptian invoices *correct* (rate, titles, currency, words). ETA submission becomes its own doc later. |

### Open decision (needs the owner, blocks shipping SA addresses only)

- **Saudi data license.** The only complete dataset found,
  [homaily/Saudi-Arabia-Regions-Cities-and-Districts](https://github.com/homaily/Saudi-Arabia-Regions-Cities-and-Districts)
  (13 regions / 4,580 cities / 3,730 districts, Arabic + English, collected from the public
  `maps.address.gov.sa`), is **GPL-2.0**. Bundling it in a closed commercial app is a legal question.
  The Egypt dataset ([Tech-Labs/egypt-governorates-and-cities-db](https://github.com/Tech-Labs/egypt-governorates-and-cities-db))
  is **MIT**, so no issue there.
  **Recommendation:** develop with the homaily data. Before shipping a Saudi build, either rebuild
  `sa.json` from the official public source (SPL National Address), get clearance, or ship SA with
  region + city only from a permissively licensed list. Egypt, the v1 market, is unaffected.

## Risks

| Risk | Mitigation |
|---|---|
| Logging slows the app or fills the disk | Batched async writes, debug off by default, rotation + retention, perf budget on the logger itself |
| Logs leak secrets or customer data in support bundles | Redaction list in one config file; DB snapshot is opt-in in the export |
| `defineService` codemod breaks the memory scanner or imports | Update the parser in the same commit; compare Service API tables before and after |
| Changing the SA seed tax path breaks accounting | EG and SA seeds both run every invariant; golden totals; read doc 02 first |
| Saudi dataset license | Open decision above; Egypt (v1) is unaffected |
| Switch direction reversal confuses existing users | Only the thumb side changes; label text and state are unchanged; covered by e2e |
| `sa.json` size | Lazy chunk, `_lite` sources without GIS data, ar+en only |
