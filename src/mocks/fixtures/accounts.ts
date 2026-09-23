import type { Account, AccountKind, AccountSubtype, FiscalYear, NormalSide, SystemRole } from '@/modules/accounting/types';

/**
 * Chart of accounts v2 (docs/v2/03-chart-of-accounts.md). A single tree: the five kinds are the
 * root header accounts (codes "1".."5"), everything below is either a header (`isGroup`) or a
 * postable leaf. Posting rules never reference `code` — they call `accountFor(role, ctx)`
 * (src/mocks/backend/accounts.ts), which looks accounts up by `systemRole`.
 *
 * `standardAccountRows()` is the قياسي template (~60 postable accounts, the seeded default).
 * `basicAccountRows()` / `detailedAccountRows()` are the مبسّط / مفصّل templates (§5). Country and
 * business add-ons are additive rows layered on top (`saAddonRows`, `pharmacyAddonRows`) — proof of
 * the add-on mechanism, not full coverage (README decision: pick 1-2 examples this phase).
 */

interface Row {
  code: string;
  name: string;
  nameEn?: string;
  parentCode: string | null;
  isGroup: boolean;
  kind: AccountKind;
  subtype: AccountSubtype;
  normalSide: NormalSide;
  role?: SystemRole;
  requiresParty?: boolean;
  allowManual: boolean;
  canDelete?: boolean;
}

const NORMAL_SIDE: Record<AccountKind, NormalSide> = {
  ASSET: 'DEBIT',
  LIABILITY: 'CREDIT',
  EQUITY: 'CREDIT',
  REVENUE: 'CREDIT',
  EXPENSE: 'DEBIT',
};

/** Shorthand: a leaf row inherits its kind's normal side unless it's a contra account. */
function leaf(
  code: string,
  name: string,
  parentCode: string,
  kind: AccountKind,
  subtype: AccountSubtype,
  opts: { role?: SystemRole; requiresParty?: boolean; allowManual?: boolean; contra?: boolean; canDelete?: boolean } = {},
): Row {
  return {
    code,
    name,
    parentCode,
    isGroup: false,
    kind,
    subtype,
    normalSide: opts.contra ? (NORMAL_SIDE[kind] === 'DEBIT' ? 'CREDIT' : 'DEBIT') : NORMAL_SIDE[kind],
    role: opts.role,
    requiresParty: opts.requiresParty,
    allowManual: opts.allowManual ?? true,
    canDelete: opts.canDelete ?? !opts.role,
  };
}

function group(code: string, name: string, parentCode: string | null, kind: AccountKind): Row {
  return { code, name, parentCode, isGroup: true, kind, subtype: 'otherCurrentAsset', normalSide: NORMAL_SIDE[kind], allowManual: false, canDelete: false };
}

/** The five root headers, shared by every template. */
function rootRows(): Row[] {
  return [
    group('1', 'الأصول', null, 'ASSET'),
    group('2', 'الالتزامات', null, 'LIABILITY'),
    group('3', 'حقوق الملكية', null, 'EQUITY'),
    group('4', 'الإيرادات', null, 'REVENUE'),
    group('5', 'تكلفة المبيعات', null, 'EXPENSE'),
    group('6', 'المصروفات', null, 'EXPENSE'),
  ];
}

