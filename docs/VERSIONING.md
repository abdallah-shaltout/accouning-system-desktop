# Versioning policy

Equal uses [SemVer](https://semver.org) (`MAJOR.MINOR.PATCH`). As of 2026-10-01 the version has
never moved from the scaffold default `0.1.0` in any of the three files that declare it — this doc
is the policy going forward, not a record of past bumps.

## Where the version lives (must always agree)

| File | Field |
| --- | --- |
| `package.json` | `"version"` |
| `src-tauri/tauri.conf.json` | `"version"` |
| `src-tauri/Cargo.toml` | `[package] version` |

All three must be bumped together, in the same commit as the change that earns the bump. A mismatch
between them is a bug: `tauri.conf.json`'s version is what ships in the installer and the app's
"About" screen; `Cargo.toml`'s is what `cargo`/`cargo-safe.ps1` print; `package.json`'s is what
`bun`/`npm` tooling sees. Bump all three with one pass — don't hand-edit one and forget the others.

## When to bump, and by how much

Decide the bump from the **highest-impact commit type** included since the last bump, using the
Conventional Commits prefixes already in this repo's history (`feat`, `fix`, `test`, `docs`,
`chore`, `refactor`, `style`, `perf`). This repo doesn't do one-bump-per-commit — it batches: bump
once per release, sized to the most significant thing in that batch.

| Bump | Triggered by | Example |
| --- | --- | --- |
| **MAJOR** (`X.0.0`) | A breaking change: an accounting posting-rule change, a DB schema change with no backward migration, an IPC contract change that breaks an older build talking to a newer one, or a deliberate product decision documented as a breaking change (see `docs/v2/02-accounting-review.md` for what counts as breaking on the accounting side) | Changing VAT rounding mode; renaming a Tauri command without a compat shim |
| **MINOR** (`x.Y.0`) | `feat:` — a new user-facing capability, a new report, a new module, a new page, a new `[[bin]]` the app ships | `feat(pos): add split-payment support` |
| **PATCH** (`x.y.Z`) | `fix:` — a bug fix, `perf:` — a performance fix, or `test:` when it closes an `ACC-`/`BUG-`/`PERF-` ledger issue (see CLAUDE.md "Diagnostics") | `fix(attachments): require write access to the attached document's area` |

### Do NOT bump for (the "trash or simple" exclusion)

These commit types never justify a version bump on their own, even batched together:

- `docs:` — documentation-only changes (this file included)
- `chore:` — tooling, CI, build config, dependency bumps with no behavior change
- `style:` — formatting, whitespace, lint-only fixes
- `refactor:` — internal restructuring with no observable behavior change
- A commit whose entire diff is inside `AGENT_MEMORY.md`, `docs/`, `plans/`, `.vscode/`, or test
  fixtures, with nothing in `src/` or `src-tauri/src/`

If a release batch contains **only** these types, skip the bump entirely — ship the same version
number again. Don't invent a bump just because time passed or a commit happened.

### Mixed batches

When a release batch mixes types, the bump follows the **highest** one present: one `feat` among
ten `fix`/`chore` commits still means MINOR, not PATCH. One breaking change among any number of
`feat`/`fix` commits means MAJOR, regardless of how small that breaking change looks.

## How to apply a bump

1. Decide the bump size using the table above, from the commits since the last version bump
   (`git log <last-bump-commit>..HEAD --oneline`).
2. Update the version in all three files listed above, to the same value.
3. Commit that version bump on its own (`chore(release): vX.Y.Z`), as the last commit before
   tagging/shipping — not mixed into a feature commit.
4. Tag the commit: `git tag vX.Y.Z` (ask before pushing tags — tagging is reversible locally but a
   pushed tag is shared state, per this repo's existing rule on actions visible to others).

## Where this fits the existing workflow

This policy only decides *whether and how much* to bump — it doesn't change anything else in
`CLAUDE.md`. A version bump commit still goes through the normal
["Definition of done"](../CLAUDE.md#definition-of-done-every-change-that-touches-ui-or-logic) gates
before it ships, same as any other commit, and a release build still goes through the "Fast loop vs.
release gate" rules in `CLAUDE.md` (`bun run tauri:build:fast` for a human-run, full-core manual
build; the `jobs = 4` default stays for everything else).
