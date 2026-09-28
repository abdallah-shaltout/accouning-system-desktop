/**
 * Drift check (05-parties.md §2): proves the ts-rs-generated `parties` DTOs
 * (`parties/types/gen/*`, written by `bun run bindings` from `src-tauri/src/domains/parties/dto.rs`)
 * have exactly the same shape as the hand-written TS types in `./index.ts` /
 * `../services/partyService.ts`.
 *
 * Never imported by app code — this file exists only to be type-checked by `bun run build`.
 */
import type { Equals, Expect } from '@/modules/core/types/contract';
import type { AgingBucket, Customer, CustomerInput, PartyGroup, PartyHistoryEntry, PartyStatementRow, Supplier, SupplierInput } from './index';
import type { DuplicateWarning, PartyFilter } from '../services/partyService';

import type { Customer as GenCustomer } from './gen/Customer';
import type { Supplier as GenSupplier } from './gen/Supplier';
import type { CustomerInput as GenCustomerInput } from './gen/CustomerInput';
import type { SupplierInput as GenSupplierInput } from './gen/SupplierInput';
import type { PartyGroup as GenPartyGroup } from './gen/PartyGroup';
import type { PartyStatementRow as GenPartyStatementRow } from './gen/PartyStatementRow';
import type { AgingBucket as GenAgingBucket } from './gen/AgingBucket';
import type { PartyHistoryEntry as GenPartyHistoryEntry } from './gen/PartyHistoryEntry';
import type { PartyFilter as GenPartyFilter } from './gen/PartyFilter';
import type { DuplicateWarning as GenDuplicateWarning } from './gen/DuplicateWarning';

export type _Customer = Expect<Equals<GenCustomer, Customer>>;
export type _Supplier = Expect<Equals<GenSupplier, Supplier>>;
export type _CustomerInput = Expect<Equals<GenCustomerInput, CustomerInput>>;
export type _SupplierInput = Expect<Equals<GenSupplierInput, SupplierInput>>;
export type _PartyGroup = Expect<Equals<GenPartyGroup, PartyGroup>>;
export type _PartyStatementRow = Expect<Equals<GenPartyStatementRow, PartyStatementRow>>;
export type _AgingBucket = Expect<Equals<GenAgingBucket, AgingBucket>>;
export type _PartyHistoryEntry = Expect<Equals<GenPartyHistoryEntry, PartyHistoryEntry>>;
export type _PartyFilter = Expect<Equals<GenPartyFilter, PartyFilter>>;
export type _DuplicateWarning = Expect<Equals<GenDuplicateWarning, DuplicateWarning>>;

// `getLinkedNetBalance`: Rust returns `number | null` (`OptionalMoney`, a manual `TS` impl with no
// standalone `gen/` file); the switch line converts `null → undefined` to match the TS signature's
// `Promise<number | undefined>`. // contract-ok: null→undefined at the switch line
