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
import { CalendarDays, Search, Star, Trash2 } from '@lucide/vue';
import AppButton from '../components/ui/AppButton.vue';
import AppCard from '../components/ui/AppCard.vue';
import AppInput from '../components/ui/AppInput.vue';
import AppTextarea from '../components/ui/AppTextarea.vue';
import AppSelect from '../components/ui/AppSelect.vue';
import AppSwitch from '../components/ui/AppSwitch.vue';
import AppCombobox, { type ComboOption } from '../components/ui/AppCombobox.vue';
import AppPhoneInput from '../components/ui/AppPhoneInput.vue';
import AppModal from '../components/ui/AppModal.vue';
import AppDatePicker from '../components/ui/AppDatePicker.vue';
import DirIcon from '../components/ui/DirIcon.vue';
import { dirIcon } from '../helpers/dirIcon';
import SegmentedControl from '../components/ui/SegmentedControl.vue';
import StatusBadge from '../components/ui/StatusBadge.vue';
import SkeletonBlock from '../components/ui/SkeletonBlock.vue';
import EmptyState from '../components/ui/EmptyState.vue';
import ErrorState from '../components/ui/ErrorState.vue';
import SearchInput from '../components/ui/SearchInput.vue';
import DateRangeFilter from '../components/ui/DateRangeFilter.vue';
import DataTable, { type Column } from '../components/ui/DataTable.vue';
import ScrollFade from '../components/ui/ScrollFade.vue';
import PageHeader from '../components/ui/PageHeader.vue';
import FormField from '../components/blocks/FormField.vue';
import FormSection from '../components/blocks/FormSection.vue';
import FormActions from '../components/blocks/FormActions.vue';
import FilterBar from '../components/blocks/FilterBar.vue';
import LineItemsEditor, { type LineColumn } from '../components/blocks/LineItemsEditor.vue';
import AddressFields from '../components/blocks/AddressFields.vue';
import TotalsPanel from '../components/blocks/TotalsPanel.vue';
import { formatAddress } from '../helpers/format';
import type { Address } from '../types/address';
import DetailHeader from '../components/blocks/DetailHeader.vue';
import StatCards from '../components/blocks/StatCards.vue';
import ListPage from '../components/layouts/ListPage.vue';
import FormPage from '../components/layouts/FormPage.vue';
import DetailPage from '../components/layouts/DetailPage.vue';
import SettingsPage from '../components/layouts/SettingsPage.vue';
import { Switch } from '../components/shadcn/switch';
import { ToggleGroup, ToggleGroupItem } from '../components/shadcn/toggle-group';
import { Breadcrumb, BreadcrumbItem, BreadcrumbLink, BreadcrumbList, BreadcrumbPage, BreadcrumbSeparator } from '../components/shadcn/breadcrumb';
import { Calendar } from '../components/shadcn/calendar';
import { RangeCalendar } from '../components/shadcn/range-calendar';
import { InputGroup, InputGroupAddon, InputGroupInput } from '../components/shadcn/input-group';
import { NativeSelect, NativeSelectOption } from '../components/shadcn/native-select';
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from '../components/shadcn/sheet';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from '../components/shadcn/dialog';

const inputValue = ref('');
const textareaValue = ref('');
const selectValue = ref<string>('');
const switchOn = ref(true);
const switchOff = ref(false);
const segmentValue = ref('a');
const comboValue = ref<string | undefined>();
const phoneEmpty = ref<string | undefined>();
const phoneEg = ref<string | undefined>('+201012345678');
const phoneSa = ref<string | undefined>('+966501234567');
const phoneInvalid = ref<string | undefined>('+20123');
const addressEg = ref<Address>({ country: 'EG' });
const addressSa = ref<Address>({ country: 'SA' });
const searchValue = ref('');
const dateFrom = ref('');
const dateTo = ref('');
const modalOpen = ref(false);
const pickerValue = ref<string | undefined>('2026-09-26');
const pickerEmpty = ref<string | undefined>();
const pickerError = ref<string | undefined>('2026-01-15');
const pickerCompact = ref<string | undefined>();
const pickerDialogOpen = ref(false);
const pickerInDialog = ref<string | undefined>();

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

