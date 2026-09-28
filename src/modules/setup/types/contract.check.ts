/**
 * Drift check (21.02-F, F-4): proves the ts-rs-generated setup DTOs (`setup/types/gen/*`, written
 * by `bun run bindings` from `src-tauri/src/infrastructure/import/dto.rs`, and later
 * `src-tauri/src/domains/setup/dto.rs`) have exactly the same shape as the hand-written TS types in
 * `./index.ts`.
 *
 * Never imported by app code — this file exists only to be type-checked by `bun run build`.
 *
 * 00-import (D10) writes the first three entries below. 02-setup (W2) appends its own once its DTOs
 * land — this file is created once, then extended, never replaced.
 */
import type { Equals, Expect, Simplify } from '@/modules/core/types/contract';
import type { ImportMode, ImportSnapshotResult, LegacySnapshotSummary } from './index';

import type { ImportMode as GenImportMode } from './gen/ImportMode';
import type { ImportSnapshotResult as GenImportSnapshotResult } from './gen/ImportSnapshotResult';
import type { LegacySnapshotSummary as GenLegacySnapshotSummary } from './gen/LegacySnapshotSummary';

export type _ImportMode = Expect<Equals<GenImportMode, ImportMode>>;
// KNOWN GAP (needs a Rust fix, not a TS one): `ImportSnapshotResult.roundedValues` is a genuine
// type mismatch, not a `Simplify<>`-fixable cosmetic difference — `src-tauri/src/infrastructure/
// import/dto.rs`'s `pub rounded_values: i64` has no `#[ts(type = "number")]` override, so ts-rs
// emits `roundedValues: bigint` in `./gen/ImportSnapshotResult.ts`, while the hand-written contract
// (`ImportSnapshotResult.roundedValues: number` in `./index.ts`) is a plain JS number — a small
// "how many values got rounded" count, never realistically outside safe-integer range, and every
// value ts-rs/serde carries over IPC as JSON has no native bigint anyway. Compare
// `schema_version: i32` a few lines below in the same Rust file, which needs no override because
// ts-rs only defaults 64-bit integers to `bigint`; smaller ints already map to `number`. Fix: add
// `#[ts(type = "number")]` above `rounded_values` (matching the `serde_number`-style overrides used
// throughout `domains/*/dto.rs` for other counts/amounts), then re-run `bun run bindings`. Per
// CLAUDE.md/plan-21 §3.4 ("the TS type is the contract"), the hand-written `number` stays as-is —
// this is a Rust DTO mistake to fix on that side, not something to change here.
export type _ImportSnapshotResult = Expect<Equals<GenImportSnapshotResult, ImportSnapshotResult>>;
// contract-ok: `LegacySnapshotSummary` is a hand-written `interface` (`./index.ts`) but ts-rs emits
// a `type` object literal for `Gen` — every field matches 1:1 (verified individually), but the
// bare `Equals<>` identity trick treats an interface declaration and a structurally-identical type
// literal as non-identical top-level shapes. `Simplify<>` (see its doc comment in
// `core/types/contract.ts`) flattens both sides to plain object types before comparing, which is
// exactly the case it exists for.
export type _LegacySnapshotSummary = Expect<Equals<Simplify<GenLegacySnapshotSummary>, Simplify<LegacySnapshotSummary>>>;

// --- 02-setup (W2) ---------------------------------------------------------------------------
//
// `WizardBranchInput`/`WizardPaymentMethodInput` and the opening line types (`OpeningCashLine`/
// `OpeningPartyLine`/`OpeningOtherLine`/`OpeningEntryInput`/`OpeningStockLine`/
// `PartyOpeningBalanceInput`) have completed the "TS moves first" step: they now live in
// `./index.ts` and `src/mocks/backend/{setup,opening}.ts` import them back (type-only). Their
// `Equals<>` checks are NOT added below yet because `bun run bindings` has not run since the
// move — `./gen/WizardBranchInput.ts` etc. do not exist on disk yet, and importing a
// not-yet-generated file would break `bun run build`. Add the checks (same pattern as the rows
// below) the next time bindings are regenerated. `CountryTaxInput.extraCurrencies[]`'s shape is
// checked structurally as part of `CountryTaxInput` itself, so `ExtraCurrency` needs no separate
// entry.

import type { CountryTaxInput, DeviceSetupState, OnboardingProgress, OnboardingProgressPatch, PairTerminalInput, PostOpeningBalancesResult } from './index';

import type { CountryTaxInput as GenCountryTaxInput } from './gen/CountryTaxInput';
import type { DeviceSetupState as GenDeviceSetupState } from './gen/DeviceSetupState';
import type { OnboardingProgress as GenOnboardingProgress } from './gen/OnboardingProgress';
import type { OnboardingProgressPatch as GenOnboardingProgressPatch } from './gen/OnboardingProgressPatch';
import type { PairTerminalInput as GenPairTerminalInput } from './gen/PairTerminalInput';
import type { PostOpeningBalancesResult as GenPostOpeningBalancesResult } from './gen/PostOpeningBalancesResult';

// contract-ok: same `interface`-vs-type-literal identity quirk as `_LegacySnapshotSummary` above —
// `Simplify<>` flattens both sides before the exact-identity check.
//
// KNOWN GAP (needs a Rust fix, not a TS one): as of this writing `_OnboardingProgress` /
// `_OnboardingProgressPatch` still fail even with `Simplify<>`, because
// `src-tauri/src/domains/setup/dto.rs`'s `#[ts(optional, type = "AccountTemplate")]` annotations on
// `OnboardingProgress.coa_template` / `OnboardingProgressPatch.coa_template` emit a bare
// `AccountTemplate` reference with no import in `setup/types/gen/OnboardingProgress(.Patch).ts` —
// ts-rs never adds an import for a raw `type = "…"` override, unlike the `import('@/modules/core/
// types/address').Address` inline-import form `WizardBranchInput.address` uses a few structs above
// in the same file. Fix: change both annotations to `type = "import('@/mocks/fixtures/accounts').
// AccountTemplate"` (matching where `setup/types/index.ts` imports `AccountTemplate` from today),
// then re-run `bun run bindings`. Until then `./gen/OnboardingProgress(.Patch).ts` don't compile on
// their own, so `_OnboardingProgress`/`_OnboardingProgressPatch` fail `bun run build` two ways at
// once (the missing-name error in the gen file itself, and this `Equals<>` never evaluating to
// `true` because `Gen` is `any`/erroring) — not something fixable from the TS side.
export type _OnboardingProgress = Expect<Equals<Simplify<GenOnboardingProgress>, Simplify<OnboardingProgress>>>;
export type _OnboardingProgressPatch = Expect<Equals<Simplify<GenOnboardingProgressPatch>, Simplify<OnboardingProgressPatch>>>;
export type _CountryTaxInput = Expect<Equals<GenCountryTaxInput, CountryTaxInput>>;
export type _PostOpeningBalancesResult = Expect<Equals<GenPostOpeningBalancesResult, PostOpeningBalancesResult>>;
export type _DeviceSetupState = Expect<Equals<GenDeviceSetupState, DeviceSetupState>>;
export type _PairTerminalInput = Expect<Equals<GenPairTerminalInput, PairTerminalInput>>;
