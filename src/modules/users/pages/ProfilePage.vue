<script setup lang="ts">
/**
 * Self-service "ملفي الشخصي" — name, phone and password, reachable by any signed-in user
 * regardless of their `users` area permission (route has no `area` meta, same pattern as
 * settings-appearance). Unlike UserEditorPage.vue (admin tool, requires users:write), this never
 * shows role/active/maxDiscount/price-list — updateUser() still needs the full UserInput, so those
 * fields are carried through unchanged from the loaded user.
 */
import { onMounted, reactive, ref } from 'vue';
import { Save } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppPasswordInput from '@/modules/core/components/ui/AppPasswordInput.vue';
import AppPhoneInput from '@/modules/core/components/ui/AppPhoneInput.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import FormActions from '@/modules/core/components/blocks/FormActions.vue';
import FormSection from '@/modules/core/components/blocks/FormSection.vue';
import FormPage from '@/modules/core/components/layouts/FormPage.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { errorMessage } from '@/modules/core/controllers/useToast';
import { ROLE_LABEL } from '@/modules/core/helpers/labels';
import { validate } from '@/modules/core/helpers/validation';
import { useAuthStore } from '../controllers/useAuthStore';
import { getUser, updateUser } from '../services/userService';
import type { User } from '../types';
import { profileSchema } from '../validators/userSchema';

const auth = useAuthStore();
const toast = useToast();

const source = ref<User | null>(null);
const form = reactive({ name: '', phone: '', password: '' });
const errors = ref<Record<string, string>>({});
const loading = ref(true);
const loadError = ref<string | null>(null);
const saving = ref(false);

async function load() {
  loading.value = true;
  loadError.value = null;
  try {
    const user = await getUser(auth.user!.id);
    source.value = user;
    Object.assign(form, { name: user.name, phone: user.phone ?? '', password: '' });
  } catch (err) {
    loadError.value = errorMessage(err);
  } finally {
    loading.value = false;
  }
}
onMounted(load);

async function save() {
  errors.value = validate(profileSchema, form);
  if (Object.keys(errors.value).length) return;
  saving.value = true;
  try {
    const current = source.value!;
    const user = await updateUser(current.id, {
      name: form.name.trim(),
      username: current.username,
      phone: form.phone.trim() || undefined,
      role: current.role,
      maxDiscount: current.maxDiscount,
      priceListId: current.priceListId,
      active: current.active,
      password: form.password || undefined,
    });
    auth.patchCurrent(user);
    source.value = user;
    form.password = '';
    toast.success('تم حفظ التعديلات');
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <FormPage title="ملفي الشخصي" :subtitle="source?.name" :back="{ name: 'home' }">
    <ErrorState v-if="loadError" :message="loadError" @retry="load" />
    <AppCard v-else-if="loading"><SkeletonBlock :lines="6" height="h-8" /></AppCard>

    <form v-else novalidate @submit.prevent="save">
      <FormSection title="البيانات الأساسية" :columns="2">
        <AppInput v-model="form.name" label="الاسم الكامل" required :error="errors.name" />
        <AppPhoneInput v-model="form.phone" label="الجوال" kind="mobile" :error="errors.phone" />
        <AppInput :model-value="source?.username" label="اسم المستخدم" ltr disabled hint="لا يمكن تغيير اسم المستخدم" />
        <AppInput :model-value="source ? ROLE_LABEL[source.role] : ''" label="الصلاحية" disabled />
      </FormSection>

      <FormSection title="كلمة المرور" :columns="2">
        <AppPasswordInput
          v-model="form.password"
          label="كلمة مرور جديدة"
          autocomplete="new-password"
          :error="errors.password"
          hint="اتركها فارغة للإبقاء على كلمة المرور الحالية"
        />
      </FormSection>

      <FormActions>
        <template #primary>
          <AppButton type="submit" variant="primary" :icon="Save" :loading="saving">حفظ</AppButton>
        </template>
      </FormActions>
    </form>
  </FormPage>
</template>