// Phase F-0 demo state (docs/v2/17-ui-system-rtl-themes.md → Phase F "One shared UI system").
const ffNameField = ref('');
const ffEmailField = ref('');
const ffDirty = ref(true);

interface DemoLine { id: string; item: string; qty: number; price: number }
const ffLines = ref<DemoLine[]>([
  { id: '1', item: 'قلم رصاص', qty: 2, price: 3.5 },
  { id: '2', item: 'دفتر ملاحظات', qty: 1, price: 12 },
]);
const ffLineColumns: LineColumn<DemoLine>[] = [
  { key: 'item', label: 'الصنف' },
  { key: 'qty', label: 'الكمية', type: 'number', align: 'end', width: '100px' },
  { key: 'price', label: 'السعر', type: 'number', align: 'end', width: '120px' },
];
function ffNewLine(): DemoLine {
  return { id: String(Date.now()), item: '', qty: 1, price: 0 };
}

interface DemoStatusRow { id: string; name: string; amount: number; status: string; date: string }
const ffStatusColumns: Column<DemoStatusRow>[] = [
  { key: 'name', label: 'العميل', type: 'party' },
  { key: 'date', label: 'التاريخ', type: 'date' },
  { key: 'status', label: 'الحالة', type: 'status', statusOf: (v) => (v === 'paid' ? { label: 'مدفوعة', tone: 'success' } : { label: 'متأخرة', tone: 'danger' }) },
  { key: 'amount', label: 'المبلغ', type: 'money', align: 'end', totals: true },
];
const ffStatusRows: DemoStatusRow[] = [
  { id: '1', name: 'متجر الأمل', amount: 1250.5, status: 'paid', date: '2026-08-01' },
  { id: '2', name: 'مؤسسة النور', amount: 430, status: 'overdue', date: '2026-08-10' },
];
const ffSelected = ref<(string | number)[]>([]);

