# Regression cases (18.F4)

Minimized repro bundles that reproduce a real, fixed accounting bug — the permanent record an
`ACC-` ledger issue's `regression_test` points at (18.F5: "every accounting bug fix ships with a
verify case; an `ACC-` issue can only become `verified` when that case exists and is green").

Each file here is a `ReproBundle` (see `src/modules/diagnostics/services/actionJournal.ts`):
exported from `/dev/diagnostics`'s "المحاسبة" tab ("تصدير حالة لإعادة الإنتاج"), then trimmed by
hand to the smallest `startSnapshot` + `actions` list that still reproduces the break, with any
real customer/party data replaced by seed-safe placeholders.

Run every case: `bun run verify:replay` (no argument). Run one bundle directly (including one not
yet moved here): `bun run verify:replay <path-to-bundle.json>`.
