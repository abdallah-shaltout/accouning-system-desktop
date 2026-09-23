import type { Account, AccountGroup, FiscalYear } from '@/modules/accounting/types';

export const accountGroupsFixture: AccountGroup[] = [
  { id: 'grp-asset', code: '1', name: 'الأصول', kind: 'ASSET', normalSide: 'DEBIT', isDefault: true },
  { id: 'grp-liability', code: '2', name: 'الالتزامات', kind: 'LIABILITY', normalSide: 'CREDIT', isDefault: true },
  { id: 'grp-equity', code: '3', name: 'حقوق الملكية', kind: 'EQUITY', normalSide: 'CREDIT', isDefault: true },
  { id: 'grp-revenue', code: '4', name: 'الإيرادات', kind: 'REVENUE', normalSide: 'CREDIT', isDefault: true },
  { id: 'grp-expense', code: '5', name: 'المصروفات', kind: 'EXPENSE', normalSide: 'DEBIT', isDefault: true },
];

type Row = [code: string, name: string, groupId: string, side: 'DEBIT' | 'CREDIT'];

/** The simplified default chart of accounts — domain_model.md §1. */
const rows: Row[] = [
  ['1110', 'الصندوق', 'grp-asset', 'DEBIT'],
  ['1120', 'البنك', 'grp-asset', 'DEBIT'],
  ['1130', 'العملاء', 'grp-asset', 'DEBIT'],
  ['1140', 'بضاعة المخزون', 'grp-asset', 'DEBIT'],
  ['1150', 'ضريبة مدفوعة قابلة للاسترداد', 'grp-asset', 'DEBIT'],
  ['2100', 'الموردين', 'grp-liability', 'CREDIT'],
  ['2150', 'ضريبة مستحقة', 'grp-liability', 'CREDIT'],
  ['3100', 'رأس المال', 'grp-equity', 'CREDIT'],
  ['3250', 'أرباح محتجزة', 'grp-equity', 'CREDIT'],
  ['3300', 'صافي الربح/الخسارة', 'grp-equity', 'CREDIT'],
  ['3400', 'مسحوبات شخصية', 'grp-equity', 'DEBIT'],
  ['4100', 'المبيعات', 'grp-revenue', 'CREDIT'],
  ['4200', 'مرتجعات المبيعات', 'grp-revenue', 'DEBIT'],
  ['4300', 'إيرادات أخرى', 'grp-revenue', 'CREDIT'],
  ['4400', 'أرباح جرد المخزون', 'grp-revenue', 'CREDIT'],
  ['5100', 'المشتريات', 'grp-expense', 'DEBIT'],
  ['5150', 'مرتجعات المشتريات', 'grp-expense', 'CREDIT'],
  ['5200', 'تكلفة البضاعة المباعة', 'grp-expense', 'DEBIT'],
  ['5300', 'مصروفات تشغيل', 'grp-expense', 'DEBIT'],
  ['5500', 'مصروفات إدارية', 'grp-expense', 'DEBIT'],
  ['5600', 'رواتب وأجور', 'grp-expense', 'DEBIT'],
  ['5800', 'خسائر جرد المخزون', 'grp-expense', 'DEBIT'],
];

export const accountsFixture: Account[] = rows.map(([code, name, groupId, normalSide]) => ({
  id: `acc-${code}`,
  code,
  name,
  groupId,
  normalSide,
  canDelete: false,
  active: true,
}));

/** Account codes the posting rules rely on. */
export const ACC = {
  cash: '1110',
  bank: '1120',
  receivable: '1130',
  inventory: '1140',
  vatInput: '1150',
  payable: '2100',
  vatOutput: '2150',
  capital: '3100',
  retainedEarnings: '3250',
  netIncome: '3300',
  drawings: '3400',
  sales: '4100',
  salesReturns: '4200',
  otherIncome: '4300',
  stocktakeGains: '4400',
  purchases: '5100',
  cogs: '5200',
  operatingExpenses: '5300',
  adminExpenses: '5500',
  salaries: '5600',
  stocktakeLosses: '5800',
} as const;

export function fiscalYearFixture(now: Date): FiscalYear[] {
  const y = now.getFullYear();
  return [
    { id: `fy-${y - 1}`, name: String(y - 1), startDate: `${y - 1}-01-01`, endDate: `${y - 1}-12-31`, isClosed: true },
    { id: `fy-${y}`, name: String(y), startDate: `${y}-01-01`, endDate: `${y}-12-31`, isClosed: false },
  ];
}
