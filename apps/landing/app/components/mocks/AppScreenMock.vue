<script setup lang="ts">
import { ScanBarcode } from 'lucide-vue-next'
import type { AccordionFeature } from '~/data/features'

/**
 * One light-theme Equal screen per accordion feature. All five share the same table skeleton
 * (title bar → rows → footer), which is also how the real app's document screens are built.
 */
const props = defineProps<{ screen: AccordionFeature['key'] }>()

interface Screen {
  title: string
  badge: string
  cols: string[]
  rows: string[][]
  footer: [string, string]
  hot?: number
}

const SCREENS: Record<AccordionFeature['key'], Screen> = {
  pos: {
    title: 'نقطة البيع · وردية الصباح',
    badge: 'مفتوحة',
    cols: ['الصنف', 'الكمية', 'الإجمالي'],
    rows: [['أرز مصري 5 كجم', '2', '310.00'], ['زيت عباد الشمس', '1', '96.50'], ['شاي ناعم 250 جم', '3', '142.50'], ['سكر أبيض 1 كجم', '4', '128.00'], ['مكرونة 400 جم', '6', '81.00']],
    footer: ['الإجمالي شامل الضريبة', '758.00'],
    hot: 2,
  },
  invoices: {
    title: 'فاتورة ضريبية #1048',
    badge: 'مدفوعة',
    cols: ['الصنف', 'السعر', 'الإجمالي'],
    rows: [['طقم أواني ستانلس', '1,250.00', '1,250.00'], ['خصم السطر', '−', '−125.00'], ['خلاط كهربائي', '890.00', '890.00'], ['ضريبة القيمة المضافة 14%', '', '282.10'], ['خصم الفاتورة', '', '−50.00']],
    footer: ['المستحق', '2,247.10'],
    hot: 1,
  },
  stock: {
    title: 'جرد دوري · مخزن رئيسي',
    badge: 'قيد المراجعة',
    cols: ['الصنف', 'الدفتري', 'المعدود'],
    rows: [['أرز مصري 5 كجم', '420', '420'], ['زيت عباد الشمس', '186', '183'], ['سكر أبيض 1 كجم', '38', '38'], ['شاي ناعم 250 جم', '264', '266'], ['مكرونة 400 جم', '640', '640']],
    footer: ['فروقات الجرد', '−3 / +2'],
    hot: 1,
  },
  purchases: {
    title: 'أمر شراء #PO-212',
    badge: 'مستلم جزئيًا',
    cols: ['الصنف', 'المطلوب', 'المستلم'],
    rows: [['زيت عباد الشمس', '240', '240'], ['سكر أبيض 1 كجم', '500', '320'], ['أرز مصري 5 كجم', '300', '300'], ['شاي ناعم 250 جم', '120', '0'], ['مكرونة 400 جم', '400', '400']],
    footer: ['المتبقي على المورد', '18,420.00'],
    hot: 1,
  },
  reports: {
    title: 'الأرباح والخسائر · سبتمبر',
    badge: 'مُحدّث',
    cols: ['البند', 'الشهر', 'التغير'],
    rows: [['المبيعات', '412,800', '+8.4%'], ['تكلفة المبيعات', '291,300', '+6.1%'], ['مجمل الربح', '121,500', '+14.2%'], ['المصروفات', '38,900', '−2.7%'], ['صافي الربح', '82,600', '+22.9%']],
    footer: ['هامش صافي الربح', '20.0%'],
    hot: 4,
  },
}

const s = computed(() => SCREENS[props.screen])
</script>

<template>
  <LWindowMock :title="s.title">
    <div class="p-4">
      <div class="flex items-center justify-between gap-3">
        <div class="flex h-8 flex-1 items-center gap-2 rounded-lg border border-panel-200 bg-panel-50 px-2.5 text-micro text-ink-mute">
          <ScanBarcode class="size-3.5" :stroke-width="1.75" />
          ابحث بالاسم أو امسح الباركود
        </div>
        <span class="rounded-full bg-coral-500/10 px-2.5 py-1 text-nano font-medium text-coral-600">{{ s.badge }}</span>
      </div>
      <table class="mt-3 w-full text-micro">
        <thead>
          <tr class="text-ink-mute">
            <th v-for="(c, i) in s.cols" :key="c" class="pb-2 font-normal" :class="i === 0 ? 'text-start' : 'text-end'">{{ c }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(r, ri) in s.rows" :key="ri" class="border-t border-panel-100" :class="ri === s.hot ? 'bg-coral-500/5' : ''">
            <td v-for="(cell, ci) in r" :key="ci" class="py-2" :class="ci === 0 ? 'text-start text-ink' : 'num text-end text-ink-soft'">{{ cell }}</td>
          </tr>
        </tbody>
      </table>
      <div class="mt-3 flex items-center justify-between rounded-lg bg-ink px-3 py-2.5 text-white">
        <span class="text-micro text-white/75">{{ s.footer[0] }}</span>
        <span class="num font-display text-base">{{ s.footer[1] }}</span>
      </div>
    </div>
  </LWindowMock>
</template>
