<script setup lang="ts">
/**
 * Dev-only component gallery (docs/v2/16-equal-rebrand-and-ui-kit.md Phase C) — every rebuilt App*
 * component in its default/disabled/loading/error states, for reviewing the shadcn-vue rebuild
 * before/after each batch landed. Light/dark and LTR/RTL aren't duplicated here: toggle them with
 * the app's own theme switch and OS setting — every component below already responds live to both,
 * since that's the whole point of building on shadcn-vue + reka-ui's ConfigProvider.
 *
 * Not registered as a real nav item; reachable only by typing the URL, and only in dev
 * (see routes/index.ts's `import.meta.env.DEV` guard).
 */
import { ref } from 'vue';
import { Star, Trash2 } from '@lucide/vue';
import AppButton from '../components/ui/AppButton.vue';
import AppCard from '../components/ui/AppCard.vue';
import AppInput from '../components/ui/AppInput.vue';
import AppTextarea from '../components/ui/AppTextarea.vue';
import AppSelect from '../components/ui/AppSelect.vue';
import AppSwitch from '../components/ui/AppSwitch.vue';
import AppCombobox, { type ComboOption } from '../components/ui/AppCombobox.vue';
import AppModal from '../components/ui/AppModal.vue';
import SegmentedControl from '../components/ui/SegmentedControl.vue';
import StatusBadge from '../components/ui/StatusBadge.vue';
import SkeletonBlock from '../components/ui/SkeletonBlock.vue';
import EmptyState from '../components/ui/EmptyState.vue';
import ErrorState from '../components/ui/ErrorState.vue';
import SearchInput from '../components/ui/SearchInput.vue';
import DateRangeFilter from '../components/ui/DateRangeFilter.vue';
import DataTable, { type Column } from '../components/ui/DataTable.vue';
import PageHeader from '../components/ui/PageHeader.vue';

const inputValue = ref('');
const textareaValue = ref('');
const selectValue = ref<string>('');
const switchOn = ref(true);
const switchOff = ref(false);
const segmentValue = ref('a');
const comboValue = ref<string | undefined>();
const searchValue = ref('');
const dateFrom = ref('');
const dateTo = ref('');
const modalOpen = ref(false);

const comboOptions: ComboOption[] = [
  { value: '1', label: 'حساب الصندوق', sublabel: '1110 — الأصول المتداولة' },
  { value: '2', label: 'حساب البنك', sublabel: '1120 — الأصول المتداولة' },
  { value: '3', label: 'حساب العملاء', sublabel: '1200 — الأصول المتداولة' },
];

interface DemoRow {
  id: string;
  name: string;
  amount: number;
}
const tableColumns: Column<DemoRow>[] = [
  { key: 'name', label: 'الاسم', sortable: true },
  { key: 'amount', label: 'المبلغ', numeric: true, align: 'end', sortable: true },
];
const tableRows: DemoRow[] = [
  { id: '1', name: 'صف تجريبي أول', amount: 1250.5 },
  { id: '2', name: 'صف تجريبي ثاني', amount: 89.9 },
  { id: '3', name: 'صف تجريبي ثالث', amount: 4200 },
];
</script>

