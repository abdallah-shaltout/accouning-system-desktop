/**
 * Branches & cost centers (docs/v2/10-branches-currencies-cost-centers.md §1, §3). Activates the
 * `branchId`/`costCenterId` fields every journal line and document has carried since Phase 1 —
 * this file is where real `Branch`/`CostCenter` rows come from and how a new branch gets its own
 * cash-drawer account (`accountFor('cash', { branchId })`, docs/v2/03-chart-of-accounts.md's
 * "several accounts may share a role, callers pick by branch" note) and its own cost center.
 */
import type { Account } from '@/modules/accounting/types';
import type { Branch, BranchInput, CostCenter, CostCenterInput } from '@/modules/settings/types';
import { db } from '../db';
import { emit } from '../events';
import { mutate } from '../persist';
import { ApiError, round2, uid } from '../utils';
import { accountById } from './accounts';
import { diffFields, logActivity, logAudit } from './core';

/** Single default branch id — matches `DEFAULT_BRANCH_ID` in `./core.ts` (every pre-phase-9 line/document already uses this id). */
export const MAIN_BRANCH_ID = 'branch-main';
export const MAIN_COST_CENTER_ID = 'cc-main';

export function branchById(id: string | undefined): Branch | undefined {
  return id ? db.branches.find((b) => b.id === id) : undefined;
}

export function activeBranches(): Branch[] {
  return db.branches.filter((b) => b.active);
}

/** Whether branches are meaningfully "on" — more than the one default branch, or the feature switch is explicitly set. */
export function branchesEnabled(): boolean {
  return db.settings.features?.branches === true;
}

export function currenciesEnabled(): boolean {
  return db.settings.features?.currencies === true;
}

export function costCentersEnabled(): boolean {
  return db.settings.features?.costCenters === true;
}

/** Finds the header account new branch cash accounts should nest under (same parent as the seeded main cash account, e.g. "11" الأصول المتداولة). */
function cashParentId(): string | null {
  const mainCash = db.accounts.find((a) => a.systemRole === 'cash' && !a.branchId);
  return mainCash?.parentId ?? null;
}

/** Next free "111x" leaf code under the cash group, so branch cash accounts don't collide with each other or the main 1110. */
function nextCashCode(): string {
  const parent = cashParentId();
  const siblings = db.accounts.filter((a) => a.parentId === parent && /^111\d$/.test(a.code));
  const used = new Set(siblings.map((a) => a.code));
  for (let i = 1; i <= 9; i++) {
    const code = `111${i}`;
    if (!used.has(code)) return code;
  }
  return `1119${db.accounts.length}`; // pathological fallback, still unique
}

/** Creates the branch's own cash-drawer account: "111x الصندوق — <branch>" (docs/v2/10 §1), via the same role/shape the seeded main cash account uses. */
function createBranchCashAccount(branch: Branch): Account {
  const account: Account = {
    id: uid('acc'),
    code: nextCashCode(),
    name: `الصندوق — ${branch.name}`,
    parentId: cashParentId(),
    isGroup: false,
    kind: 'ASSET',
    subtype: 'cash',
    normalSide: 'DEBIT',
    systemRole: 'cash',
    branchId: branch.id,
    allowManual: true,
    active: true,
    canDelete: false,
  };
  mutate(() => db.accounts.push(account));
  return account;
}

/** Creates the branch's own cost center (docs/v2/10 §3 "Branch cost centers are created with each branch and can't be deleted"). */
function createBranchCostCenter(branch: Branch): CostCenter {
  const cc: CostCenter = {
    id: uid('cc'),
    code: `CC-${branch.code}`,
    name: branch.name,
    type: 'branch',
    active: true,
    canDelete: false,
    branchId: branch.id,
  };
  mutate(() => db.costCenters.push(cc));
  return cc;
}

export function listBranches(): Branch[] {
  return db.branches;
}

