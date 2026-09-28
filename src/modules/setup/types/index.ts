/**
 * v2 phase 5 (docs/v2/05-onboarding.md §2): the 11-step setup wizard's own local types. Nothing
 * here is persisted directly — each step's `commit()` writes into the real domain tables
 * (accounts, branches, customers, settings…) through the existing services / new `setupService`.
 */
import { COUNTRY_PROFILES, DEFAULT_COUNTRY } from '@/modules/core/helpers/countryProfiles';
import type { CountryCode } from '@/modules/core/helpers/countryProfiles';
import type { Address } from '@/modules/core/types/address';
import type { AccountTemplate } from '@/mocks/fixtures/accounts';

export type BusinessType =
    | "clothing"
    | "pharmacy"
    | "supermarket"
    | "electronics"
    | "retail"
    | "services"
    | "wholesale";

export const BUSINESS_TYPES: {
    value: BusinessType;
    label: string;
    note: string;
}[] = [
    { value: "retail", label: "تجزئة عامة", note: "دليل حسابات ومنتجات عامة" },
    { value: "clothing", label: "ملابس", note: "مقاسات وألوان كخصائص للمنتج" },
    {
        value: "pharmacy",
        label: "صيدلية",
        note: "تواريخ انتهاء صلاحية وتتبع تشغيلات إلزامي",
    },
    {
        value: "supermarket",
        label: "سوبر ماركت",
        note: "وحدات متعددة (كرتونة → قطعة) ومنتجات كثيرة",
    },
    {
        value: "electronics",
        label: "إلكترونيات",
        note: "أرقام تسلسلية وضمانات",
    },
    {
        value: "services",
        label: "خدمات",
        note: "بدون مخزون — فواتير خدمات فقط",
    },
    {
        value: "wholesale",
        label: "جملة",
        note: "حدود ائتمان وأسعار متدرجة حسب الكمية",
    },
];

export interface WizardStepMeta {
    key: string;
    label: string;
    required: boolean;
    why: string;
}

export const WIZARD_STEPS: WizardStepMeta[] = [
    {
        key: "businessType",
        label: "نوع النشاط",
        required: true,
        why: "يضبط الوحدات والحقول وشجرة الحسابات المقترح تلقائياً.",
    },
    {
        key: "countryTax",
        label: "الدولة والعملة والضريبة",
        required: true,
        why: "تحدد العملة الأساسية ونسبة الضريبة الافتراضية — تُقفل العملة بعد أول ترحيل.",
    },
    {
        key: "company",
        label: "بيانات المنشأة",
        required: true,
        why: "تظهر في رأس كل فاتورة ومستند مطبوع.",
    },
    {
        key: "fiscalYear",
        label: "السنة المالية وتاريخ البدء",
        required: true,
        why: "تاريخ البدء هو تاريخ الأرصدة الافتتاحية — يُقفل بعد ترحيلها.",
    },
    {
        key: "branches",
        label: "الفروع",
        required: true,
        why: "كل مستند وحركة مخزون مرتبطة بفرع.",
    },
    {
        key: "coa",
        label: "شجرة الحسابات",
        required: true,
        why: "أساس كل التقارير المالية — اختر المستوى المناسب لحجم عملك.",
    },
    {
        key: "paymentMethods",
        label: "طرق الدفع",
        required: true,
        why: "تظهر كأزرار تحصيل في نقطة البيع وشاشة السداد.",
    },
    {
        key: "opening",
        label: "الأرصدة الافتتاحية",
        required: false,
        why: "لنقل أرصدة العملاء والموردين والمخزون والنقدية من النظام القديم.",
    },
    {
        key: "users",
        label: "المستخدمون",
        required: false,
        why: "أضف الكاشير والمحاسب وأمين المخزن الآن أو لاحقاً من الإعدادات.",
    },
    {
        key: "printing",
        label: "الطباعة",
        required: false,
        why: "يحدد شكل الفاتورة المطبوعة وحجم الطابعة الحرارية.",
    },
    {
        key: "ready",
        label: "جاهز",
        required: true,
        why: "مراجعة أخيرة قبل بدء العمل.",
    },
];

