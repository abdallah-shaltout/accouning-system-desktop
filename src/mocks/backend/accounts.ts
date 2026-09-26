/**
 * Account lookups for the posting rules (docs/v2/02-accounting-review.md F3, docs/v2/03-chart-of-
 * accounts.md §2). This is the ONLY place in the mock backend that should know account codes —
 * everything else calls `accountFor(role, ctx)`. Nothing outside `src/mocks/backend/*` should look
 * accounts up by code either; pages go through `modules/accounting/services`.
 */
import type { Account, SystemRole } from "@/modules/accounting/types";
import { db } from "../db";
import { ApiError } from "../utils";

const ROLE_LABEL: Record<SystemRole, string> = {
    cash: "الصندوق",
    bank: "البنك",
    cardClearing: "تسوية البطاقات",
    walletClearing: "تسوية المحافظ الإلكترونية",
    receivable: "العملاء",
    inventory: "المخزون",
    inventoryInTransit: "بضاعة بالطريق بين الفروع",
    vatInput: "ضريبة القيمة المضافة — مدخلات",
    payable: "الموردين",
    vatOutput: "ضريبة القيمة المضافة — مخرجات",
    vatPayable: "ضريبة القيمة المضافة — صافي مستحق",
    customerAdvances: "دفعات مقدمة من العملاء",
    capital: "رأس المال",
    ownerCurrent: "جاري المالك",
    drawings: "المسحوبات الشخصية",
    retainedEarnings: "الأرباح المحتجزة",
    currentEarnings: "صافي ربح الفترة",
    openingBalanceEquity: "أرصدة افتتاحية",
    sales: "مبيعات البضائع",
    serviceRevenue: "إيرادات الخدمات",
    salesReturns: "مرتجعات المبيعات",
    otherIncome: "إيرادات أخرى",
    fxGain: "أرباح فروق العملة",
    cashOver: "زيادة الصندوق",
    purchaseDiscounts: "خصم مكتسب من الموردين",
    cogs: "تكلفة البضاعة المباعة",
    inventoryVariance: "فروقات جرد المخزون",
    inventoryWriteOff: "بضاعة تالفة ومنتهية الصلاحية",
    freightIn: "شحن وتخليص المشتريات",
    cardFees: "عمولات البطاقات",
    bankFees: "رسوم بنكية",
    fxLoss: "خسائر فروق العملة",
    cashShort: "عجز الصندوق",
    badDebt: "ديون معدومة",
    depreciation: "مصروف الإهلاك",
    zakat: "الزكاة",
};

export interface AccountCtx {
    branchId?: string;
    currency?: string;
}

/**
 * Resolves a system role to its account. Several accounts may share a role in principle (branch
 * cash drawers, per-currency banks — `ctx` is reserved for that), but Phase 1 seeds exactly one
 * account per role, so this always returns the single match. Throws an Arabic ApiError naming the
 * missing role if the CoA has no account for it (e.g. a template that dropped an optional role).
 */
export function accountFor(role: SystemRole, ctx: AccountCtx = {}): Account {
    const candidates = db.accounts.filter(
        (a) => a.systemRole === role && a.active,
    );
    const account =
        (ctx.branchId && candidates.find((a) => a.branchId === ctx.branchId)) ||
        (ctx.currency && candidates.find((a) => a.currency === ctx.currency)) ||
        candidates.find((a) => !a.branchId && !a.currency) ||
        candidates[0];
    if (!account)
        throw new ApiError(
            `لا يوجد حساب في شجرة الحسابات لدور "${ROLE_LABEL[role] ?? role}" — أضف حساباً بهذا الدور أولاً`,
            "NOT_FOUND",
        );
    return account;
}

export function accountById(id: string): Account {
    const account = db.accounts.find((a) => a.id === id);
    if (!account)
        throw new ApiError("الحساب غير موجود في شجرة الحسابات", "NOT_FOUND");
    return account;
}

/** Cash for cash payments, Bank for card / transfer / credit settlement. Credit itself never settles here. */
export function settlementAccountFor(
    method: string,
    ctx: AccountCtx = {},
): Account {
    return method === "cash"
        ? accountFor("cash", ctx)
        : accountFor("bank", ctx);
}

// ---------------------------------------------------------------------------------------------
// v2 phase 6 (docs/v2/07-products-and-inventory.md "Account resolution"): product → category →
// settings default, for the product form's "الضريبة والحسابات" tab and for Phase 7/8 postings that
// want a per-product revenue/COGS/purchase account instead of the flat `accountFor(role)` default.
// Every resolver here degrades gracefully to the role account when nothing overrides it, so callers
// that don't care about per-product accounts (Phase 3's sales posting today) are unaffected.
// ---------------------------------------------------------------------------------------------

import type { Category, Product } from "@/modules/products/types";

/** Revenue account: product override → category override → `sales`/`serviceRevenue` role default. */
export function revenueAccountFor(
    product: Product,
    category: Category | undefined,
): Account {
    if (product.revenueAccountId) return accountById(product.revenueAccountId);
    if (category?.revenueAccountId)
        return accountById(category.revenueAccountId);
    return accountFor(product.type === "service" ? "serviceRevenue" : "sales");
}

/** COGS account: product override → category override → `cogs` role default. */
export function cogsAccountFor(
    product: Product,
    category: Category | undefined,
): Account {
    if (product.cogsAccountId) return accountById(product.cogsAccountId);
    if (category?.cogsAccountId) return accountById(category.cogsAccountId);
    return accountFor("cogs");
}

/** Purchase/expense account for a non-stock or service line: product → category → settings default → `freightIn` fallback. */
export function purchaseAccountFor(
    product: Product,
    category: Category | undefined,
    defaultPurchaseAccountId?: string,
): Account {
    if (product.purchaseAccountId)
        return accountById(product.purchaseAccountId);
    if (category?.purchaseAccountId)
        return accountById(category.purchaseAccountId);
    if (defaultPurchaseAccountId) return accountById(defaultPurchaseAccountId);
    return accountFor("freightIn");
}

/** Sale/purchase tax id: product override → category override → undefined (caller falls back to the store default tax). */
export function saleTaxIdFor(
    product: Product,
    category: Category | undefined,
): string | undefined {
    return product.saleTaxId ?? category?.saleTaxId;
}

export function purchaseTaxIdFor(
    product: Product,
    category: Category | undefined,
): string | undefined {
    return product.purchaseTaxId ?? category?.purchaseTaxId;
}