export function createBranch(input: BranchInput, userId: string): Branch {
  if (!input.name.trim()) throw new ApiError('اسم الفرع مطلوب');
  if (!input.code.trim()) throw new ApiError('رمز الفرع مطلوب');
  if (db.branches.some((b) => b.code.toLowerCase() === input.code.trim().toLowerCase())) {
    throw new ApiError('رمز الفرع مستخدم بالفعل');
  }
  const branch: Branch = {
    id: uid('branch'),
    name: input.name.trim(),
    code: input.code.trim().toUpperCase(),
    address: input.address,
    phone: input.phone,
    receiptHeader: input.receiptHeader,
    bankAccountId: input.bankAccountId,
    defaultPriceListId: input.defaultPriceListId,
    active: input.active ?? true,
    canDelete: false,
    createdAt: new Date().toISOString(),
  };
  mutate(() => db.branches.push(branch));
  const cashAccount = createBranchCashAccount(branch);
  const costCenter = createBranchCostCenter(branch);
  mutate(() => {
    branch.cashAccountId = cashAccount.id;
    branch.costCenterId = costCenter.id;
  });
  logActivity('settings', `إضافة فرع "${branch.name}" (${branch.code})`, userId, branch.createdAt!, '/settings/branches');
  emit('ledger:changed');
  return branch;
}

export function updateBranch(id: string, input: Partial<BranchInput>, userId: string): Branch {
  const branch = db.branches.find((b) => b.id === id);
  if (!branch) throw new ApiError('الفرع غير موجود', 'NOT_FOUND');
  if (input.code && input.code.trim().toLowerCase() !== branch.code.toLowerCase() && db.branches.some((b) => b.id !== id && b.code.toLowerCase() === input.code!.trim().toLowerCase())) {
    throw new ApiError('رمز الفرع مستخدم بالفعل');
  }
  const before = { name: branch.name, code: branch.code, address: branch.address, phone: branch.phone, receiptHeader: branch.receiptHeader, bankAccountId: branch.bankAccountId, defaultPriceListId: branch.defaultPriceListId };
  mutate(() =>
    Object.assign(branch, {
      ...(input.name !== undefined ? { name: input.name.trim() } : {}),
      ...(input.code !== undefined ? { code: input.code.trim().toUpperCase() } : {}),
      ...(input.address !== undefined ? { address: input.address } : {}),
      ...(input.phone !== undefined ? { phone: input.phone } : {}),
      ...(input.receiptHeader !== undefined ? { receiptHeader: input.receiptHeader } : {}),
      ...(input.bankAccountId !== undefined ? { bankAccountId: input.bankAccountId } : {}),
      ...(input.defaultPriceListId !== undefined ? { defaultPriceListId: input.defaultPriceListId } : {}),
    }),
  );
  if (input.name && branch.cashAccountId) {
    const acc = db.accounts.find((a) => a.id === branch.cashAccountId);
    if (acc) mutate(() => (acc.name = `الصندوق — ${branch.name}`));
  }
  const after = { name: branch.name, code: branch.code, address: branch.address, phone: branch.phone, receiptHeader: branch.receiptHeader, bankAccountId: branch.bankAccountId, defaultPriceListId: branch.defaultPriceListId };
  const { before: beforeDiff, after: afterDiff } = diffFields(before, after);
  logAudit({
    entity: 'branch',
    entityId: branch.id,
    entityLabel: branch.name,
    action: 'update',
    before: beforeDiff,
    after: afterDiff,
    userId,
    message: `تعديل بيانات الفرع "${branch.name}"`,
    link: '/settings/branches',
    activityKind: 'settings',
  });
  return branch;
}

/** Deactivating a branch requires zero stock and no open shifts (docs/v2/10 §1). */
export function deactivateBranch(id: string, userId: string): Branch {
  const branch = db.branches.find((b) => b.id === id);
  if (!branch) throw new ApiError('الفرع غير موجود', 'NOT_FOUND');
  if (!branch.active) return branch;
  if (db.branches.filter((b) => b.active).length <= 1) throw new ApiError('لا يمكن إلغاء تفعيل الفرع الوحيد النشط');

  const stockLeft = db.products.reduce((a, p) => a + Math.abs(p.stockByBranch?.[id]?.qty ?? 0), 0);
  if (round2(stockLeft) > 0.001) throw new ApiError('لا يمكن إلغاء تفعيل الفرع — لا يزال يحتوي على مخزون. أنشئ تحويلاً لتفريغه أولاً', 'FORBIDDEN');

  const openShift = db.shifts.some((s) => s.status === 'OPEN' && s.branchId === id);
  if (openShift) throw new ApiError('لا يمكن إلغاء تفعيل الفرع — توجد وردية مفتوحة عليه', 'FORBIDDEN');

  mutate(() => (branch.active = false));
  if (branch.cashAccountId) {
    const acc = db.accounts.find((a) => a.id === branch.cashAccountId);
    if (acc) mutate(() => (acc.active = false));
  }
  logActivity('settings', `إلغاء تفعيل الفرع "${branch.name}"`, userId, new Date().toISOString(), '/settings/branches');
  return branch;
}

