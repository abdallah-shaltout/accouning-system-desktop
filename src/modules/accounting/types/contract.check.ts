/**
 * Drift check (21.02-F, F-4 / plans/pending/21-rust-backend/03-domains/{12-accounting,12b-period-close}.md §2):
 * proves the ts-rs-generated `accounting` DTOs (`accounting/types/gen/*`, written by `bun run
 * bindings` from `src-tauri/src/domains/accounting/dto/*.rs`) have exactly the same shape as the
 * hand-written TS types in `./index.ts` / `../services/accountingService.ts`.
 *
 * Never imported by app code — this file exists only to be type-checked by `bun run build`.
 */
import type { Equals, Expect, Simplify } from '@/modules/core/types/contract';
import type { Account, AccountInput, FiscalYear, JournalEntry, JournalEntryInput, JournalFilter, JournalTemplate, JournalTemplateInput } from './index';

import type { AccountWithBalance, getJournalEntry, JournalRow, LinkedJournalEntry } from '../services/accountingService';

import type { Account as GenAccount } from './gen/Account';
import type { AccountInput as GenAccountInput } from './gen/AccountInput';
import type { AccountWithBalance as GenAccountWithBalance } from './gen/AccountWithBalance';
import type { FiscalYear as GenFiscalYear } from './gen/FiscalYear';
import type { JournalEntry as GenJournalEntry } from './gen/JournalEntry';
import type { JournalLine as GenJournalLine } from './gen/JournalLine';
import type { JournalEntryInput as GenJournalEntryInput } from './gen/JournalEntryInput';
import type { JournalFilter as GenJournalFilter } from './gen/JournalFilter';
import type { JournalTemplate as GenJournalTemplate } from './gen/JournalTemplate';
import type { JournalTemplateInput as GenJournalTemplateInput } from './gen/JournalTemplateInput';
import type { JournalRow as GenJournalRow } from './gen/JournalRow';
import type { LinkedJournalEntry as GenLinkedJournalEntry } from './gen/LinkedJournalEntry';
import type { JournalEntryDetail as GenJournalEntryDetail } from './gen/JournalEntryDetail';

// 12b-period-close.md §2's five additions.
import type { CloseYearPreCheck, VatPeriodTotals } from './index';
import type { CloseYearPreCheck as GenCloseYearPreCheck } from './gen/CloseYearPreCheck';
import type { VatPeriodTotals as GenVatPeriodTotals } from './gen/VatPeriodTotals';
import type { FiscalYearInput as GenFiscalYearInput } from './gen/FiscalYearInput';
import type { CloseYearResult as GenCloseYearResult } from './gen/CloseYearResult';
import type { closeYear } from '../services/accountingService';

// `JournalLine` isn't re-exported by name from `./index` at the top level of this file's imports
// above (it's used inside `JournalEntry`), so pull it in for its own equality check too.
import type { JournalLine } from './index';

export type _Account = Expect<Equals<GenAccount, Account>>;
export type _AccountInput = Expect<Equals<GenAccountInput, AccountInput>>;
export type _JournalEntry = Expect<Equals<GenJournalEntry, JournalEntry>>;
export type _JournalLine = Expect<Equals<GenJournalLine, JournalLine>>;
export type _JournalEntryInput = Expect<Equals<GenJournalEntryInput, JournalEntryInput>>;
export type _JournalFilter = Expect<Equals<GenJournalFilter, JournalFilter>>;
export type _JournalTemplate = Expect<Equals<GenJournalTemplate, JournalTemplate>>;
export type _JournalTemplateInput = Expect<Equals<GenJournalTemplateInput, JournalTemplateInput>>;
// `AccountWithBalance`/`JournalRow`/`JournalEntryDetail`/`CloseYearResult` are hand-written as
// intersection types (`Account & {...}` etc.) — `Simplify<>` flattens them so `Equals` compares
// members, not intersection-vs-flat-object nominal shape (contract.ts's own doc comment).
export type _AccountWithBalance = Expect<Equals<GenAccountWithBalance, Simplify<AccountWithBalance>>>;
export type _JournalRow = Expect<Equals<GenJournalRow, Simplify<JournalRow>>>;
export type _LinkedJournalEntry = Expect<Equals<GenLinkedJournalEntry, LinkedJournalEntry>>;
export type _JournalEntryDetail = Expect<Equals<GenJournalEntryDetail, Simplify<Awaited<ReturnType<typeof getJournalEntry>>>>>;

// 12b-period-close.md §2 (5 entries).
export type _FiscalYear = Expect<Equals<GenFiscalYear, FiscalYear>>;
export type _FiscalYearInput = Expect<Equals<GenFiscalYearInput, Omit<FiscalYear, 'id'>>>;
export type _CloseYearPreCheck = Expect<Equals<GenCloseYearPreCheck, CloseYearPreCheck>>;
export type _VatPeriodTotals = Expect<Equals<GenVatPeriodTotals, VatPeriodTotals>>;
export type _CloseYearResult = Expect<Equals<GenCloseYearResult, Simplify<Awaited<ReturnType<typeof closeYear>>>>>;