export interface WizardState {
    businessType: BusinessType;
    company: {
        nameAr: string;
        nameEn: string;
        logo?: string;
        type: "individual" | "company";
        vatNumber: string;
        crNumber: string;
        /** doc 18.E: the address picker's output (region/city/district + street/building), country-driven by `countryTax.country`. */
        nationalAddress: Address;
        phone: string;
        email: string;
    };
    countryTax: {
        country: "EG" | "SA";
        currency: string;
        vatRegistered: boolean;
        pricesIncludeTax: boolean;
        extraCurrencies: { code: string; rate: number }[];
    };
    fiscalYear: {
        startMonth: number; // 1-12
        startDay: number;
        goLiveDate: string;
    };
    branches: { id?: string; name: string; code: string; address?: Address }[];
    coa: { template: "basic" | "standard" | "detailed" };
    paymentMethods: {
        id?: string;
        key: string;
        name: string;
        type: string;
        accountRole: string;
        active: boolean;
    }[];
    openingDone: boolean;
    users: { name: string; username: string; role: string; pin?: string }[];
    printing: {
        templateId?: string;
        printerMode: "a4" | "thermal";
        thermalWidth: 58 | 80;
    };
}

// --- D10 snapshot importer (21.03 §00-import, Rust-only — Tauri desktop builds) -------------------

/** `mode` distinguishes the shipped "import from previous version" flow (job 1) from the dev-only
 * demo-data reseed (job 2) — `infrastructure::import::run::import_snapshot`'s own two callers. */
export type ImportMode = "legacy" | "demo";

/** One branch as it appears in the snapshot, before id remapping — used only to let the one-time
 * import screen ask "which branch do these print templates belong to" when the snapshot has more
 * than one (cross-cutting.md §9's "ask, never guess" rule). */
export interface LegacySnapshotBranch {
    id: string;
    name: string;
    code: string;
}

/** `setup_inspect_legacy_snapshot`'s return: enough for the one-time import screen to show
 * "استيراد بيانات شركتك السابقة (٤٣٢ منتج، ١٢٠ فاتورة...)" before committing to the import. */
export interface LegacySnapshotSummary {
    schemaVersion: number;
    savedAt?: string;
    company: string;
    /** Same keys/order as `backupArchive.ts`'s `tableCounts()` (array tables only, then `attachments: 0`). */
    counts: Record<string, number>;
    branches: LegacySnapshotBranch[];
    hasTemplates: boolean;
    targetEmpty: boolean;
}

/** `setup_import_snapshot`'s return. */
export interface ImportSnapshotResult {
    /** Post-import row counts, same keys as `LegacySnapshotSummary.counts`. */
    counts: Record<string, number>;
    /** Snapshot values that fell outside their column's decimal scale and were rounded on import
     * (D-4 — zero silent change; the count is shown to the user, never hidden). */
    roundedValues: number;
    defaultBranchId: string;
}

// --- 02-setup (W2) Rust DTOs --------------------------------------------------------------------
//
// Plan 21 Part 03 §2 ("TS moves first") calls for these to be the ONE source of truth, with
// `src/mocks/backend/setup.ts` / `opening.ts` / `fixtures/accounts.ts` importing them back instead
// of declaring their own. That migration touches files outside `src/modules/setup/**` (this wave's
// edit boundary) — done here as additive exports so `setupService.ts`'s Rust switch lines have a
// contract to check against; the manager still needs to point the mock files at these (see this
// wave's final report's "Needs from manager").

/** Moved from `setupService.ts` (setup.md §2 "TS moves first") — the Rust `OnboardingProgress` DTO
 * mirrors this field-for-field (`skipped`/`done` always present, never `undefined`). */
export interface OnboardingProgress {
    businessType?: string;
    goLiveDate?: string;
    completedStep?: number;
    skipped: string[];
    done: string[];
    finishedAt?: string;
    openingEntryId?: string;
    closingEntryId?: string;
    coaTemplate?: AccountTemplate;
}

/** `Partial<OnboardingProgress>` — a key absent means "unchanged" (`saveOnboardingProgress`). */
export type OnboardingProgressPatch = Partial<OnboardingProgress>;

export interface CountryTaxInput {
    country: CountryCode;
    currency: string;
    vatRegistered?: boolean;
    pricesIncludeTax: boolean;
    extraCurrencies: { code: string; rate: number }[];
}

export type PostOpeningBalancesResult = { openingEntryId: string; closingEntryId?: string };

export type CloseTarget = 'capital' | 'ownerCurrent';

