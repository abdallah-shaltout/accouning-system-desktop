/**
 * Drift check (21.02-F, F-4): proves the ts-rs-generated audit DTOs (`diagnostics/types/gen/*`,
 * written by `bun run bindings` from `src-tauri/src/core/dto.rs`) have exactly the same shape as
 * the hand-written TS types in `./index.ts`.
 *
 * Never imported by app code — this file exists only to be type-checked by `bun run build`.
 */
import type { Equals, Expect } from '@/modules/core/types/contract';
import type { AuditAction, AuditEntry, AuditFieldDiff } from './index';

import type { AuditAction as GenAuditAction } from './gen/AuditAction';
import type { AuditFieldDiff as GenAuditFieldDiff } from './gen/AuditFieldDiff';
import type { AuditEntry as GenAuditEntry } from './gen/AuditEntry';

export type _AuditAction = Expect<Equals<GenAuditAction, AuditAction>>;
export type _AuditFieldDiff = Expect<Equals<GenAuditFieldDiff, AuditFieldDiff>>;
export type _AuditEntry = Expect<Equals<GenAuditEntry, AuditEntry>>;
