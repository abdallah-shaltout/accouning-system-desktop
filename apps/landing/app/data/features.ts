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
  /** The real screen crop shown on the card (data/screens.ts). */
  shot: 'cart' | 'reorder' | 'health'
  /** How much wider than the card the window is (1 = fits); wide crops bleed off the end edge. */
  bleed: number
  title: string
  body: string
}

/** S4 — three product cards on coral. */
export const PRODUCT_CARDS: ProductCard[] = [
  {
    shot: 'cart',
    bleed: 1,
    title: 'بيع بثقة.',
    body: 'فاتورة ضريبية في ثوانٍ، من الكاشير أو من المكتب، بضريبة محسوبة صح — دايمًا.',
  },
  {
    shot: 'reorder',
    bleed: 1.7,
    title: 'مخزونك تحت عينك.',
    body: 'كل حركة صنف مسجّلة بمتوسط التكلفة، وتنبيه قبل ما الصنف يخلص.',
  },
  {
    shot: 'health',
    bleed: 1.7,
    title: 'قراراتك بالأرقام.',
    body: 'ربحيتك وسيولتك وتحصيلك في رقم واحد، و28 تقريرًا جاهزًا من غير إكسل.',
  },
]

export interface AccordionFeature {
  key: 'pos' | 'invoices' | 'stock' | 'purchases' | 'reports'
  title: string
  body: string
  /** The real app screen shown for this feature (public/screens/<key>.png, from scripts/app-shots.py). */
  shot: { title: string; alt: string }
}

/** S6 — feature accordion, mapped to real modules of the desktop app. */
export const ACCORDION_FEATURES: AccordionFeature[] = [
  {
    key: 'pos',
    title: 'كاشير سريع POS',
    body: 'افتح وردية، بيع بالباركود، علّق فاتورة وارجع لها — والشاشة كلها تشتغل بالكيبورد.',
    shot: { title: 'نقطة البيع', alt: 'شاشة نقطة البيع: سلة فيها أربعة أصناف، والإجمالي وضريبة القيمة المضافة وزر الدفع F12' },
  },
  {
    key: 'invoices',
    title: 'فواتير ومرتجعات',
    body: 'فاتورة ضريبية أو عرض سعر يتحول لفاتورة، ومرتجع مربوط بفاتورته الأصلية.',
    shot: { title: 'الفواتير', alt: 'قائمة فواتير المبيعات مع التاريخ والعميل وطريقة الدفع وحالة السداد' },
  },
  {
    key: 'stock',
    title: 'مخزون وجرد دوري',
    body: 'جرد بدون إغلاق المحل، تسويات بمستندات، وتتبع تواريخ الصلاحية بالتشغيلة.',
    shot: { title: 'المنتجات', alt: 'قائمة المنتجات مع سعر البيع والتكلفة وهامش الربح والكمية في المخزون' },
  },
  {
    key: 'purchases',
    title: 'مشتريات وموردون',
    body: 'أمر شراء، استلام جزئي، وإشعار خصم — وكشف حساب المورد جاهز في أي لحظة.',
    shot: { title: 'أوامر الشراء', alt: 'قائمة أوامر الشراء مع المورد وحالة الاستلام والسداد والمتبقي للمورد' },
  },
  {
    key: 'reports',
    title: 'تقارير ومؤشرات',
    body: 'ميزان مراجعة، أرباح وخسائر، أعمار ديون — 28 تقريرًا تتصدّر وتُطبع.',
    shot: { title: 'قائمة الدخل', alt: 'تقرير قائمة الدخل: صافي الربح وهامش مجمل الربح والإيرادات لكل حساب' },
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
