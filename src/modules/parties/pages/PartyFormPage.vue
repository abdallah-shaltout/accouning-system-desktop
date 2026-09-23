<script setup lang="ts">
/**
 * Full sectioned party form (docs/v2/08-customers-and-suppliers.md §1): basics / contact /
 * national address / tax & registration / terms / bank / accounts (advanced) / opening balance
 * (stub — Phase 5 posts it) / notes & attachments. Used for both `/customers/new|:id` and
 * `/suppliers/new|:id` (route meta decides `kind`).
 */
import { computed, onMounted, reactive, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Plus, Save, Trash2, TriangleAlert } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppPhoneInput from '@/modules/core/components/ui/AppPhoneInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import AppTextarea from '@/modules/core/components/ui/AppTextarea.vue';
import AttachmentField from '@/modules/core/components/ui/AttachmentField.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { errorMessage } from '@/modules/core/controllers/useToast';
import { COUNTRIES } from '@/modules/core/helpers/countries';
import { PHONE_LABEL } from '@/modules/core/helpers/labels';
import { uid } from '@/mocks';
import {
  checkDuplicates,
  getCustomer,
  getCustomers,
  getPartyGroups,
  getSupplier,
  getSuppliers,
  linkPartyRecords,
  saveCustomer,
  saveSupplier,
  unlinkPartyRecord,
  type DuplicateWarning,
} from '../services/partyService';
import type { Customer, NationalAddress, PartyContact, PartyGroup, PartyPhone, Supplier } from '../types';
import { isValidIban } from '../validators/partySchema';

const props = defineProps<{ kind: 'customer' | 'supplier' }>();
const route = useRoute();
const router = useRouter();
const toast = useToast();

const id = computed(() => (route.params.id && route.params.id !== 'new' ? String(route.params.id) : undefined));
const isCustomer = computed(() => props.kind === 'customer');

interface FormState {
  type: 'individual' | 'company';
  name: string;
  nameEn: string;
  groupId: string;
  active: boolean;
  phones: PartyPhone[];
  email: string;
  contacts: PartyContact[];
  nationalAddress: NationalAddress;
  vatNumber: string;
  crNumber: string;
  nationalId: string;
  currency: string;
  paymentTermsDays: number | undefined;
  creditLimit: number | undefined;
  bankName: string;
  iban: string;
  accountName: string;
  contactPerson: string;
  notes: string;
  openingAmount: number | undefined;
  openingSide: 'debit' | 'credit';
  openingAsOf: string;
}

function emptyForm(): FormState {
  return {
    type: 'individual',
    name: '',
    nameEn: '',
    groupId: '',
    active: true,
    phones: [{ id: uid('phone'), label: 'mobile', number: '' }],
    email: '',
    contacts: [],
    nationalAddress: {},
    vatNumber: '',
    crNumber: '',
    nationalId: '',
    currency: 'SAR',
    paymentTermsDays: undefined,
    creditLimit: undefined,
    bankName: '',
    iban: '',
    accountName: '',
    contactPerson: '',
    notes: '',
    openingAmount: undefined,
    openingSide: 'debit',
    openingAsOf: '',
  };
}

const form = reactive<FormState>(emptyForm());
const loading = ref(true);
const loadError = ref<string | null>(null);
const saving = ref(false);
const groups = ref<PartyGroup[]>([]);
const duplicates = ref<DuplicateWarning[]>([]);
const linkedId = ref<string | undefined>();
const linkCandidates = ref<(Customer | Supplier)[]>([]);
const showLinkPicker = ref(false);

async function load() {
  loading.value = true;
  loadError.value = null;
  try {
    groups.value = await getPartyGroups(props.kind);
    if (id.value) {
      const p = isCustomer.value ? await getCustomer(id.value) : await getSupplier(id.value);
      Object.assign(form, {
        type: p.type,
        name: p.name,
        nameEn: p.nameEn ?? '',
        groupId: p.groupId ?? '',
        active: p.active,
        phones: p.phones?.length ? p.phones : p.phone ? [{ id: uid('phone'), label: 'mobile', number: p.phone }] : [{ id: uid('phone'), label: 'mobile', number: '' }],
        email: p.email ?? '',
        contacts: p.contacts ?? [],
        nationalAddress: { ...(p.nationalAddress ?? {}) },
        vatNumber: p.vatNumber ?? '',
        crNumber: p.crNumber ?? '',
        nationalId: p.nationalId ?? '',
        currency: p.currency ?? 'SAR',
        paymentTermsDays: p.paymentTermsDays,
        creditLimit: (p as Customer).creditLimit,
        bankName: p.bank?.bankName ?? '',
        iban: p.bank?.iban ?? '',
        accountName: p.bank?.accountName ?? '',
        contactPerson: (p as Supplier).contactPerson ?? '',
        notes: p.notes ?? '',
      });
      linkedId.value = p.linkedPartyId;
    }
  } catch (err) {
    loadError.value = errorMessage(err);
  } finally {
    loading.value = false;
  }
}
onMounted(load);