/** Standard template (قياسي) — docs/v2/03-chart-of-accounts.md §4. */
export function standardAccountRows(): Row[] {
  return [
    ...rootRows(),

    group('11', 'الأصول المتداولة', '1', 'ASSET'),
    leaf('1110', 'الصندوق — الفرع الرئيسي', '11', 'ASSET', 'cash', { role: 'cash' }),
    leaf('1115', 'العهد النقدية وسلف الموظفين', '11', 'ASSET', 'otherCurrentAsset'),
    leaf('1120', 'البنك — الحساب الجاري', '11', 'ASSET', 'bank', { role: 'bank' }),
    leaf('1125', 'مدى والبطاقات تحت التسوية', '11', 'ASSET', 'clearing', { role: 'cardClearing' }),
    leaf('1126', 'المحافظ الإلكترونية تحت التسوية', '11', 'ASSET', 'clearing', { role: 'walletClearing' }),
    leaf('1130', 'العملاء', '11', 'ASSET', 'receivable', { role: 'receivable', requiresParty: true }),
    leaf('1140', 'المخزون', '11', 'ASSET', 'inventory', { role: 'inventory', allowManual: false }),
    leaf('1145', 'بضاعة بالطريق بين الفروع', '11', 'ASSET', 'inventory', { role: 'inventoryInTransit', allowManual: false }),
    leaf('1150', 'ضريبة القيمة المضافة — مدخلات', '11', 'ASSET', 'tax', { role: 'vatInput', allowManual: false }),
    leaf('1160', 'دفعات مقدمة للموردين', '11', 'ASSET', 'otherCurrentAsset'),
    leaf('1170', 'مصروفات مدفوعة مقدماً', '11', 'ASSET', 'prepaid'),
    leaf('1190', 'مخصص الديون المشكوك في تحصيلها', '11', 'ASSET', 'otherCurrentAsset', { contra: true }),

    group('12', 'الأصول غير المتداولة', '1', 'ASSET'),
    leaf('1210', 'أثاث وديكورات', '12', 'ASSET', 'fixedAsset'),
    leaf('1220', 'أجهزة ومعدات', '12', 'ASSET', 'fixedAsset'),
    leaf('1230', 'سيارات', '12', 'ASSET', 'fixedAsset'),
    leaf('1240', 'تحسينات على مبانٍ مستأجرة', '12', 'ASSET', 'fixedAsset'),
    leaf('1290', 'مجمع الإهلاك', '12', 'ASSET', 'accumulatedDepreciation', { contra: true }),

    group('21', 'الالتزامات المتداولة', '2', 'LIABILITY'),
    leaf('2100', 'الموردين', '21', 'LIABILITY', 'payable', { role: 'payable', requiresParty: true }),
    leaf('2150', 'ضريبة القيمة المضافة — مخرجات', '21', 'LIABILITY', 'tax', { role: 'vatOutput', allowManual: false }),
    leaf('2155', 'ضريبة القيمة المضافة — صافي مستحق', '21', 'LIABILITY', 'tax', { role: 'vatPayable' }),
    leaf('2160', 'رواتب مستحقة', '21', 'LIABILITY', 'currentLiability'),
    leaf('2170', 'مصروفات مستحقة', '21', 'LIABILITY', 'currentLiability'),
    leaf('2180', 'دفعات مقدمة وأرصدة دائنة للعملاء', '21', 'LIABILITY', 'currentLiability', { role: 'customerAdvances' }),

    group('22', 'الالتزامات غير المتداولة', '2', 'LIABILITY'),
    leaf('2210', 'قروض طويلة الأجل', '22', 'LIABILITY', 'longTermLiability'),
    leaf('2220', 'مخصص مكافأة نهاية الخدمة', '22', 'LIABILITY', 'longTermLiability'),

    leaf('3100', 'رأس المال', '3', 'EQUITY', 'equity', { role: 'capital' }),
    leaf('3150', 'جاري المالك / الشركاء', '3', 'EQUITY', 'equity', { role: 'ownerCurrent' }),
    leaf('3250', 'الأرباح المحتجزة', '3', 'EQUITY', 'equity', { role: 'retainedEarnings' }),
    leaf('3300', 'صافي ربح الفترة (افتراضي — يُحسب)', '3', 'EQUITY', 'equity', { role: 'currentEarnings', allowManual: false, canDelete: false }),
    leaf('3400', 'المسحوبات الشخصية', '3', 'EQUITY', 'equity', { role: 'drawings', contra: true }),
    leaf('3900', 'أرصدة افتتاحية (مؤقت)', '3', 'EQUITY', 'equity', { role: 'openingBalanceEquity' }),

    leaf('4100', 'مبيعات البضائع', '4', 'REVENUE', 'revenue', { role: 'sales' }),
    leaf('4110', 'إيرادات الخدمات', '4', 'REVENUE', 'revenue', { role: 'serviceRevenue' }),
    leaf('4200', 'مرتجعات ومسموحات المبيعات', '4', 'REVENUE', 'revenue', { role: 'salesReturns', contra: true }),
    leaf('4300', 'إيرادات أخرى', '4', 'REVENUE', 'otherIncome', { role: 'otherIncome' }),
    leaf('4310', 'أرباح فروق العملة', '4', 'REVENUE', 'otherIncome', { role: 'fxGain' }),
    leaf('4320', 'خصم مكتسب من الموردين', '4', 'REVENUE', 'otherIncome', { role: 'purchaseDiscounts' }),
    leaf('4330', 'زيادة الصندوق', '4', 'REVENUE', 'otherIncome', { role: 'cashOver' }),

    leaf('5100', 'تكلفة البضاعة المباعة', '5', 'EXPENSE', 'costOfSales', { role: 'cogs', allowManual: false }),
    leaf('5110', 'فروقات جرد المخزون', '5', 'EXPENSE', 'costOfSales', { role: 'inventoryVariance', allowManual: false }),
    leaf('5120', 'بضاعة تالفة ومنتهية الصلاحية', '5', 'EXPENSE', 'costOfSales', { role: 'inventoryWriteOff' }),
    leaf('5130', 'شحن وتخليص المشتريات', '5', 'EXPENSE', 'costOfSales', { role: 'freightIn' }),

    group('61', 'مصروفات البيع والتسويق', '6', 'EXPENSE'),
    leaf('6110', 'عمولات ومكافآت البيع', '61', 'EXPENSE', 'operatingExpense'),
    leaf('6120', 'دعاية وإعلان', '61', 'EXPENSE', 'operatingExpense'),
    leaf('6130', 'مواد تغليف وأكياس', '61', 'EXPENSE', 'operatingExpense'),
    leaf('6140', 'عمولات البطاقات ونقاط البيع', '61', 'EXPENSE', 'operatingExpense', { role: 'cardFees' }),
    leaf('6150', 'مصاريف توصيل', '61', 'EXPENSE', 'operatingExpense'),

    group('62', 'المصروفات العمومية والإدارية', '6', 'EXPENSE'),
    leaf('6210', 'الرواتب والأجور', '62', 'EXPENSE', 'operatingExpense'),
    leaf('6215', 'التأمينات الاجتماعية', '62', 'EXPENSE', 'operatingExpense'),
    leaf('6220', 'الإيجار', '62', 'EXPENSE', 'operatingExpense'),
    leaf('6230', 'الكهرباء والمياه', '62', 'EXPENSE', 'operatingExpense'),
    leaf('6240', 'الاتصالات والإنترنت', '62', 'EXPENSE', 'operatingExpense'),
    leaf('6250', 'الصيانة والإصلاح', '62', 'EXPENSE', 'operatingExpense'),
    leaf('6260', 'رسوم حكومية وتراخيص', '62', 'EXPENSE', 'operatingExpense'),
    leaf('6270', 'أدوات مكتبية ومطبوعات', '62', 'EXPENSE', 'operatingExpense'),
    leaf('6280', 'رسوم بنكية', '62', 'EXPENSE', 'operatingExpense', { role: 'bankFees' }),
    leaf('6290', 'مصروف الإهلاك', '62', 'EXPENSE', 'operatingExpense', { role: 'depreciation' }),
    leaf('6295', 'ديون معدومة ومشكوك فيها', '62', 'EXPENSE', 'operatingExpense', { role: 'badDebt' }),

    group('63', 'مصروفات أخرى', '6', 'EXPENSE'),
    leaf('6310', 'خسائر فروق العملة', '63', 'EXPENSE', 'otherExpense', { role: 'fxLoss' }),
    leaf('6320', 'عجز الصندوق', '63', 'EXPENSE', 'otherExpense', { role: 'cashShort' }),
    leaf('6390', 'مصروفات متنوعة', '63', 'EXPENSE', 'otherExpense'),
  ];
}