export function reactivateBranch(id: string, userId: string): Branch {
  const branch = db.branches.find((b) => b.id === id);
  if (!branch) throw new ApiError('الفرع غير موجود', 'NOT_FOUND');
  mutate(() => (branch.active = true));
  if (branch.cashAccountId) {
    const acc = db.accounts.find((a) => a.id === branch.cashAccountId);
    if (acc) mutate(() => (acc.active = true));
  }
  logActivity('settings', `إعادة تفعيل الفرع "${branch.name}"`, userId, new Date().toISOString(), '/settings/branches');
  return branch;
}

/** Branch-prefixed numbering (docs/v2/10 §1): "RYD-INV-00042" when >1 branch exists, unprefixed for a single-branch company. */
export function branchPrefix(branchId: string | undefined): string {
  if (db.branches.length <= 1) return '';
  const branch = branchById(branchId);
  return branch ? `${branch.code}-` : '';
}

// ---------------------------------------------------------------------------------------------
// Cost centers (§3)
// ---------------------------------------------------------------------------------------------

export function listCostCenters(): CostCenter[] {
  return db.costCenters;
}

export function costCenterById(id: string | undefined): CostCenter | undefined {
  return id ? db.costCenters.find((c) => c.id === id) : undefined;
}

export function createCostCenter(input: CostCenterInput, userId: string): CostCenter {
  if (!input.name.trim()) throw new ApiError('اسم مركز التكلفة مطلوب');
  if (!input.code.trim()) throw new ApiError('رمز مركز التكلفة مطلوب');
  if (db.costCenters.some((c) => c.code.toLowerCase() === input.code.trim().toLowerCase())) {
    throw new ApiError('رمز مركز التكلفة مستخدم بالفعل');
  }
  const cc: CostCenter = {
    id: uid('cc'),
    code: input.code.trim(),
    name: input.name.trim(),
    type: input.type,
    parentId: input.parentId,
    managerUserId: input.managerUserId,
    active: input.active ?? true,
    budgets: input.budgets,
    canDelete: true,
  };
  mutate(() => db.costCenters.push(cc));
  logActivity('settings', `إضافة مركز تكلفة "${cc.name}"`, userId, new Date().toISOString(), '/settings/cost-centers');
  return cc;
}

export function updateCostCenter(id: string, input: Partial<CostCenterInput>, userId: string): CostCenter {
  const cc = db.costCenters.find((c) => c.id === id);
  if (!cc) throw new ApiError('مركز التكلفة غير موجود', 'NOT_FOUND');
  mutate(() => Object.assign(cc, input));
  logActivity('settings', `تعديل مركز التكلفة "${cc.name}"`, userId, new Date().toISOString(), '/settings/cost-centers');
  return cc;
}

export function deleteCostCenter(id: string): void {
  const cc = db.costCenters.find((c) => c.id === id);
  if (!cc) throw new ApiError('مركز التكلفة غير موجود', 'NOT_FOUND');
  if (!cc.canDelete) throw new ApiError('لا يمكن حذف مركز تكلفة الفرع', 'FORBIDDEN');
  const inUse = db.journalEntries.some((e) => e.lines.some((l) => l.costCenterId === id));
  if (inUse) throw new ApiError('لا يمكن حذف مركز تكلفة له حركات مرحّلة', 'FORBIDDEN');
  mutate(() => (db.costCenters = db.costCenters.filter((c) => c.id !== id)));
}

/** Resolves the cost center a new document/line should default to: explicit choice → branch's own cost center → undefined. */
export function defaultCostCenterFor(branchId: string | undefined, explicit?: string): string | undefined {
  if (explicit) return explicit;
  const branch = branchById(branchId);
  return branch?.costCenterId;
}

export { accountById };