function addPhone() {
  form.phones.push({ id: uid('phone'), label: form.phones.length ? 'work' : 'mobile', number: '' });
}
function removePhone(phoneId: string) {
  form.phones = form.phones.filter((p) => p.id !== phoneId);
}

function addContact() {
  form.contacts.push({ id: uid('contact'), name: '' });
}
function removeContact(contactId: string) {
  form.contacts = form.contacts.filter((c) => c.id !== contactId);
}

const primaryPhone = computed(() => form.phones[0]?.number);

let dupTimer: ReturnType<typeof setTimeout> | undefined;
watch([primaryPhone, () => form.vatNumber], () => {
  clearTimeout(dupTimer);
  dupTimer = setTimeout(async () => {
    if (!primaryPhone.value && !form.vatNumber) {
      duplicates.value = [];
      return;
    }
    duplicates.value = await checkDuplicates({ phone: primaryPhone.value, vatNumber: form.vatNumber || undefined }, id.value);
  }, 300);
});

function duplicateLink(d: DuplicateWarning) {
  return d.existingId.startsWith('sup') ? `/suppliers/${d.existingId}` : `/customers/${d.existingId}`;
}

const ibanError = computed(() => (form.iban && !isValidIban(form.iban) ? 'رقم آيبان غير صحيح' : undefined));
const nameError = computed(() => (submitted.value && !form.name.trim() ? 'الاسم مطلوب' : undefined));
const vatError = computed(() => (form.vatNumber && !/^3\d{13}3$/.test(form.vatNumber) ? '15 رقماً يبدأ وينتهي بالرقم 3' : undefined));
const submitted = ref(false);

const nationalAddressPreview = computed(() => {
  const a = form.nationalAddress;
  const parts = [a.street, a.district, a.city, a.buildingNo && `مبنى ${a.buildingNo}`, a.postalCode].filter(Boolean);
  return parts.length ? parts.join('، ') : undefined;
});