/** Basic template (مبسّط, ~25 accounts) — one bank, no fixed-asset/loan/accrual accounts, expenses collapsed. */
export function basicAccountRows(): Row[] {
  return [
    ...rootRows(),
    leaf('1110', 'الصندوق', '1', 'ASSET', 'cash', { role: 'cash' }),
    leaf('1120', 'البنك', '1', 'ASSET', 'bank', { role: 'bank' }),
    leaf('1130', 'العملاء', '1', 'ASSET', 'receivable', { role: 'receivable', requiresParty: true }),
    leaf('1140', 'المخزون', '1', 'ASSET', 'inventory', { role: 'inventory', allowManual: false }),
    leaf('1150', 'ضريبة القيمة المضافة — مدخلات', '1', 'ASSET', 'tax', { role: 'vatInput', allowManual: false }),
    leaf('2100', 'الموردين', '2', 'LIABILITY', 'payable', { role: 'payable', requiresParty: true }),
    leaf('2150', 'ضريبة القيمة المضافة — مخرجات', '2', 'LIABILITY', 'tax', { role: 'vatOutput', allowManual: false }),
    leaf('2155', 'ضريبة القيمة المضافة — صافي مستحق', '2', 'LIABILITY', 'tax', { role: 'vatPayable' }),
    leaf('3100', 'رأس المال', '3', 'EQUITY', 'equity', { role: 'capital' }),
    leaf('3250', 'الأرباح المحتجزة', '3', 'EQUITY', 'equity', { role: 'retainedEarnings' }),
    leaf('3300', 'صافي ربح الفترة (افتراضي — يُحسب)', '3', 'EQUITY', 'equity', { role: 'currentEarnings', allowManual: false }),
    leaf('3400', 'المسحوبات الشخصية', '3', 'EQUITY', 'equity', { role: 'drawings', contra: true }),
    leaf('3900', 'أرصدة افتتاحية (مؤقت)', '3', 'EQUITY', 'equity', { role: 'openingBalanceEquity' }),
    leaf('4100', 'مبيعات البضائع', '4', 'REVENUE', 'revenue', { role: 'sales' }),
    leaf('4200', 'مرتجعات ومسموحات المبيعات', '4', 'REVENUE', 'revenue', { role: 'salesReturns', contra: true }),
    leaf('4300', 'إيرادات أخرى', '4', 'REVENUE', 'otherIncome', { role: 'otherIncome' }),
    leaf('5100', 'تكلفة البضاعة المباعة', '5', 'EXPENSE', 'costOfSales', { role: 'cogs', allowManual: false }),
    leaf('5110', 'فروقات جرد المخزون', '5', 'EXPENSE', 'costOfSales', { role: 'inventoryVariance', allowManual: false }),
    leaf('5120', 'بضاعة تالفة ومنتهية الصلاحية', '5', 'EXPENSE', 'costOfSales', { role: 'inventoryWriteOff' }),
    leaf('6110', 'إيجار ورواتب', '6', 'EXPENSE', 'operatingExpense'),
    leaf('6120', 'مصروفات تشغيل عامة', '6', 'EXPENSE', 'operatingExpense'),
    leaf('6130', 'مواد تغليف وأكياس', '6', 'EXPENSE', 'operatingExpense'),
    leaf('6140', 'رسوم بنكية وعمولات بطاقات', '6', 'EXPENSE', 'operatingExpense', { role: 'cardFees' }),
    leaf('6390', 'مصروفات متنوعة', '6', 'EXPENSE', 'otherExpense'),
  ];
}