<template>
  <div class="max-w-5xl space-y-8">
    <PageHeader title="معرض المكوّنات (Dev)" subtitle="مراجعة كل مكوّن بعد إعادة بنائه على shadcn-vue — الوضع الداكن/RTL يتبعان إعدادات التطبيق." />

    <AppCard title="الأزرار (AppButton)">
      <div class="flex flex-wrap items-center gap-2">
        <AppButton variant="primary">أساسي</AppButton>
        <AppButton variant="secondary">ثانوي</AppButton>
        <AppButton variant="ghost">شبح</AppButton>
        <AppButton variant="danger">خطر (حدود)</AppButton>
        <AppButton variant="danger-solid">خطر (معبأ)</AppButton>
        <AppButton variant="primary" :icon="Star">مع أيقونة</AppButton>
        <AppButton variant="primary" loading>جارٍ التحميل</AppButton>
        <AppButton variant="primary" disabled>معطّل</AppButton>
        <AppButton variant="primary" kbd="Ctrl+S">مع اختصار</AppButton>
      </div>
    </AppCard>

    <AppCard title="الشارات (StatusBadge)">
      <div class="flex flex-wrap items-center gap-2">
        <StatusBadge tone="neutral" label="محايد" />
        <StatusBadge tone="success" label="مدفوعة" />
        <StatusBadge tone="warning" label="جزئي" />
        <StatusBadge tone="danger" label="متأخر" />
        <StatusBadge tone="primary" label="مسودة" />
      </div>
    </AppCard>

    <AppCard title="الحقول (AppInput / AppTextarea / AppSelect / AppSwitch)">
      <div class="grid gap-4 sm:grid-cols-2">
        <AppInput v-model="inputValue" label="حقل نصي" placeholder="اكتب هنا…" />
        <AppInput label="حقل بخطأ" model-value="" error="هذا الحقل مطلوب" />
        <AppInput label="حقل معطّل" model-value="قيمة ثابتة" disabled />
        <AppSelect
          v-model="selectValue"
          label="قائمة منسدلة"
          placeholder="اختر…"
          :options="[
            { value: 'a', label: 'خيار أول' },
            { value: 'b', label: 'خيار ثانٍ' },
            { value: 'c', label: 'خيار ثالث (معطّل)', disabled: true },
          ]"
        />
        <AppTextarea v-model="textareaValue" label="ملاحظات" placeholder="نص متعدد الأسطر…" />
        <AppCombobox v-model="comboValue" label="بحث واختيار (Combobox)" :options="comboOptions" />
      </div>
      <div class="mt-4 flex flex-wrap gap-6">
        <AppSwitch v-model="switchOn" label="مفعّل" description="حالة تشغيل" />
        <AppSwitch v-model="switchOff" label="غير مفعّل" description="حالة إيقاف" />
        <AppSwitch v-model="switchOff" label="معطّل" disabled />
      </div>
    </AppCard>

    <AppCard title="البحث والفلاتر (SearchInput / DateRangeFilter / SegmentedControl)">
      <div class="space-y-4">
        <SearchInput v-model="searchValue" placeholder="بحث…" kbd="Ctrl K" />
        <DateRangeFilter v-model:from="dateFrom" v-model:to="dateTo" />
        <SegmentedControl
          v-model="segmentValue"
          :options="[
            { value: 'a', label: 'اليوم' },
            { value: 'b', label: 'هذا الأسبوع', count: 7 },
            { value: 'c', label: 'هذا الشهر' },
          ]"
        />
      </div>
    </AppCard>

    <AppCard title="النوافذ المنبثقة (AppModal)">
      <AppButton variant="primary" @click="modalOpen = true">فتح نافذة تجريبية</AppButton>
      <AppModal v-model:open="modalOpen" title="نافذة تجريبية" description="لمراجعة التركيز والإغلاق بـ Escape">
        <p class="text-body text-text-secondary">محتوى النافذة هنا.</p>
        <template #footer>
          <AppButton @click="modalOpen = false">إلغاء</AppButton>
          <AppButton variant="primary" @click="modalOpen = false">تأكيد</AppButton>
        </template>
      </AppModal>
    </AppCard>

    <AppCard title="جدول البيانات (DataTable)">
      <DataTable :columns="tableColumns" :rows="tableRows" />
    </AppCard>

    <AppCard title="حالات التحميل والفراغ والخطأ (SkeletonBlock / EmptyState / ErrorState)">
      <div class="grid gap-6 sm:grid-cols-3">
        <div>
          <p class="mb-2 text-xs text-text-secondary">هيكل التحميل</p>
          <SkeletonBlock :lines="3" />
        </div>
        <div>
          <p class="mb-2 text-xs text-text-secondary">لا توجد بيانات</p>
          <EmptyState compact title="لا توجد نتائج" description="جرّب تعديل الفلاتر." />
        </div>
        <div>
          <p class="mb-2 text-xs text-text-secondary">تعذّر التحميل</p>
          <ErrorState compact message="حدث خطأ ما." @retry="() => {}" />
        </div>
      </div>
    </AppCard>

    <AppCard title="ملاحظة">
      <p class="text-body text-text-secondary">
        <Trash2 class="me-1 inline size-4 align-text-bottom text-text-secondary" />
        هذه الصفحة للمطورين فقط ولا تظهر في التنقّل — انظر
        <code class="rounded bg-surface-hover px-1 py-0.5 text-xs">docs/v2/16-equal-rebrand-and-ui-kit.md</code>
        Phase C.
      </p>
    </AppCard>
  </div>
</template>