async function save() {
  submitted.value = true;
  if (nameError.value || vatError.value || ibanError.value) {
    toast.error('راجع الحقول المطلوبة');
    return;
  }
  saving.value = true;
  try {
    const common = {
      type: form.type,
      name: form.name.trim(),
      nameEn: form.nameEn.trim() || undefined,
      groupId: form.groupId || undefined,
      active: form.active,
      phone: primaryPhone.value,
      phones: form.phones.filter((p) => p.number),
      email: form.email.trim() || undefined,
      contacts: form.contacts.filter((c) => c.name.trim()),
      nationalAddress: Object.values(form.nationalAddress).some(Boolean) ? form.nationalAddress : undefined,
      vatNumber: form.vatNumber.trim() || undefined,
      crNumber: form.crNumber.trim() || undefined,
      nationalId: form.nationalId.trim() || undefined,
      currency: form.currency || undefined,
      paymentTermsDays: form.paymentTermsDays,
      bank: form.bankName || form.iban || form.accountName ? { bankName: form.bankName || undefined, iban: form.iban || undefined, accountName: form.accountName || undefined } : undefined,
      notes: form.notes.trim() || undefined,
      // Opening-balance stub (docs/v2/08 §1 "رصيد سابق"): captured now, posted by the Phase 5
      // opening-balance wizard. TODO(phase 5): post an opening journal entry from this on first save.
      openingBalance:
        form.openingAmount !== undefined ? { amount: form.openingAmount, side: form.openingSide, asOfDate: form.openingAsOf || undefined } : undefined,
    };
    // Reactive form state holds nested arrays-of-objects (phones/contacts/nationalAddress) as Vue
    // Proxies; the mock backend later `structuredClone()`s the whole DB for IndexedDB persistence
    // (src/mocks/persist.ts), which chokes on a stray Proxy. Deep-clone to a plain object before
    // sending, same pattern already used by modules/templates/pages/TemplateDesignerPage.vue.
    const payload = JSON.parse(
      JSON.stringify(isCustomer.value ? { ...common, creditLimit: form.creditLimit } : { ...common, contactPerson: form.contactPerson.trim() || undefined }),
    );
    const saved = isCustomer.value ? await saveCustomer(payload, id.value) : await saveSupplier(payload, id.value);
    toast.success(id.value ? 'تم حفظ التعديلات' : isCustomer.value ? 'تمت إضافة العميل' : 'تمت إضافة المورد', saved.name);
    router.push(isCustomer.value ? `/customers/${saved.id}` : `/suppliers/${saved.id}`);
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

// --- "Both roles" linking (docs/v2/08 §1) --------------------------------------------------------

async function openLinkPicker() {
  linkCandidates.value = isCustomer.value ? await getSuppliers({ includeInactive: true }) : await getCustomers({ includeInactive: true });
  showLinkPicker.value = true;
}
async function link(otherId: string) {
  if (!id.value) return;
  await linkPartyRecords(isCustomer.value ? id.value : otherId, isCustomer.value ? otherId : id.value);
  linkedId.value = otherId;
  showLinkPicker.value = false;
  toast.success('تم الربط بسجل الطرف الآخر');
}
async function unlink() {
  if (!id.value) return;
  await unlinkPartyRecord(id.value, props.kind);
  linkedId.value = undefined;
  toast.success('تم إلغاء الربط');
}

const countryOptions = COUNTRIES.map((c) => ({ value: c.code, label: `${c.flag} ${c.nameAr}` }));
</script>

<template>
  <div>
    <PageHeader
      :title="id ? `تعديل ${isCustomer ? 'العميل' : 'المورد'}` : isCustomer ? 'عميل جديد' : 'مورد جديد'"
      :subtitle="id ? form.name : undefined"
      :back="id ? `/${isCustomer ? 'customers' : 'suppliers'}/${id}` : `/${isCustomer ? 'customers' : 'suppliers'}`"
    />

    <ErrorState v-if="loadError" :message="loadError" @retry="load" />
    <div v-else-if="loading" class="space-y-5">
      <AppCard><SkeletonBlock :lines="6" height="h-9" /></AppCard>
      <AppCard><SkeletonBlock :lines="4" height="h-9" /></AppCard>
    </div>

    <form v-else class="grid items-start gap-5 xl:grid-cols-[1fr_320px]" novalidate @submit.prevent="save">
      <div class="space-y-5">
        <!-- الأساسي -->
        <AppCard title="الأساسي">
          <div class="grid gap-4 sm:grid-cols-2">
            <div class="sm:col-span-2">
              <span class="field-label">النوع</span>
              <SegmentedControl v-model="form.type" :options="[{ value: 'individual', label: 'فرد' }, { value: 'company', label: 'منشأة' }]" />
            </div>
            <AppInput v-model="form.name" class="sm:col-span-2" :label="form.type === 'company' ? 'اسم المنشأة' : 'الاسم'" required :error="nameError" />
            <AppInput v-model="form.nameEn" label="الاسم (إنجليزي، اختياري)" ltr />
            <AppSelect v-model="form.groupId" label="المجموعة" placeholder="بدون مجموعة" :options="groups.map((g) => ({ value: g.id, label: g.name }))" />
            <AppSwitch v-if="id" v-model="form.active" class="sm:col-span-2" label="نشط" description="غير النشط لا يظهر في قوائم الاختيار" />
          </div>
        </AppCard>

        <!-- التواصل -->
        <AppCard title="التواصل">
          <div class="space-y-3">
            <div v-for="(phone, i) in form.phones" :key="phone.id" class="flex items-end gap-2">
              <AppSelect v-model="phone.label" class="w-28 shrink-0" :options="(['mobile', 'work', 'whatsapp'] as const).map((l) => ({ value: l, label: PHONE_LABEL[l] }))" />
              <div class="flex-1"><AppPhoneInput v-model="phone.number" :label="i === 0 ? 'الهاتف' : undefined" /></div>
              <button type="button" class="mb-2 shrink-0 rounded p-2 text-text-secondary hover:bg-surface-hover hover:text-danger" @click="removePhone(phone.id)">
                <Trash2 class="size-4" />
              </button>
            </div>
            <AppButton type="button" size="sm" :icon="Plus" @click="addPhone">إضافة رقم آخر</AppButton>

            <div v-if="duplicates.length" class="flex items-start gap-2 rounded-lg border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning">
              <TriangleAlert class="mt-0.5 size-3.5 shrink-0" />
              <ul class="space-y-1">
                <li v-for="d in duplicates" :key="`${d.field}-${d.existingId}`">
                  {{ d.field === 'phone' ? 'رقم الهاتف' : 'الرقم الضريبي' }} مستخدم بالفعل لدى
                  <RouterLink :to="duplicateLink(d)" class="font-medium underline" target="_blank">{{ d.existingName }}</RouterLink>
                </li>
              </ul>
            </div>

            <AppInput v-model="form.email" label="البريد الإلكتروني" type="email" ltr />

            <div class="border-t border-border pt-3">
              <span class="field-label">أشخاص التواصل</span>
              <div v-for="c in form.contacts" :key="c.id" class="mt-2 grid grid-cols-1 gap-2 sm:grid-cols-[1fr_1fr_1fr_auto]">
                <AppInput v-model="c.name" placeholder="الاسم" />
                <AppInput v-model="c.role" placeholder="الصفة" />
                <AppPhoneInput v-model="c.phone" />
                <button type="button" class="justify-self-end rounded p-2 text-text-secondary hover:bg-surface-hover hover:text-danger sm:justify-self-auto" @click="removeContact(c.id)">
                  <Trash2 class="size-4" />
                </button>
              </div>
              <AppButton type="button" class="mt-2" size="sm" :icon="Plus" @click="addContact">إضافة شخص تواصل</AppButton>
            </div>
          </div>
        </AppCard>

        <!-- العنوان الوطني -->
        <AppCard title="العنوان الوطني">
          <div class="grid gap-4 sm:grid-cols-3">
            <AppSelect v-model="form.nationalAddress.country" label="الدولة" placeholder="اختر…" :options="countryOptions" />
            <AppInput v-model="form.nationalAddress.city" label="المدينة" />
            <AppInput v-model="form.nationalAddress.district" label="الحي" />
            <AppInput v-model="form.nationalAddress.street" label="الشارع" />
            <AppInput v-model="form.nationalAddress.buildingNo" label="رقم المبنى" ltr />
            <AppInput v-model="form.nationalAddress.additionalNo" label="الرقم الإضافي" ltr />
            <AppInput v-model="form.nationalAddress.postalCode" label="الرمز البريدي" ltr />
            <AppInput v-model="form.nationalAddress.unitNo" label="رقم الوحدة" ltr />
            <AppInput v-model="form.nationalAddress.shortAddress" label="العنوان المختصر" ltr placeholder="RRRD2929" />
          </div>
          <p v-if="nationalAddressPreview" class="mt-3 border-t border-border pt-3 text-xs text-text-secondary">
            يظهر في الفاتورة هكذا: <span class="text-text-primary">{{ nationalAddressPreview }}</span>
          </p>
        </AppCard>

        <!-- الضريبة والسجل -->
        <AppCard title="الضريبة والسجل">
          <div class="grid gap-4 sm:grid-cols-3">
            <AppInput v-model="form.vatNumber" label="الرقم الضريبي" ltr placeholder="3xxxxxxxxxxxxx3" :error="vatError" hint="يفعّل الفواتير الضريبية B2B" />
            <AppInput v-model="form.crNumber" label="رقم السجل التجاري" ltr />
            <AppInput v-if="form.type === 'individual'" v-model="form.nationalId" label="رقم الهوية الوطنية" ltr />
          </div>
        </AppCard>

        <!-- التعامل -->
        <AppCard title="التعامل">
          <div class="grid gap-4 sm:grid-cols-3">
            <AppSelect v-model="form.currency" label="العملة" :options="[{ value: 'SAR', label: 'ريال سعودي (SAR)' }, { value: 'USD', label: 'دولار أمريكي (USD)' }, { value: 'EGP', label: 'جنيه مصري (EGP)' }, { value: 'AED', label: 'درهم إماراتي (AED)' }]" hint="تُقفل بعد أول مستند" />
            <AppInput v-model="form.paymentTermsDays" type="number" min="0" label="شروط السداد (أيام)" hint="يُحسب منها تاريخ استحقاق الفواتير الآجلة" />
            <AppInput v-if="isCustomer" v-model="form.creditLimit" type="number" min="0" label="الحد الائتماني" hint="0 = بدون حد" />
          </div>
        </AppCard>

        <!-- البنك -->
        <AppCard title="البنك">
          <div class="grid gap-4 sm:grid-cols-3">
            <AppInput v-model="form.bankName" label="اسم البنك" />
            <AppInput v-model="form.iban" label="الآيبان (IBAN)" ltr placeholder="SA0380000000608010167519" :error="ibanError" />
            <AppInput v-model="form.accountName" label="اسم صاحب الحساب" />
          </div>
        </AppCard>

        <!-- الحسابات (متقدم) -->
        <AppCard v-if="!isCustomer" title="الحسابات (متقدم)">
          <AppInput v-model="form.contactPerson" label="الشخص المسؤول" />
        </AppCard>

        <!-- رصيد سابق -->
        <AppCard title="رصيد سابق (من نظام قديم)">
          <p class="mb-3 text-xs text-text-secondary">يُحفظ الآن كملاحظة على البطاقة؛ ترحيله كقيد افتتاحي فعلي يتم من معالج الأرصدة الافتتاحية (المرحلة القادمة).</p>
          <div class="grid gap-4 sm:grid-cols-3">
            <AppInput v-model="form.openingAmount" type="number" min="0" label="المبلغ" />
            <div>
              <span class="field-label">الجهة</span>
              <SegmentedControl v-model="form.openingSide" :options="[{ value: 'debit', label: 'مدين (له/علينا)' }, { value: 'credit', label: 'دائن (عليه/لنا)' }]" size="sm" />
            </div>
            <AppInput v-model="form.openingAsOf" type="date" label="كما في تاريخ" />
          </div>
        </AppCard>

        <!-- ملاحظات ومرفقات -->
        <AppCard title="ملاحظات ومرفقات">
          <AppTextarea v-model="form.notes" label="ملاحظات" :rows="3" />
          <div v-if="id" class="mt-4 border-t border-border pt-4">
            <span class="field-label">المرفقات</span>
            <AttachmentField :owner-ref="`${kind}:${id}`" />
          </div>
          <p v-else class="mt-3 text-xs text-text-secondary">يمكن إضافة المرفقات بعد حفظ البطاقة أول مرة.</p>
        </AppCard>

        <div class="flex justify-end gap-2">
          <AppButton :to="id ? `/${isCustomer ? 'customers' : 'suppliers'}/${id}` : `/${isCustomer ? 'customers' : 'suppliers'}`">إلغاء</AppButton>
          <AppButton type="submit" variant="primary" :icon="Save" :loading="saving">حفظ</AppButton>
        </div>
      </div>

      <!-- كلا الدورين -->
      <AppCard title="عميل ومورد معاً" padding="sm">
        <p class="text-xs leading-5 text-text-secondary">
          إذا كان هذا الطرف عميلاً ومورداً في نفس الوقت (مثل تاجر جملة نبيع له ونشتري منه)، اربط بطاقته بسجل
          {{ isCustomer ? 'المورد' : 'العميل' }} المقابل لعرض رصيد صافٍ بينهما.
        </p>
        <template v-if="!id">
          <p class="mt-3 text-xs text-text-secondary">احفظ البطاقة أولاً لتتمكن من الربط.</p>
        </template>
        <template v-else-if="linkedId">
          <RouterLink :to="`/${isCustomer ? 'suppliers' : 'customers'}/${linkedId}`" class="mt-3 block text-sm text-primary hover:underline">
            عرض سجل {{ isCustomer ? 'المورد' : 'العميل' }} المرتبط
          </RouterLink>
          <AppButton type="button" size="sm" class="mt-2" @click="unlink">إلغاء الربط</AppButton>
        </template>
        <template v-else>
          <AppButton type="button" size="sm" class="mt-3" @click="openLinkPicker">ربط بسجل {{ isCustomer ? 'مورد' : 'عميل' }} موجود</AppButton>
          <div v-if="showLinkPicker" class="mt-2 max-h-56 space-y-1 overflow-y-auto rounded-lg border border-border p-1">
            <button
              v-for="c in linkCandidates"
              :key="c.id"
              type="button"
              class="block w-full rounded-md px-2 py-1.5 text-start text-xs hover:bg-surface-hover"
              @click="link(c.id)"
            >
              {{ c.name }}
            </button>
            <p v-if="!linkCandidates.length" class="px-2 py-3 text-center text-xs text-text-secondary">لا توجد سجلات</p>
          </div>
        </template>
      </AppCard>
    </form>
  </div>
</template>