/** Detailed template (مفصّل, ~90) — standard plus branch/cheque/GOSI/EOSB extras (§5). Kept additive to the standard set. */
export function detailedAccountRows(): Row[] {
  return [
    ...standardAccountRows(),
    leaf('1135', 'شيكات تحت التحصيل', '11', 'ASSET', 'otherCurrentAsset'),
    leaf('2110', 'شيكات تحت الدفع', '21', 'LIABILITY', 'currentLiability'),
    leaf('2185', 'قسائم شراء مصدرة', '21', 'LIABILITY', 'currentLiability'),
    leaf('6216', 'التأمينات الاجتماعية (جوسي)', '62', 'EXPENSE', 'operatingExpense'),
    leaf('6217', 'مكافأة نهاية الخدمة', '62', 'EXPENSE', 'operatingExpense'),
    leaf('1171', 'إيجار مدفوع مقدماً — لكل فرع', '11', 'ASSET', 'prepaid'),
  ];
}

/** SA country add-on (§5): zakat + GOSI + end-of-service provision — proof of the add-on mechanism. */
export function saAddonRows(): Row[] {
  return [leaf('6910', 'الزكاة', '69', 'EXPENSE', 'zakatTax', { role: 'zakat' })];
}

/** Pharmacy business add-on (§5): a zero/exempt-rated revenue line for the VAT return, and an expiry-aware write-off name. */
export function pharmacyAddonRows(): Row[] {
  return [leaf('4120', 'مبيعات أدوية معفاة/صفرية', '4', 'REVENUE', 'revenue')];
}

export type AccountTemplate = 'basic' | 'standard' | 'detailed';

export interface TemplateOptions {
  template?: AccountTemplate;
  country?: 'SA' | 'EG' | 'AE';
  businessType?: 'pharmacy' | 'clothing' | 'services' | 'retail' | string;
}

function rowsForTemplate(template: AccountTemplate): Row[] {
  if (template === 'basic') return basicAccountRows();
  if (template === 'detailed') return detailedAccountRows();
  return standardAccountRows();
}

/** Builds the account tree (with `id`s and resolved `parentId`s) for a template + add-ons. */
export function buildAccounts(opts: TemplateOptions = {}): Account[] {
  const rows = [...rowsForTemplate(opts.template ?? 'standard')];
  if (opts.country === 'SA') rows.push(...saAddonRows());
  if (opts.businessType === 'pharmacy') rows.push(...pharmacyAddonRows());

  const byCode = new Map(rows.map((r) => [r.code, r]));
  return rows.map((r) => ({
    id: `acc-${r.code}`,
    code: r.code,
    name: r.name,
    nameEn: r.nameEn,
    parentId: r.parentCode && byCode.has(r.parentCode) ? `acc-${r.parentCode}` : null,
    isGroup: r.isGroup,
    kind: r.kind,
    subtype: r.subtype,
    normalSide: r.normalSide,
    systemRole: r.role,
    requiresParty: r.requiresParty,
    allowManual: r.allowManual,
    active: true,
    canDelete: r.canDelete ?? true,
  }));
}

/** The demo seed's chart of accounts: standard template + SA add-on. */
export const accountsFixture: Account[] = buildAccounts({ template: 'standard', country: 'SA' });

export function fiscalYearFixture(now: Date): FiscalYear[] {
  const y = now.getFullYear();
  return [
    { id: `fy-${y - 1}`, name: String(y - 1), startDate: `${y - 1}-01-01`, endDate: `${y - 1}-12-31`, isClosed: true },
    { id: `fy-${y}`, name: String(y), startDate: `${y}-01-01`, endDate: `${y}-12-31`, isClosed: false },
  ];
}