// RTL section demo state (docs/v2/17-ui-system-rtl-themes.md Phase A gate).
const rtlSwitchOn = ref(true);
const rtlSwitchOff = ref(false);
const rtlToggleValue = ref('b');
const rtlSheetLeftOpen = ref(false);
const rtlSheetRightOpen = ref(false);
const rtlDialogOpen = ref(false);
const rtlPage = ref(3);
const rtlPageCount = 8;
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

    <AppCard title="التاريخ (AppDatePicker)">
      <div class="grid gap-4 sm:grid-cols-2">
        <AppDatePicker v-model="pickerValue" label="تاريخ" hint="يمكن الكتابة مباشرة أو فتح التقويم" />
        <AppDatePicker v-model="pickerEmpty" label="بدون قيمة" placeholder="YYYY-MM-DD" />
        <AppDatePicker v-model="pickerError" label="تاريخ بخطأ" error="التاريخ غير صالح" />
        <AppDatePicker v-model="pickerValue" label="تاريخ معطّل" disabled />
        <AppDatePicker v-model="pickerCompact" label="حجم مضغوط (لخلايا الجدول)" compact />
        <div>
          <p class="mb-2 field-label">داخل نافذة منبثقة (فحص تراكب popover)</p>
          <Dialog v-model:open="pickerDialogOpen">
            <DialogTrigger as-child><AppButton>فتح نافذة تحتوي حقل تاريخ</AppButton></DialogTrigger>
            <DialogContent>
              <DialogHeader><DialogTitle>حقل تاريخ داخل نافذة</DialogTitle></DialogHeader>
              <AppDatePicker v-model="pickerInDialog" label="تاريخ" />
            </DialogContent>
          </Dialog>
        </div>
      </div>
    </AppCard>

    <AppCard title="الهاتف (AppPhoneInput)">
      <div class="grid gap-4 sm:grid-cols-2">
        <AppPhoneInput v-model="phoneEmpty" label="فارغ" default-country="EG" hint="اكتب رقماً محلياً بصفر البداية، مثل 01012345678" />
        <AppPhoneInput v-model="phoneEg" label="مصر (صحيح)" default-country="EG" />
        <AppPhoneInput v-model="phoneSa" label="السعودية (صحيح)" default-country="SA" kind="mobile" />
        <AppPhoneInput v-model="phoneInvalid" label="غير صحيح (بعد الخروج من الحقل)" default-country="EG" />
      </div>
    </AppCard>

    <AppCard title="العنوان (AddressFields)">
      <div class="grid gap-6 sm:grid-cols-2">
        <div>
          <p class="mb-2 text-xs text-text-secondary">مصر — جرّب "القاهرة" ثم اختر مدينة بلوحة المفاتيح</p>
          <AddressFields v-model="addressEg" country="EG" />
        </div>
        <div>
          <p class="mb-2 text-xs text-text-secondary">السعودية — "منطقة الرياض" ← "الرياض" ← حي</p>
          <AddressFields v-model="addressSa" country="SA" />
        </div>
      </div>
      <p class="mt-4 border-t border-border pt-3 text-xs text-text-secondary">
        مصر: <span class="text-text-primary">{{ formatAddress(addressEg) || '—' }}</span><br />
        السعودية: <span class="text-text-primary">{{ formatAddress(addressSa) || '—' }}</span>
      </p>
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

    <AppCard title="شريط قابل للتمرير (ScrollFade)">
      <div class="max-w-sm space-y-2">
        <p class="text-xs text-text-secondary">اسحب بالماوس أو مرّر بالعجلة — التدرّج يظهر فقط على الجهة التي بها محتوى إضافي.</p>
        <ScrollFade class="rounded-md border border-border">
          <div class="flex gap-1 p-1">
            <button v-for="n in 10" :key="n" type="button" class="shrink-0 rounded-md px-3 py-1.5 text-body text-text-secondary hover:bg-surface-hover hover:text-text-primary">
              عنصر {{ n }}
            </button>
          </div>
        </ScrollFade>
      </div>
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

    <AppCard title="RTL — الاتجاه الحقيقي (docs/v2/17 Phase A)">
      <div class="space-y-6">
        <div>
          <p class="mb-2 text-xs text-text-secondary">مفتاح التبديل (Switch) — القرص ينزلق لليمين عند التفعيل في RTL</p>
          <div class="flex flex-wrap items-center gap-6">
            <label class="flex items-center gap-2 text-body"><Switch v-model="rtlSwitchOn" /> مفعّل</label>
            <label class="flex items-center gap-2 text-body"><Switch v-model="rtlSwitchOff" /> غير مفعّل</label>
          </div>
        </div>

        <div>
          <p class="mb-2 text-xs text-text-secondary">مجموعة أزرار (ToggleGroup) — الأطراف المستديرة على الجانب الصحيح</p>
          <ToggleGroup v-model="rtlToggleValue" type="single" variant="outline" :spacing="0">
            <ToggleGroupItem value="a">يومي</ToggleGroupItem>
            <ToggleGroupItem value="b">أسبوعي</ToggleGroupItem>
            <ToggleGroupItem value="c">شهري</ToggleGroupItem>
          </ToggleGroup>
        </div>

        <div>
          <p class="mb-2 text-xs text-text-secondary">مسار التنقّل (Breadcrumb) — السهم يشير للخلف</p>
          <Breadcrumb>
            <BreadcrumbList>
              <BreadcrumbItem><BreadcrumbLink href="#">الرئيسية</BreadcrumbLink></BreadcrumbItem>
              <BreadcrumbSeparator />
              <BreadcrumbItem><BreadcrumbLink href="#">المبيعات</BreadcrumbLink></BreadcrumbItem>
              <BreadcrumbSeparator />
              <BreadcrumbItem><BreadcrumbPage>فاتورة جديدة</BreadcrumbPage></BreadcrumbItem>
            </BreadcrumbList>
          </Breadcrumb>
        </div>

        <div class="grid gap-6 sm:grid-cols-2">
          <div>
            <p class="mb-2 text-xs text-text-secondary">تقويم (Calendar) — أزرار السابق/التالي وقوائم الشهر/السنة</p>
            <div class="w-fit rounded-md border border-border">
              <Calendar layout="month-and-year" />
            </div>
          </div>
          <div>
            <p class="mb-2 text-xs text-text-secondary">تقويم مدى (RangeCalendar) — بداية/نهاية التحديد</p>
            <div class="w-fit rounded-md border border-border">
              <RangeCalendar />
            </div>
          </div>
        </div>

        <div>
          <p class="mb-2 text-xs text-text-secondary">مجموعة إدخال (InputGroup) — إضافات البداية/النهاية</p>
          <div class="flex max-w-xs flex-col gap-3">
            <InputGroup>
              <InputGroupAddon><Search class="size-4" /></InputGroupAddon>
              <InputGroupInput placeholder="إضافة في البداية" />
            </InputGroup>
            <InputGroup>
              <InputGroupInput placeholder="إضافة في النهاية" />
              <InputGroupAddon align="inline-end"><CalendarDays class="size-4" /></InputGroupAddon>
            </InputGroup>
          </div>
        </div>

        <div>
          <p class="mb-2 text-xs text-text-secondary">قائمة منسدلة أصلية (NativeSelect) — السهم في النهاية</p>
          <NativeSelect class="max-w-xs">
            <NativeSelectOption value="a">فرع الرياض</NativeSelectOption>
            <NativeSelectOption value="b">فرع جدة</NativeSelectOption>
          </NativeSelect>
        </div>

        <div class="flex flex-wrap items-center gap-3">
          <Sheet v-model:open="rtlSheetRightOpen">
            <SheetTrigger as-child><AppButton>لوحة من اليمين (الافتراضي)</AppButton></SheetTrigger>
            <SheetContent side="right">
              <SheetHeader><SheetTitle>لوحة جانبية</SheetTitle></SheetHeader>
            </SheetContent>
          </Sheet>
          <Sheet v-model:open="rtlSheetLeftOpen">
            <SheetTrigger as-child><AppButton>لوحة من اليسار</AppButton></SheetTrigger>
            <SheetContent side="left">
              <SheetHeader><SheetTitle>لوحة جانبية</SheetTitle></SheetHeader>
            </SheetContent>
          </Sheet>
          <Dialog v-model:open="rtlDialogOpen">
            <DialogTrigger as-child><AppButton>نافذة (زر الإغلاق أعلى اليسار)</AppButton></DialogTrigger>
            <DialogContent>
              <DialogHeader><DialogTitle>نافذة تجريبية</DialogTitle></DialogHeader>
            </DialogContent>
          </Dialog>
        </div>

        <div>
          <p class="mb-2 text-xs text-text-secondary">ترقيم الصفحات — السابق يشير لليمين، التالي لليسار</p>
          <div class="flex items-center gap-2">
            <button type="button" class="rounded-md border border-border p-1.5 hover:bg-surface-hover disabled:opacity-40" :disabled="rtlPage <= 1" @click="rtlPage--">
              <DirIcon :icon="dirIcon.prev" class="size-4" />
            </button>
            <span class="num min-w-12 text-center text-body">{{ rtlPage }} / {{ rtlPageCount }}</span>
            <button type="button" class="rounded-md border border-border p-1.5 hover:bg-surface-hover disabled:opacity-40" :disabled="rtlPage >= rtlPageCount" @click="rtlPage++">
              <DirIcon :icon="dirIcon.next" class="size-4" />
            </button>
          </div>
        </div>

        <div>
          <p class="mb-2 text-xs text-text-secondary">زر رجوع — يشير لليمين في RTL</p>
          <AppButton size="sm" variant="ghost" :icon="dirIcon.back" icon-rtl-flip>رجوع</AppButton>
        </div>
      </div>
    </AppCard>

    <AppCard title="نظام الصفحات المشترك — Phase F-0 (docs/v2/17 Phase F)">
      <p class="mb-4 text-body text-text-secondary">
        الكتل (blocks) وتخطيطات الصفحات (layouts) الجديدة — لم تُربط بعد بأي صفحة حقيقية (F-1..F-6 لاحقاً)،
        وهي هنا للمراجعة فقط. تتبع نفس الوضع الداكن/RTL التلقائي مثل بقية المعرض.
      </p>

      <div class="space-y-6">
        <div>
          <p class="mb-2 text-xs text-text-secondary">FormField + FormSection (شبكة عمودين)</p>
          <FormSection title="بيانات التواصل" description="تُستخدم في نماذج العملاء والموردين" :columns="2">
            <FormField label="الاسم" required>
              <AppInput v-model="ffNameField" placeholder="الاسم الكامل" />
            </FormField>
            <FormField label="البريد الإلكتروني" hint="اختياري">
              <AppInput v-model="ffEmailField" type="email" ltr />
            </FormField>
          </FormSection>
        </div>

        <div>
          <p class="mb-2 text-xs text-text-secondary">FormSection قابلة للطي</p>
          <FormSection title="خيارات متقدمة" description="مطوية افتراضياً" collapsible :default-open="false">
            <FormField label="ملاحظات داخلية"><AppInput placeholder="نص…" /></FormField>
          </FormSection>
        </div>

        <div>
          <p class="mb-2 text-xs text-text-secondary">FormActions — شريط حفظ ثابت مع تنبيه تغييرات غير محفوظة</p>
          <div class="rounded-xl border border-border p-4">
            <FormActions :dirty="ffDirty">
              <template #secondary><AppButton @click="ffDirty = !ffDirty">إلغاء</AppButton></template>
              <template #primary><AppButton variant="primary">حفظ (Ctrl+S)</AppButton></template>
            </FormActions>
          </div>
        </div>

        <div>
          <p class="mb-2 text-xs text-text-secondary">FilterBar — بحث + فلتر + مسح، متزامن مع رابط الصفحة</p>
          <FilterBar
            search-placeholder="بحث في العملاء…"
            :filters="[{ key: 'status', label: 'الحالة', options: [{ value: 'active', label: 'نشط' }, { value: 'inactive', label: 'غير نشط' }] }]"
            date-range
          />
        </div>

        <div>
          <p class="mb-2 text-xs text-text-secondary">DataTable — أعمدة نوعية (money / date / status) + إجمالي + تحديد صفوف</p>
          <DataTable v-model:selected="ffSelected" :columns="ffStatusColumns" :rows="ffStatusRows" selectable clickable />
        </div>

        <div>
          <p class="mb-2 text-xs text-text-secondary">LineItemsEditor — لا يحسب أي مجموع، يعرض ويُصدر الأحداث فقط</p>
          <LineItemsEditor :lines="ffLines" :columns="ffLineColumns" :new-line="ffNewLine" @lines-change="(l) => (ffLines = l)" />
        </div>

        <div class="grid gap-4 sm:grid-cols-2">
          <div>
            <p class="mb-2 text-xs text-text-secondary">TotalsPanel — مع سطر تفقيط</p>
            <TotalsPanel
              :rows="[
                { label: 'الإجمالي قبل الضريبة', amount: 400 },
                { label: 'الخصم', amount: 20, negative: true },
                { label: 'ضريبة القيمة المضافة', amount: 57 },
                { label: 'الإجمالي', amount: 437, emphasis: true },
              ]"
              show-tafqit
            />
          </div>
          <div>
            <p class="mb-2 text-xs text-text-secondary">StatCards — مع اتجاه (لا يُعكس في RTL)</p>
            <StatCards
              :cards="[
                { label: 'المبيعات اليوم', value: '4,320', trend: '+12%' },
                { label: 'الفواتير المتأخرة', value: '3', trend: '-5%' },
              ]"
            />
          </div>
        </div>

        <div>
          <p class="mb-2 text-xs text-text-secondary">DetailHeader — رقم المستند + الحالة + شرائح البيانات</p>
          <DetailHeader
            title="فاتورة مبيعات"
            number="INV-2026-0042"
            :status="{ label: 'مدفوعة', tone: 'success' }"
            :chips="[{ label: 'العميل', value: 'متجر الأمل' }, { label: 'التاريخ', value: '2026-09-26' }]"
          />
        </div>
      </div>
    </AppCard>

    <AppCard title="تخطيطات الصفحات — ListPage / FormPage / DetailPage / SettingsPage (Phase F-0)">
      <div class="space-y-6">
        <div class="rounded-xl border border-dashed border-border p-3">
          <p class="mb-2 text-xs text-text-secondary">ListPage (مصغّر داخل بطاقة للمعاينة فقط)</p>
          <ListPage title="العملاء" subtitle="١٢٣ سجلاً" primary-action-label="عميل جديد">
            <template #filters>
              <FilterBar search-placeholder="بحث…" />
            </template>
            <DataTable :columns="tableColumns" :rows="tableRows" />
          </ListPage>
        </div>

        <div class="rounded-xl border border-dashed border-border p-3">
          <p class="mb-2 text-xs text-text-secondary">FormPage (نموذج + شريط أعمال جانبي)</p>
          <FormPage title="عميل جديد">
            <FormSection title="البيانات الأساسية" :columns="2">
              <FormField label="الاسم" required><AppInput placeholder="اسم العميل" /></FormField>
              <FormField label="الهاتف"><AppInput ltr placeholder="+9665…" /></FormField>
            </FormSection>
            <template #aside>
              <TotalsPanel :rows="[{ label: 'الرصيد الحالي', amount: 0 }]" />
            </template>
            <template #actions>
              <FormActions>
                <template #primary><AppButton variant="primary">حفظ</AppButton></template>
              </FormActions>
            </template>
          </FormPage>
        </div>

        <div class="rounded-xl border border-dashed border-border p-3">
          <p class="mb-2 text-xs text-text-secondary">DetailPage (تبويبات + StatCards)</p>
          <DetailPage
            title="فاتورة مبيعات"
            number="INV-2026-0042"
            :status="{ label: 'مدفوعة', tone: 'success' }"
            :stats="[{ label: 'الإجمالي', value: '437' }, { label: 'المدفوع', value: '437' }]"
            :tabs="[{ key: 'details', label: 'التفاصيل' }, { key: 'history', label: 'السجل' }]"
          >
            <template #tab-details><p class="text-body text-text-secondary">محتوى التفاصيل هنا.</p></template>
            <template #tab-history><p class="text-body text-text-secondary">محتوى السجل هنا.</p></template>
          </DetailPage>
        </div>

        <div class="rounded-xl border border-dashed border-border p-3">
          <p class="mb-2 text-xs text-text-secondary">SettingsPage (رأس + شريحة تنقّل مُمرَّرة + محتوى)</p>
          <SettingsPage title="الإعدادات" subtitle="عام">
            <template #nav>
              <nav class="mb-4 flex gap-1 border-b border-border text-xs text-text-secondary">
                <span class="-mb-px border-b-2 border-primary px-3 py-2 font-medium text-text-primary">عام</span>
                <span class="px-3 py-2">الضرائب</span>
              </nav>
            </template>
            <p class="text-body text-text-secondary">محتوى قسم الإعدادات هنا.</p>
          </SettingsPage>
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
