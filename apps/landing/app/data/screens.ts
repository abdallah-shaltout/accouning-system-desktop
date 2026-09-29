/**
 * Real Equal screens used across the landing (public/screens/*.png). They are captured from the
 * desktop app's demo data by `scripts/app-shots.py` at 2x, so width/height are device pixels.
 * Shots that sit side by side share one aspect ratio (cards ~0.775, team panels 1.8) so their
 * boxes follow the image and line up. The full per-feature screens are on ACCORDION_FEATURES.
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
    height: 1084,
    title: 'نقطة البيع',
    alt: 'سلة نقطة البيع: ثلاثة أصناف بكمياتها، والمجموع وضريبة القيمة المضافة والإجمالي 516 ج.م وزر الدفع F12',
  },
  products: {
    src: '/screens/crop-products.png',
    width: 1312,
    height: 1692,
    title: 'المنتجات',
    alt: 'قائمة المنتجات: لكل صنف سعر البيع والتكلفة بمتوسط التكلفة وهامش الربح والكمية في المخزون',
  },
  health: {
    src: '/screens/crop-health.png',
    width: 1280,
    height: 1652,
    title: 'الصحة المالية للمنشأة',
    alt: 'تقرير الصحة المالية للمنشأة: نتيجة إجمالية من 100، ودرجات السيولة والربحية والمديونية والتحصيل',
  },
  users: {
    src: '/screens/crop-users.png',
    width: 2016,
    height: 1120,
    title: 'المستخدمين',
    alt: 'قائمة المستخدمين: لكل موظف صلاحيته (مدير، محاسب، كاشير، أمين مخزن) وأقصى خصم مسموح وقائمة الأسعار والحالة',
  },
  roles: {
    src: '/screens/crop-roles.png',
    width: 2240,
    height: 1244,
    title: 'المستخدمون والأدوار',
    alt: 'مصفوفة الصلاحيات: لكل دور صلاحية كاملة أو عرض أو لا شيء في كل قسم من البرنامج',
  },
} satisfies Record<string, AppShot>
