<script setup lang="ts">
import { reactive, ref, watch } from 'vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { validate } from '@/modules/core/helpers/validation';
import { saveCustomer, saveSupplier } from '../services/partyService';
import type { Customer, Supplier } from '../types';
import { partySchema } from '../validators/partySchema';

const props = defineProps<{ kind: 'customer' | 'supplier'; party?: Customer | Supplier | null }>();
const open = defineModel<boolean>('open', { default: false });
const emit = defineEmits<{ saved: [party: Customer | Supplier] }>();
const toast = useToast();

const form = reactive({
  name: '',
  phone: '',
  address: '',
  vatNumber: '',
  contactPerson: '',
  type: 'individual' as 'individual' | 'company',
  active: true,
});
const errors = ref<Record<string, string>>({});
const saving = ref(false);

watch(open, (isOpen) => {
  if (!isOpen) return;
  const p = props.party;
  Object.assign(form, {
    name: p?.name ?? '',
    phone: p?.phone ?? '',
    address: p?.address ?? '',
    vatNumber: p?.vatNumber ?? '',
    contactPerson: (p as Supplier | undefined)?.contactPerson ?? '',
    type: (p as Customer | undefined)?.type ?? 'individual',
    active: p?.active ?? true,
  });
  errors.value = {};
});

async function save() {
  errors.value = validate(partySchema, form);
  if (Object.keys(errors.value).length) return;
  saving.value = true;
  try {
    const saved =
      props.kind === 'customer'
        ? await saveCustomer({ name: form.name, phone: form.phone, address: form.address, vatNumber: form.vatNumber, type: form.type, active: form.active }, props.party?.id)
        : await saveSupplier(
            { name: form.name, phone: form.phone, address: form.address, vatNumber: form.vatNumber, contactPerson: form.contactPerson, active: form.active },
            props.party?.id,
          );
    toast.success(props.party ? 'تم حفظ التعديلات' : props.kind === 'customer' ? 'تمت إضافة العميل' : 'تمت إضافة المورد', saved.name);
    open.value = false;
    emit('saved', saved);
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <AppModal
    v-model:open="open"
    :title="party ? `تعديل ${kind === 'customer' ? 'العميل' : 'المورد'}` : kind === 'customer' ? 'عميل جديد' : 'مورد جديد'"
    :persistent="saving"
  >
    <form id="party-form" class="grid gap-4 sm:grid-cols-2" novalidate @submit.prevent="save">
      <div v-if="kind === 'customer'" class="sm:col-span-2">
        <span class="field-label">نوع العميل</span>
        <SegmentedControl
          v-model="form.type"
          :options="[
            { value: 'individual', label: 'فرد' },
            { value: 'company', label: 'منشأة' },
          ]"
        />
      </div>
      <AppInput v-model="form.name" class="sm:col-span-2" :label="kind === 'customer' && form.type === 'company' ? 'اسم المنشأة' : 'الاسم'" required :error="errors.name" />
      <AppInput v-model="form.phone" label="الهاتف" type="tel" ltr placeholder="05xxxxxxxx" :error="errors.phone" />
      <AppInput v-if="kind === 'supplier'" v-model="form.contactPerson" label="الشخص المسؤول" />
      <AppInput
        v-model="form.vatNumber"
        label="الرقم الضريبي"
        ltr
        placeholder="3xxxxxxxxxxxxx3"
        :error="errors.vatNumber"
        :hint="kind === 'customer' ? 'مطلوب للفواتير الضريبية للمنشآت' : undefined"
      />
      <AppInput v-model="form.address" class="sm:col-span-2" label="العنوان" />
      <AppSwitch v-if="party" v-model="form.active" class="sm:col-span-2" label="نشط" description="غير النشط لا يظهر في قوائم الاختيار" />
    </form>
    <template #footer>
      <AppButton @click="open = false">إلغاء</AppButton>
      <AppButton type="submit" form="party-form" variant="primary" :loading="saving">حفظ</AppButton>
    </template>
  </AppModal>
</template>
