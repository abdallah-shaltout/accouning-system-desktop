/**
 * v2 phase 5 (docs/v2/05-onboarding.md §2): the 11-step setup wizard's own local types. Nothing
 * here is persisted directly — each step's `commit()` writes into the real domain tables
 * (accounts, branches, customers, settings…) through the existing services / new `setupService`.
 */
import { COUNTRY_PROFILES, DEFAULT_COUNTRY } from '@/modules/core/helpers/countryProfiles';
import type { Address } from '@/modules/core/types/address';

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
