/**
 * Drift check (21.02-F, F-4): proves every ts-rs-generated cross-cutting DTO
 * (`core/types/gen/*`, written by `bun run bindings` from `src-tauri/src/core/{dto,events}.rs`)
 * has *exactly* the same shape as the hand-written TS type it mirrors. A field-type or
 * optionality drift on either side fails `vue-tsc` here, not silently at runtime.
 *
 * Never imported by app code — this file exists only to be type-checked by `bun run build`.
 */
import type { Equals, Expect } from './contract';
import type { ApiErrorPayload, BackendChangedPayload, BackendServerStatus, BackendStatus, ChangeCategory } from './backend';
import type { PagedQuery, PagedResult, PageSort } from './paging';
import type { ActivityEntry } from './index';

import type { ApiErrorPayload as GenApiErrorPayload } from './gen/ApiErrorPayload';
import type { BackendChangedPayload as GenBackendChangedPayload } from './gen/BackendChangedPayload';
import type { BackendServerStatus as GenBackendServerStatus } from './gen/BackendServerStatus';
import type { BackendStatus as GenBackendStatus } from './gen/BackendStatus';
import type { ChangeCategory as GenChangeCategory } from './gen/ChangeCategory';
import type { PageSort as GenPageSort } from './gen/PageSort';
import type { PagedQuery as GenPagedQuery } from './gen/PagedQuery';
import type { PagedResult as GenPagedResult } from './gen/PagedResult';
import type { ActivityEntry as GenActivityEntry } from './gen/ActivityEntry';

export type _ApiErrorPayload = Expect<Equals<GenApiErrorPayload, ApiErrorPayload>>;
export type _BackendChangedPayload = Expect<Equals<GenBackendChangedPayload, BackendChangedPayload>>;
export type _BackendServerStatus = Expect<Equals<GenBackendServerStatus, BackendServerStatus>>;
export type _BackendStatus = Expect<Equals<GenBackendStatus, BackendStatus>>;
export type _ChangeCategory = Expect<Equals<GenChangeCategory, ChangeCategory>>;
export type _PageSort = Expect<Equals<GenPageSort, PageSort>>;

// Generics checked at a concrete instantiation (F-4) — the Rust side is exported with a `String`
// filler type argument (`core/ipc.rs`'s `export_bindings`), so the TS comparison uses the same
// concrete instance rather than the bare generic alias.
export type _PagedQuery = Expect<Equals<GenPagedQuery<Record<string, unknown>>, PagedQuery<Record<string, unknown>>>>;
export type _PagedResult = Expect<Equals<GenPagedResult<string>, PagedResult<string>>>;

export type _ActivityEntry = Expect<Equals<GenActivityEntry, ActivityEntry>>;
