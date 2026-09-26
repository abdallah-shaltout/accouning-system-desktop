<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Save } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppPhoneInput from '@/modules/core/components/ui/AppPhoneInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import FormActions from '@/modules/core/components/blocks/FormActions.vue';
import FormField from '@/modules/core/components/blocks/FormField.vue';
import FormSection from '@/modules/core/components/blocks/FormSection.vue';
import FormPage from '@/modules/core/components/layouts/FormPage.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useForm } from '@/modules/core/controllers/useForm';
import { useToast } from '@/modules/core/controllers/useToast';
import { errorMessage } from '@/modules/core/controllers/useToast';
import { ROLE_LABEL } from '@/modules/core/helpers/labels';
import { getPriceLists } from '@/modules/products/services/catalogService';
import type { PriceList } from '@/modules/products/types';
import UserAccessPreview from '../components/UserAccessPreview.vue';
import { useAuthStore } from '../controllers/useAuthStore';
import { createUser, getUser, updateUser } from '../services/userService';
import type { Role } from '../types';
import { userSchema } from '../validators/userSchema';

const route = useRoute('user-editor');
const router = useRouter();
const toast = useToast();
const auth = useAuthStore();

const id = computed(() => (route.params.id === 'new' || !route.params.id ? undefined : String(route.params.id)));
const isSelf = computed(() => id.value === auth.user?.id);

const priceLists = ref<PriceList[]>([]);
const loading = ref(true);
const loadError = ref<string | null>(null);

const form = useForm({
  schema: () => userSchema(!id.value),
  initial: {
    name: '',
    username: '',
    phone: '',
    role: 'cashier' as Role,
    maxDiscount: 5 as number | undefined,
    priceListId: '' as string,
    active: true,
    password: '',
  },
  onSubmit: async (values) => {
    const input = {
      name: values.name.trim(),
      username: values.username.trim(),
      phone: values.phone.trim() || undefined,
      role: values.role,
      maxDiscount: values.maxDiscount ?? 0,
      priceListId: values.priceListId || undefined,
      active: values.active,
      password: values.password || undefined,
    };
    const user = id.value ? await updateUser(id.value, input) : await createUser(input);
    auth.patchCurrent(user);
    toast.success(id.value ? 'تم حفظ التعديلات' : 'تمت إضافة المستخدم', user.name);
    router.push('/users');
  },
});

async function load() {
  loading.value = true;
  loadError.value = null;
  try {
    priceLists.value = await getPriceLists();
    if (id.value) {
      const user = await getUser(id.value);
      form.reset({
        name: user.name,
        username: user.username,
        phone: user.phone ?? '',
        role: user.role,
        maxDiscount: user.maxDiscount,
        priceListId: user.priceListId ?? '',
        active: user.active,
        password: '',
      });
    }
  } catch (err) {
    loadError.value = errorMessage(err);
  } finally {
    loading.value = false;
  }
}
onMounted(load);

const roleOptions = (Object.keys(ROLE_LABEL) as Role[]).map((r) => ({ value: r, label: ROLE_LABEL[r] }));
const priceListOptions = computed(() => priceLists.value.filter((p) => p.active || p.id === form.values.priceListId).map((p) => ({ value: p.id, label: p.name })));

async function onSave() {
  const ok = await form.submit();
  if (!ok) return;
}
</script>

<template>
  <ErrorState v-if="loadError" :message="loadError" @retry="load" />
  <div v-else-if="loading" class="grid gap-5 lg:grid-cols-[1fr_320px]">
    <AppCard><SkeletonBlock :lines="8" height="h-8" /></AppCard>
    <AppCard><SkeletonBlock :lines="6" /></AppCard>
  </div>

  <FormPage v-else :title="id ? 'تعديل مستخدم' : 'مستخدم جديد'" :subtitle="id ? form.values.name : 'أضف حساباً لموظف وحدد صلاحياته'" :back="'/users' as any">
    <template #aside>
      <UserAccessPreview :role="form.values.role" />
    </template>

    <FormSection title="البيانات الأساسية" :columns="2">
      <FormField label="الاسم الكامل" required name="name" :error="form.errors.name">
        <AppInput v-model="form.values.name" />
      </FormField>
      <FormField label="الجوال" name="phone" :error="form.errors.phone">
        <AppPhoneInput v-model="form.values.phone" kind="mobile" />
      </FormField>
      <FormField label="اسم المستخدم" required name="username" hint="يستخدم لتسجيل الدخول" :error="form.errors.username">
        <AppInput v-model="form.values.username" ltr />
      </FormField>
      <FormField
        :label="id ? 'كلمة مرور جديدة' : 'كلمة المرور'"
        name="password"
        :required="!id"
        :hint="id ? 'اتركها فارغة للإبقاء على كلمة المرور الحالية' : undefined"
        :error="form.errors.password"
      >
        <AppInput v-model="form.values.password" type="password" ltr />
      </FormField>
    </FormSection>

    <FormSection title="الصلاحيات ونقطة البيع" :columns="3">
      <FormField label="الصلاحية" name="role" :hint="isSelf ? 'لا يمكنك تغيير صلاحيتك' : undefined">
        <AppSelect v-model="form.values.role" :options="roleOptions" :disabled="isSelf" />
      </FormField>
      <FormField label="أقصى نسبة خصم" name="maxDiscount" hint="يُطبق في نقطة البيع" :error="form.errors.maxDiscount">
        <AppInput v-model="form.values.maxDiscount" type="number" min="0" max="100">
          <template #suffix>%</template>
        </AppInput>
      </FormField>
      <FormField label="قائمة الأسعار" name="priceListId" hint="الأسعار التي يبيع بها">
        <AppSelect v-model="form.values.priceListId" :options="priceListOptions" placeholder="السعر الأساسي" />
      </FormField>
      <div class="sm:col-span-3 border-t border-border pt-4">
        <AppSwitch v-model="form.values.active" label="الحساب نشط" description="الحساب الموقوف لا يمكنه تسجيل الدخول" :disabled="isSelf" />
      </div>
    </FormSection>

    <template #actions>
      <FormActions :dirty="form.dirty">
        <template #secondary><AppButton to="/users">إلغاء</AppButton></template>
        <template #primary><AppButton variant="primary" :icon="Save" :loading="form.submitting" @click="onSave">حفظ</AppButton></template>
      </FormActions>
    </template>
  </FormPage>
</template>
