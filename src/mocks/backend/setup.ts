/**
 * v2 phase 5 (docs/v2/05-onboarding.md): backend glue for the setup wizard. Additive module — never
 * touches phases 0-9's existing posting/closing logic. Wraps `buildAccounts` (CoA templates),
 * `db.branches`/`db.currencies` seeding and the `opening.ts` posting helpers behind one surface the
 * wizard's `setupService.ts` calls.
 */
import type { Account, FiscalYear } from '@/modules/accounting/types';
import type { AccountTemplate } from '../fixtures/accounts';
import { buildAccounts } from '../fixtures/accounts';
import { db } from '../db';
import { mutate } from '../persist';
import { ApiError, localDateKey, uid } from '../utils';
import { createBranch } from './branches';

/**
 * Business-type defaults (docs/v2/05 §2 step 1 "Sets the defaults: units, product fields…"). Only
 * seeds units when the catalog is still empty (a fresh company) — never overwrites units the owner
 * may already have edited by revisiting this step.
 */
export function applyBusinessTypeUnitDefaults(businessType: string): void {
  if (db.units.length > 0) return;
  const base = [{ id: uid('unit'), name: 'قطعة', symbol: 'pc' }];
  const extra =
    businessType === 'pharmacy'
      ? [{ id: uid('unit'), name: 'علبة', symbol: 'box' }, { id: uid('unit'), name: 'شريط', symbol: 'strip' }]
      : businessType === 'supermarket'
        ? [{ id: uid('unit'), name: 'كرتونة', symbol: 'ctn' }]
        : businessType === 'services'
          ? [{ id: uid('unit'), name: 'خدمة' }]
          : businessType === 'clothing'
            ? [{ id: uid('unit'), name: 'طقم', symbol: 'set' }]
            : [];
  mutate(() => (db.units = [...base, ...extra]));
}

/** Applies a CoA template (docs/v2/05 §2 step 6): replaces `db.accounts` wholesale — only valid before any posting exists. */
export function applyCoaTemplate(template: AccountTemplate, country: 'SA' | 'EG' | 'AE', businessType?: string): Account[] {
  if (db.journalEntries.length > 0) throw new ApiError('لا يمكن تغيير دليل الحسابات بعد بدء الترحيل', 'FORBIDDEN');
  const accounts = buildAccounts({ template, country, businessType });
  mutate(() => (db.accounts = accounts));
  return accounts;
}

/** Live preview of a template's tree without touching `db` — used by the CoA step's picker. */
export function previewCoaTemplate(template: AccountTemplate, country: 'SA' | 'EG' | 'AE', businessType?: string): Account[] {
  return buildAccounts({ template, country, businessType });
}

/** Sets the fiscal year to start on (startMonth, startDay) of the go-live year and run 12 months. */
export function setFiscalYear(startMonth: number, startDay: number, referenceDate: string): FiscalYear {
  const ref = new Date(referenceDate);
  let start = new Date(ref.getFullYear(), startMonth - 1, startDay);
  if (start > ref) start = new Date(ref.getFullYear() - 1, startMonth - 1, startDay);
  const end = new Date(start);
  end.setFullYear(end.getFullYear() + 1);
  end.setDate(end.getDate() - 1);
  const fy: FiscalYear = {
    id: db.fiscalYears[0]?.id ?? uid('fy'),
    name: String(start.getFullYear()),
    startDate: localDateKey(start),
    endDate: localDateKey(end),
    isClosed: false,
  };
  mutate(() => (db.fiscalYears = [fy]));
  return fy;
}

export interface WizardBranchInput {
  name: string;
  code: string;
  address?: string;
}

/**
 * Creates every branch from the wizard's step 5 (docs/v2/05 §2 step 5), accountFor-style
 * auto-created cash account + cost center included (reuses the real `createBranch` — same
 * function Settings → Branches uses — for every branch after the first).
 */
export function applyBranches(branches: WizardBranchInput[], userId: string): void {
  if (!branches.length) throw new ApiError('أضف فرعاً واحداً على الأقل');
  const [first, ...rest] = branches;
  // Rename the seeded main branch instead of duplicating it — `seedEmptyCompany`/`seedAccounts`
  // already created `branch-main` + its cost center + cash account so every pre-wizard posting
  // path (even before the wizard runs) has somewhere real to land.
  mutate(() => {
    const main = db.branches.find((b) => b.id === 'branch-main') ?? db.branches[0];
    if (main) {
      main.name = first.name;
      main.code = first.code.toUpperCase();
      main.address = first.address;
    }
  });
  for (const b of rest) {
    if (db.branches.some((x) => x.code.toLowerCase() === b.code.toLowerCase())) continue;
    createBranch({ name: b.name, code: b.code, address: b.address, active: true }, userId);
  }
  if (rest.length) mutate(() => (db.settings.features = { ...db.settings.features, branches: true }));
}

export interface WizardPaymentMethodInput {
  name: string;
  type: 'cash' | 'card' | 'bank_transfer' | 'wallet' | 'credit' | 'store_credit';
  accountRole: 'cash' | 'bank' | 'cardClearing' | 'walletClearing' | 'receivable';
  active: boolean;
}

/** Replaces the seeded payment-methods shell with the wizard's step-7 selection. */
export function applyPaymentMethods(methods: WizardPaymentMethodInput[]): void {
  mutate(
    () =>
      (db.paymentMethods = methods.map((m, i) => ({
        id: uid('pm'),
        name: m.name,
        type: m.type,
        accountRole: m.accountRole,
        feePct: 0,
        showInPos: true,
        showInPayments: true,
        sortOrder: i + 1,
        active: m.active,
        canDelete: true,
      }))),
  );
}
