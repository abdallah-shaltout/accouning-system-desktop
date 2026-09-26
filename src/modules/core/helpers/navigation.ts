import type { Component } from "vue";
import {
    AlertTriangle,
    ArrowLeftRight,
    BookOpen,
    CalendarRange,
    ChartColumn,
    ClipboardCheck,
    ClipboardList,
    CreditCard,
    FolderTree,
    HandCoins,
    House,
    LineChart,
    Package,
    Receipt,
    ReceiptText,
    Repeat,
    Settings,
    ShieldCheck,
    ShoppingBag,
    ShoppingCart,
    Tags,
    Truck,
    UserCog,
    Users,
    Wallet,
} from "@lucide/vue";
import type { Area } from "@/modules/users/types";

export interface NavItem {
    label: string;
    to: string;
    icon: Component;
    area: Area;
    /** Match only this exact path (otherwise any sub-path activates the item). */
    exact?: boolean;
}

export interface NavGroup {
    label?: string;
    /** Row icon for the collapsed accordion header (docs/v2/17-ui-system-rtl-themes.md Phase D). Omitted only for the single-item "الرئيسية" group, which renders as a plain link. */
    icon?: Component;
    items: NavItem[];
}

/**
 * Sidebar IA (docs/v2/17-ui-system-rtl-themes.md Phase D — ≈9 closed rows instead of ~33 flat
 * links). Items are filtered by role at render time; a group whose filtered items collapse to one
 * renders as a plain link instead of a one-item accordion (`NavMain.vue`).
 */
export const NAVIGATION: NavGroup[] = [
    {
        items: [
            {
                label: "الرئيسية",
                to: "/",
                icon: House,
                area: "dashboard",
                exact: true,
            },
        ],
    },
    {
        label: "المبيعات",
        icon: ShoppingCart,
        items: [
            {
                label: "نقطة البيع",
                to: "/pos",
                icon: ShoppingCart,
                area: "pos",
            },
            { label: "الورديات", to: "/pos/shifts", icon: Wallet, area: "pos" },
            {
                label: "الفواتير",
                to: "/invoices",
                icon: ReceiptText,
                area: "sales",
            },
            {
                label: "فاتورة جديدة",
                to: "/sales/invoices/new",
                icon: Receipt,
                area: "sales",
            },
            {
                label: "عروض الأسعار",
                to: "/sales/quotations",
                icon: ClipboardList,
                area: "sales",
            },
        ],
    },
    {
        label: "المخزون",
        icon: Package,
        items: [
            {
                label: "المنتجات",
                to: "/products",
                icon: Package,
                area: "inventory",
            },
            {
                label: "التصنيفات والوحدات",
                to: "/catalog/categories",
                icon: FolderTree,
                area: "inventory",
            },
            {
                label: "قوائم الأسعار",
                to: "/catalog/price-lists",
                icon: Tags,
                area: "inventory",
            },
            {
                label: "تسويات المخزون",
                to: "/inventory/adjustments",
                icon: ClipboardList,
                area: "inventory",
            },
            {
                label: "الجرد",
                to: "/inventory/counts",
                icon: ClipboardCheck,
                area: "inventory",
            },
            {
                label: "حركة المخزون",
                to: "/inventory/movements",
                icon: ArrowLeftRight,
                area: "inventory",
            },
            {
                label: "تقرير الصلاحية",
                to: "/inventory/expiry",
                icon: AlertTriangle,
                area: "inventory",
            },
            // v2 phase 9 (docs/v2/07 §4, deferred from phase 6): branch-to-branch stock transfers.
            {
                label: "تحويلات الفروع",
                to: "/inventory/transfers",
                icon: Truck,
                area: "inventory",
            },
        ],
    },
    {
        label: "العملاء والموردين",
        icon: Users,
        items: [
            {
                label: "العملاء",
                to: "/customers",
                icon: Users,
                area: "parties",
            },
            {
                label: "الموردين",
                to: "/suppliers",
                icon: Truck,
                area: "parties",
            },
        ],
    },
    {
        label: "المشتريات والمصروفات",
        icon: ShoppingBag,
        items: [
            {
                label: "أوامر الشراء",
                to: "/purchases",
                icon: ShoppingBag,
                area: "purchases",
            },
            {
                label: "المصروفات",
                to: "/expenses",
                icon: Receipt,
                area: "expenses",
            },
        ],
    },
    {
        label: "الحسابات",
        icon: BookOpen,
        items: [
            {
                label: "شجرة الحسابات",
                to: "/accounting/accounts",
                icon: BookOpen,
                area: "accounting",
            },
            {
                label: "القيود اليومية",
                to: "/accounting/journal",
                icon: Wallet,
                area: "accounting",
            },
            {
                label: "قوالب القيود المتكررة",
                to: "/accounting/journal-templates",
                icon: Repeat,
                area: "accounting",
            },
            {
                label: "تسوية ضريبة القيمة المضافة",
                to: "/accounting/vat-settlement",
                icon: Receipt,
                area: "accounting",
            },
            {
                label: "السنة المالية",
                to: "/accounting/fiscal-years",
                icon: CalendarRange,
                area: "accounting",
            },
        ],
    },
    {
        label: "المدفوعات",
        icon: HandCoins,
        items: [
            {
                label: "سندات القبض والصرف",
                to: "/payments",
                icon: HandCoins,
                area: "payments",
            },
            {
                label: "تسوية البطاقات",
                to: "/payments/settlements",
                icon: CreditCard,
                area: "payments",
            },
            {
                label: "السندات العامة",
                to: "/vouchers",
                icon: Wallet,
                area: "payments",
            },
        ],
    },
    {
        label: "التقارير",
        icon: ChartColumn,
        items: [
            {
                label: "التقارير",
                to: "/reports",
                icon: ChartColumn,
                area: "reports",
            },
            // v2 phase 10 (docs/v2/11-journal-dashboard-insights.md Part C).
            {
                label: "التحليلات",
                to: "/analytics",
                icon: LineChart,
                area: "analytics",
            },
        ],
    },
    {
        label: "الإدارة",
        icon: Settings,
        items: [
            // v2 phase 13b (docs/v2/14-platform.md §6): async manager approvals queue.
            {
                label: "طلبات الاعتماد",
                to: "/approvals",
                icon: ShieldCheck,
                area: "approvals",
            },
            { label: "المستخدمين", to: "/users", icon: UserCog, area: "users" },
            {
                label: "الإعدادات",
                to: "/settings",
                icon: Settings,
                area: "settings",
            },
        ],
    },
];

export interface QuickAction {
    label: string;
    to: string;
    icon: Component;
    area: Area;
}

/** "إجراءات سريعة" list below the groups (docs/v2/17 Phase D — the sidebar-07 "Projects" slot). Filtered by permission, hidden in icon mode. */
export const QUICK_ACTIONS: QuickAction[] = [
    { label: "بيع جديد", to: "/pos", icon: ShoppingCart, area: "pos" },
    {
        label: "فاتورة جديدة",
        to: "/sales/invoices/new",
        icon: Receipt,
        area: "sales",
    },
    { label: "سند قبض", to: "/payments", icon: HandCoins, area: "payments" },
];
