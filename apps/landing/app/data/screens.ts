/**
 * Real Equal screens used across the landing (public/screens/*.png). They are captured from the
 * desktop app's demo data by `scripts/app-shots.py` at 2x, so width/height are device pixels.
 * The full per-feature screens are listed on ACCORDION_FEATURES (data/features.ts).
 */
export interface AppShot {
  src: string
  width: number
  height: number
  /** Window title shown above the shot: the screen's name in the app. */
  title: string
  alt: string
}

export const SHOTS = {
  cart: {
    src: '/screens/crop-cart.png',
    width: 840,
    height: 1424,
    title: 'نقطة البيع',
    alt: 'سلة نقطة البيع: أربعة أصناف بكمياتها، والمجموع وضريبة القيمة المضافة والإجمالي 655 ج.م وزر الدفع F12',
  },
  reorder: {
    src: '/screens/crop-reorder.png',
    width: 2032,
    height: 660,
    title: 'المخزون المنخفض وإعادة الطلب',
    alt: 'تقرير المخزون المنخفض: صنفان بحاجة لإعادة طلب، مع الكمية الحالية والحد الأدنى والكمية المقترحة للطلب',
  },
  health: {
    src: '/screens/crop-health.png',
    width: 2032,
    height: 840,
    title: 'الصحة المالية للمنشأة',
    alt: 'تقرير الصحة المالية: نتيجة 92.54 من 100، ودرجات السيولة والربحية والمديونية والتحصيل',
  },
  analytics: {
    src: '/screens/crop-analytics.png',
    width: 2032,
    height: 600,
    title: 'التحليلات',
    alt: 'رسم أعمدة لاتجاه المبيعات اليومية لآخر 30 يومًا في شاشة التحليلات',
  },
  roles: {
    src: '/screens/crop-roles.png',
    width: 2320,
    height: 1280,
    title: 'المستخدمون والأدوار',
    alt: 'مصفوفة الصلاحيات: لكل دور (مدير النظام، مدير المتجر، محاسب، كاشير، أمين مخزن) صلاحية كاملة أو عرض أو لا شيء في كل قسم',
  },
} satisfies Record<string, AppShot>
