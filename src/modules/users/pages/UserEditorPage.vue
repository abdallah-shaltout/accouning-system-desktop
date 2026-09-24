<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Check, Eye, Minus, Pencil, Save } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppPhoneInput from '@/modules/core/components/ui/AppPhoneInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { errorMessage } from '@/modules/core/controllers/useToast';
import { ROLE_LABEL } from '@/modules/core/helpers/labels';
import { validate } from '@/modules/core/helpers/validation';
import { getPriceLists } from '@/modules/products/services/catalogService';
import type { PriceList } from '@/modules/products/types';
import { useAuthStore } from '../controllers/useAuthStore';
import { ROLE_ACCESS } from '../helpers/permissions';
import { createUser, getUser, updateUser } from '../services/userService';
import type { Area, Role } from '../types';
import { userSchema } from '../validators/userSchema';

const route = useRoute();
const router = useRouter();
const toast = useToast();
const auth = useAuthStore();

const id = computed(() => (route.params.id === 'new' || !route.params.id ? undefined : String(route.params.id)));
const isSelf = computed(() => id.value === auth.user?.id);

const form = reactive({
  name: '',
  username: '',
  phone: '',
  role: 'cashier' as Role,
  maxDiscount: 5 as number | undefined,
  priceListId: '' as string,
  active: true,
  password: '',
});
const errors = ref<Record<string, string>>({});
const priceLists = ref<PriceList[]>([]);
const loading = ref(true);
const loadError = ref<string | null>(null);
const saving = ref(false);

async function load() {
  loading.value = true;
  loadError.value = null;
  try {
    priceLists.value = await getPriceLists();
    if (id.value) {
      const user = await getUser(id.value);
      Object.assign(form, {
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
const priceListOptions = computed(() => priceLists.value.filter((p) => p.active || p.id === form.priceListId).map((p) => ({ value: p.id, label: p.name })));

const AREA_LABEL: Record<Area, string> = {
  dashboard: 'الرئيسية',
  pos: 'نقطة البيع',
  sales: 'الفواتير والمرتجعات',
  inventory: 'المنتجات والمخزون',
  parties: 'العملاء والموردين',
  purchases: 'المشتريات',
  expenses: 'المصروفات',
  accounting: 'الحسابات والقيود',
  payments: 'سندات القبض والصرف',
  reports: 'التقارير',
  analytics: 'التحليلات',
  users: 'إدارة المستخدمين',
  settings: 'الإعدادات',
};
const accessRows = computed(() => (Object.keys(AREA_LABEL) as Area[]).map((a) => ({ area: a, label: AREA_LABEL[a], access: ROLE_ACCESS[form.role][a] })));

async function save() {
  errors.value = validate(userSchema(!id.value), form);
  if (Object.keys(errors.value).length) return;
  saving.value = true;
  try {
    const input = {
      name: form.name.trim(),
      username: form.username.trim(),
      phone: form.phone.trim() || undefined,
      role: form.role,
      maxDiscount: form.maxDiscount ?? 0,
      priceListId: form.priceListId || undefined,
      active: form.active,
      password: form.password || undefined,
    };
    const user = id.value ? await updateUser(id.value, input) : await createUser(input);
    auth.patchCurrent(user);
    toast.success(id.value ? 'تم حفظ التعديلات' : 'تمت إضافة المستخدم', user.name);
    router.push('/users');
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div>
    <PageHeader :title="id ? 'تعديل مستخدم' : 'مستخدم جديد'" :subtitle="id ? form.name : 'أضف حساباً لموظف وحدد صلاحياته'" back="/users" />

    <ErrorState v-if="loadError" :message="loadError" @retry="load" />
    <div v-else-if="loading" class="grid gap-5 lg:grid-cols-[1fr_320px]">
      <AppCard><SkeletonBlock :lines="8" height="h-8" /></AppCard>
      <AppCard><SkeletonBlock :lines="6" /></AppCard>
    </div>

    <form v-else class="grid items-start gap-5 lg:grid-cols-[1fr_320px]" novalidate @submit.prevent="save">
      <div class="space-y-5">
        <AppCard title="البيانات الأساسية">
          <div class="grid gap-4 sm:grid-cols-2">
            <AppInput v-model="form.name" label="الاسم الكامل" required :error="errors.name" />
            <AppPhoneInput v-model="form.phone" label="الجوال" :error="errors.phone" />
            <AppInput v-model="form.username" label="اسم المستخدم" required ltr :error="errors.username" hint="يستخدم لتسجيل الدخول" />
            <AppInput
              v-model="form.password"
              :label="id ? 'كلمة مرور جديدة' : 'كلمة المرور'"
              type="password"
              ltr
              :required="!id"
              :error="errors.password"
              :hint="id ? 'اتركها فارغة للإبقاء على كلمة المرور الحالية' : undefined"
            />
          </div>
        </AppCard>

        <AppCard title="الصلاحيات ونقطة البيع">
          <div class="grid gap-4 sm:grid-cols-3">
            <AppSelect v-model="form.role" label="الصلاحية" :options="roleOptions" :disabled="isSelf" :hint="isSelf ? 'لا يمكنك تغيير صلاحيتك' : undefined" />
            <AppInput v-model="form.maxDiscount" label="أقصى نسبة خصم" type="number" min="0" max="100" :error="errors.maxDiscount" hint="يُطبق في نقطة البيع">
              <template #suffix>%</template>
            </AppInput>
            <AppSelect v-model="form.priceListId" label="قائمة الأسعار" :options="priceListOptions" placeholder="السعر الأساسي" hint="الأسعار التي يبيع بها" />
          </div>
          <div class="mt-5 border-t border-border pt-4">
            <AppSwitch v-model="form.active" label="الحساب نشط" description="الحساب الموقوف لا يمكنه تسجيل الدخول" :disabled="isSelf" />
          </div>
        </AppCard>

        <div class="flex justify-end gap-2">
          <AppButton to="/users">إلغاء</AppButton>
          <AppButton type="submit" variant="primary" :icon="Save" :loading="saving">حفظ</AppButton>
        </div>
      </div>

      <AppCard :title="`صلاحيات ${ROLE_LABEL[form.role]}`" padding="none">
        <ul class="divide-y divide-border">
          <li v-for="r in accessRows" :key="r.area" class="flex items-center justify-between px-4 py-2 text-body">
            <span :class="r.access === 'none' && 'text-text-secondary'">{{ r.label }}</span>
            <span
              class="inline-flex items-center gap-1 text-xs"
              :class="r.access === 'write' ? 'text-success' : r.access === 'read' ? 'text-text-primary' : 'text-text-secondary'"
            >
              <Pencil v-if="r.access === 'write'" class="size-3" />
              <Eye v-else-if="r.access === 'read'" class="size-3" />
              <Minus v-else class="size-3" />
              {{ r.access === 'write' ? 'كامل' : r.access === 'read' ? 'عرض فقط' : 'لا يوجد' }}
            </span>
          </li>
        </ul>
        <p class="flex items-start gap-1.5 border-t border-border px-4 py-3 text-xs text-text-secondary">
          <Check class="mt-0.5 size-3.5 shrink-0" /> الصلاحيات قوالب ثابتة حسب الدور — لا حاجة لضبط كل صلاحية على حدة.
        </p>
      </AppCard>
    </form>
  </div>
</template>
