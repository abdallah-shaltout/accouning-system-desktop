import type { Component } from 'vue'
import { Pill, Shirt, ShoppingCart, Wrench } from 'lucide-vue-next'

export interface BusinessType {
  key: string
  icon: Component
  name: string
  scenario: string
  fits: string
}

/** S10 — replaces the reference's testimonials with honest business-type scenarios (decision D6). */
export const BUSINESS_TYPES: BusinessType[] = [
  {
    key: 'grocery',
    icon: ShoppingCart,
    name: 'سوبر ماركت وبقالة',
    scenario:
      'باركود سريع على الكاشير، ورديات صباحية ومسائية بتقفيل يومي، وأسعار جملة وقطاعي لنفس الصنف. ولما الصنف يقرب يخلص، ايكوال ينبهك قبل ما الزبون يسأل.',
    fits: 'بقالة · سوبر ماركت · مخبوزات',
  },
  {
    key: 'fashion',
    icon: Shirt,
    name: 'محلات ملابس وأحذية',
    scenario:
      'مقاسات وألوان بحقول مخصصة، مرتجعات موسم مربوطة بفواتيرها، وجرد آخر الموسم من غير ما تقفل يوم واحد.',
    fits: 'ملابس · أحذية · إكسسوارات',
  },
  {
    key: 'parts',
    icon: Wrench,
    name: 'قطع غيار وورش',
    scenario:
      'رقم الصنف والبديل له، فاتورة فيها قطع وخدمة صنعة مع بعض، وكشف حساب لكل عميل ورشة.',
    fits: 'قطع غيار · ورش صيانة · أدوات',
  },
  {
    key: 'pharmacy',
    icon: Pill,
    name: 'صيدليات ومستلزمات',
    scenario:
      'تشغيلات بتواريخ صلاحية، إنذار قبل انتهاء الصلاحية، وإرجاع التالف للمورد بمستند خصم.',
    fits: 'صيدليات · مستلزمات طبية · مستحضرات',
  },
]
