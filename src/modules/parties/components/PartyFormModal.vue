<script setup lang="ts">
/**
 * Quick-add modal (docs/v2/08-customers-and-suppliers.md §1: "a modal for quick-add (name + phone +
 * type only)"). The full sectioned form (basics/contact/national-address/tax/terms/bank/accounts/
 * opening-balance/notes) lives at `/customers/new` and `/suppliers/new` (PartyFormPage.vue) — this
 * modal links there for anyone who needs more than the three fields.
 */
import { reactive, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import { ExternalLink, TriangleAlert } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppPhoneInput from '@/modules/core/components/ui/AppPhoneInput.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { validate } from '@/modules/core/helpers/validation';
import { checkDuplicates, saveCustomer, saveSupplier, type DuplicateWarning } from '../services/partyService';
import type { Customer, Supplier } from '../types';
import { partyQuickSchema } from '../validators/partySchema';

const props = defineProps<{ kind: 'customer' | 'supplier'; party?: Customer | Supplier | null }>();
const open = defineModel<boolean>('open', { default: false });
const emit = defineEmits<{ saved: [party: Customer | Supplier] }>();
const toast = useToast();
const router = useRouter();

const form = reactive({
  name: '',
  phone: '' as string | undefined,
  vatNumber: '',
  type: 'individual' as 'individual' | 'company',
  active: true,
});
const errors = ref<Record<string, string>>({});
const saving = ref(false);
const duplicates = ref<DuplicateWarning[]>([]);

watch(open, (isOpen) => {
  if (!isOpen) return;
  const p = props.party;
  Object.assign(form, {
    name: p?.name ?? '',
    phone: p?.phone ?? undefined,
    vatNumber: p?.vatNumber ?? '',
    type: (p as Customer | undefined)?.type ?? 'individual',
    active: p?.active ?? true,
  });
  errors.value = {};
  duplicates.value = [];
});

let dupTimer: ReturnType<typeof setTimeout> | undefined;
watch([() => form.phone, () => form.vatNumber], () => {
  clearTimeout(dupTimer);
  dupTimer = setTimeout(async () => {
    if (!form.phone && !form.vatNumber) {
      duplicates.value = [];
      return;
    }
    duplicates.value = await checkDuplicates({ phone: form.phone || undefined, vatNumber: form.vatNumber || undefined }, props.party?.id);
  }, 300);
});

function duplicateLink(d: DuplicateWarning) {
  // Guess the kind from the id prefix (cus-/sup-) since findDuplicates searches both lists.
  return d.existingId.startsWith('sup') ? `/suppliers/${d.existingId}` : `/customers/${d.existingId}`;
}

async function save() {
  errors.value = validate(partyQuickSchema, form);
  if (Object.keys(errors.value).length) return;
  saving.value = true;
  try {
    const saved =
      props.kind === 'customer'
        ? await saveCustomer({ name: form.name, phone: form.phone, vatNumber: form.vatNumber || undefined, type: form.type, active: form.active }, props.party?.id)
        : await saveSupplier({ name: form.name, phone: form.phone, vatNumber: form.vatNumber || undefined, type: form.type, active: form.active }, props.party?.id);
    toast.success(props.party ? 'تم حفظ التعديلات' : props.kind === 'customer' ? 'تمت إضافة العميل' : 'تمت إضافة المورد', saved.name);
    open.value = false;
    emit('saved', saved);
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

function openFullForm() {
  open.value = false;
  router.push(props.kind === 'customer' ? '/customers/new' : '/suppliers/new');
}
</script>

<template>
  <AppModal
    v-model:open="open"
    :title="party ? `تعديل ${kind === 'customer' ? 'العميل' : 'المورد'}` : kind === 'customer' ? 'عميل جديد (إضافة سريعة)' : 'مورد جديد (إضافة سريعة)'"
    :persistent="saving"
  >
    <form id="party-quick-form" class="space-y-4" novalidate @submit.prevent="save">
      <div>
        <span class="field-label">النوع</span>
        <SegmentedControl
          v-model="form.type"
          :options="[
            { value: 'individual', label: 'فرد' },
            { value: 'company', label: 'منشأة' },
          ]"
        />
      </div>
      <AppInput v-model="form.name" :label="form.type === 'company' ? 'اسم المنشأة' : 'الاسم'" required :error="errors.name" />
      <AppPhoneInput v-model="form.phone" label="الهاتف" :error="errors.phone" />
      <AppInput v-model="form.vatNumber" label="الرقم الضريبي (اختياري)" ltr placeholder="3xxxxxxxxxxxxx3" :error="errors.vatNumber" />

      <div v-if="duplicates.length" class="flex items-start gap-2 rounded-lg border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning">
        <TriangleAlert class="mt-0.5 size-3.5 shrink-0" />
        <ul class="space-y-1">
          <li v-for="d in duplicates" :key="`${d.field}-${d.existingId}`">
            {{ d.field === 'phone' ? 'رقم الهاتف' : 'الرقم الضريبي' }} مستخدم بالفعل لدى
            <RouterLink :to="duplicateLink(d)" class="font-medium underline" target="_blank">{{ d.existingName }}</RouterLink>
          </li>
        </ul>
      </div>

      <button type="button" class="flex items-center gap-1.5 text-xs text-primary hover:underline" @click="openFullForm">
        <ExternalLink class="size-3.5" /> فتح النموذج الكامل (العنوان الوطني، البنك، شروط التعامل…)
      </button>
    </form>
    <template #footer>
      <AppButton @click="open = false">إلغاء</AppButton>
      <AppButton type="submit" form="party-quick-form" variant="primary" :loading="saving">حفظ</AppButton>
    </template>
  </AppModal>
</template>
