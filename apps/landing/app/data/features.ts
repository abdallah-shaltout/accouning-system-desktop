import type { Component } from 'vue'
import {
  BadgeCheck,
  Boxes,
  Building2,
  ChartColumn,
  DatabaseBackup,
  History,
  KeyRound,
  Languages,
  Network,
  Receipt,
  ScanBarcode,
  ShieldCheck,
  WifiOff,
} from 'lucide-vue-next'

export interface IconLabel {
  icon: Component
  label: string
}

/** S2 — capability ticker (replaces the reference's client-logo strip, decision D4). */
export const TICKER: IconLabel[] = [
  { icon: Receipt, label: 'فواتير ضريبية' },
  { icon: ScanBarcode, label: 'كاشير POS' },
  { icon: Boxes, label: 'مخزون وجرد' },
  { icon: ChartColumn, label: 'تقارير جاهزة' },
  { icon: DatabaseBackup, label: 'نسخ احتياطي' },
  { icon: Building2, label: 'متعدد الفروع' },
  { icon: WifiOff, label: 'يعمل بدون إنترنت' },
  { icon: Languages, label: 'واجهة عربية 100%' },
]

export interface ProductCard {
  mock: 'invoice' | 'stock' | 'report'
  title: string
  body: string
}

/** S4 — three product cards on coral. */
export const PRODUCT_CARDS: ProductCard[] = [
  {
    mock: 'invoice',
    title: 'بيع بثقة.',
    body: 'فاتورة ضريبية في ثوانٍ، من الكاشير أو من المكتب، بضريبة محسوبة صح — دايمًا.',
  },
  {
    mock: 'stock',
    title: 'مخزونك تحت عينك.',
    body: 'كل حركة صنف مسجّلة بمتوسط التكلفة، وتنبيه قبل ما الصنف يخلص.',
  },
  {
    mock: 'report',
    title: 'قراراتك بالأرقام.',
    body: 'مبيعات اليوم، أرباحك، وأصنافك الأكثر بيعًا — تقارير جاهزة من غير إكسل.',
  },
]

export interface AccordionFeature {
  key: 'pos' | 'invoices' | 'stock' | 'purchases' | 'reports'
  title: string
  body: string
}

/** S6 — feature accordion, mapped to real modules of the desktop app. */
export const ACCORDION_FEATURES: AccordionFeature[] = [
  {
    key: 'pos',
    title: 'كاشير سريع POS',
    body: 'افتح وردية، بيع بالباركود، علّق فاتورة وارجع لها — والشاشة كلها تشتغل بالكيبورد.',
  },
  {
    key: 'invoices',
    title: 'فواتير ومرتجعات',
    body: 'فاتورة ضريبية أو عرض سعر يتحول لفاتورة، ومرتجع مربوط بفاتورته الأصلية.',
  },
  {
    key: 'stock',
    title: 'مخزون وجرد دوري',
    body: 'جرد بدون إغلاق المحل، تسويات بمستندات، وتتبع تواريخ الصلاحية بالتشغيلة.',
  },
  {
    key: 'purchases',
    title: 'مشتريات وموردون',
    body: 'أمر شراء، استلام جزئي، وإشعار خصم — وكشف حساب المورد جاهز في أي لحظة.',
  },
  {
    key: 'reports',
    title: 'تقارير ومؤشرات',
    body: 'ميزان مراجعة، أرباح وخسائر، أعمار ديون — 28 تقريرًا تتصدّر وتُطبع.',
  },
]

export interface RuleItem {
  title: string
  body: string
}

export interface RuleMode {
  key: 'single' | 'multi'
  label: string
  items: RuleItem[]
}

/** S7 — "دفاترك. قواعدك." segmented content. */
export const RULE_MODES: RuleMode[] = [
  {
    key: 'single',
    label: 'محل واحد',
    items: [
      { title: 'وردية اليوم', body: 'افتح وردية، اقفلها بتقرير X، وكل جنيه له سند.' },
      { title: 'سندات قبض وصرف', body: 'كل حركة كاش موثّقة ومربوطة بحسابها تلقائيًا.' },
      { title: 'نسخة احتياطية تلقائية', body: 'نسخة مشفّرة كل يوم على المكان اللي تختاره.' },
    ],
  },
  {
    key: 'multi',
    label: 'عدة فروع',
    items: [
      { title: 'تحويلات بين الفروع', body: 'صنف يخرج من فرع ويوصل للتاني بمستند تحويل كامل.' },
      { title: 'صلاحيات لكل دور', body: 'الكاشير يشوف الكاشير بس — والمدير يشوف كل حاجة.' },
      { title: 'مقارنة بين الفروع', body: 'تقرير واحد يوريك أي فرع بيكسب فعلًا.' },
    ],
  },
]

/** S9 — hairline feature grid under the team panels (3 + 2). */
export const TEAM_FEATURES: IconLabel[] = [
  { icon: KeyRound, label: 'صلاحيات وأدوار' },
  { icon: BadgeCheck, label: 'موافقات المدير' },
  { icon: History, label: 'سجل تدقيق كامل' },
  { icon: Network, label: 'مزامنة عبر الشبكة المحلية' },
  { icon: ShieldCheck, label: 'نسخ احتياطي مشفّر' },
]

/** S8 — roles shown in the honeycomb (real roles in the desktop app). */
export const ROLE_LABELS = { start: 'كاشير', end: 'مدير' } as const