// --- Moved from `src/mocks/backend/{setup,opening}.ts` (setup.md §2 "TS moves first") -----------
//
// These are the wire shapes the Rust `domains/setup` DTOs mirror field-for-field; the mock files
// (`src/mocks/backend/setup.ts` / `opening.ts`) import them back type-only instead of redeclaring
// them, so the contract has exactly one hand-written source per setup.md §2.

export interface WizardBranchInput {
    name: string;
    code: string;
    address?: Address;
}

export interface WizardPaymentMethodInput {
    name: string;
    type: 'cash' | 'card' | 'bank_transfer' | 'wallet' | 'credit' | 'store_credit';
    accountRole: 'cash' | 'bank' | 'cardClearing' | 'walletClearing' | 'receivable';
    active: boolean;
}

export interface OpeningCashLine {
    /** 'cash' role (a drawer) or 'bank' role account id — always a specific account, not a role, since a company can have several. */
    accountId: string;
    amount: number;
    currency?: string;
    amountFc?: number;
    rate?: number;
}

export interface OpeningPartyLine {
    partyKind: 'customer' | 'supplier';
    partyId: string;
    amount: number;
    /** 'debit' = the customer owes us / we owe the supplier less; mirrors the party-form stub's `side`. */
    side: 'debit' | 'credit';
}

export interface OpeningOtherLine {
    accountId: string;
    side: 'debit' | 'credit';
    amount: number;
    description?: string;
}

export interface OpeningEntryInput {
    date: string;
    cash: OpeningCashLine[];
    customers: OpeningPartyLine[];
    suppliers: OpeningPartyLine[];
    other: OpeningOtherLine[];
    createdBy: string;
}

export interface OpeningStockLine {
    productId: string;
    qty: number;
    unitCost: number;
    batchNo?: string;
    expiryDate?: string;
}

export interface PartyOpeningBalanceInput {
    partyKind: 'customer' | 'supplier';
    partyId: string;
    amount: number;
    side: 'debit' | 'credit';
    asOfDate: string;
    createdBy: string;
}

/** `Omit<OpeningEntryInput, 'createdBy'>` (setup.md §2) — the wire shape `postOpeningBalances` sends. */
export type PostOpeningBalancesInput = Omit<OpeningEntryInput, 'createdBy'>;

/** `Omit<PartyOpeningBalanceInput, 'createdBy'>` (setup.md §2) — the wire shape `postPartyOpening` sends. */
export type PartyOpeningInput = Omit<PartyOpeningBalanceInput, 'createdBy'>;

// --- Device setup (Part 02 handoff §9, D-1/D-2) -------------------------------------------------

export type DeviceRole = 'main' | 'terminal';

export interface DeviceSetupState {
    configured: boolean;
    role: DeviceRole;
    /** False when this machine can't host the embedded database (no bundled MariaDB payload) —
     * the "main device" card is disabled with an explanatory note in that case. */
    canHostDatabase: boolean;
    /** Whether the connected database (main or paired-to) already has at least one user row. */
    hasUsers: boolean;
}

export interface PairTerminalInput {
    host: string;
    port: number;
    code: string;
}

export function defaultWizardState(): WizardState {
    const today = new Date().toISOString().slice(0, 10);
    // v2 doc 18.D (decision 2): default country is Egypt, not Saudi — `countryProfiles.ts` is the
    // one owner of the actual rate/currency/label values; only the *default choice* lives here.
    const defaultProfile = COUNTRY_PROFILES[DEFAULT_COUNTRY];
    return {
        businessType: "retail",
        company: {
            nameAr: "",
            nameEn: "",
            type: "company",
            vatNumber: "",
            crNumber: "",
            nationalAddress: { country: defaultProfile.code },
            phone: "",
            email: "",
        },
        countryTax: {
            country: defaultProfile.code,
            currency: defaultProfile.currency.code,
            vatRegistered: true,
            pricesIncludeTax: defaultProfile.vat.pricesIncludeTaxDefault,
            extraCurrencies: [],
        },
        fiscalYear: { startMonth: 1, startDay: 1, goLiveDate: today },
        branches: [{ name: "الفرع الرئيسي", code: "MAIN", address: { country: defaultProfile.code } }],
        coa: { template: "standard" },
        paymentMethods: [],
        openingDone: false,
        users: [],
        printing: { printerMode: "a4", thermalWidth: 80 },
    };
}
