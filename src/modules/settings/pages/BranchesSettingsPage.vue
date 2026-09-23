<script setup lang="ts">
/**
 * v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md §1): Settings → Branches. Each branch
 * auto-creates its own cash-drawer account + cost center on save (see
 * src/mocks/backend/branches.ts's `createBranch`) — this page never manages those directly.
 */
import { computed, onMounted, reactive, ref } from 'vue';
import { Pencil, Plus, Power } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import AppTextarea from '@/modules/core/components/ui/AppTextarea.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import SettingsTabs from '../components/SettingsTabs.vue';
import { createBranch, deactivateBranch, getBranches, reactivateBranch, updateBranch } from '../services/branchesService';
import type { Branch } from '../types';

const auth = useAuthStore();
const toast = useToast();
const confirm = useConfirm();
const canWrite = computed(() => auth.can('settings', 'write'));

const loading = ref(true);
const branches = ref<Branch[]>([]);

async function reload() {
  branches.value = await getBranches();
}

onMounted(async () => {
  await reload();
  loading.value = false;
});

const formOpen = ref(false);
const editing = ref<Branch | null>(null);
const saving = ref(false);
const errors = ref<Record<string, string>>({});
const form = reactive({ name: '', code: '', address: '', phone: '', receiptHeader: '', active: true });

function openCreate() {
  editing.value = null;
  Object.assign(form, { name: '', code: '', address: '', phone: '', receiptHeader: '', active: true });
  errors.value = {};
  formOpen.value = true;
}

function openEdit(branch: Branch) {
  editing.value = branch;
  Object.assign(form, { name: branch.name, code: branch.code, address: branch.address ?? '', phone: branch.phone ?? '', receiptHeader: branch.receiptHeader ?? '', active: branch.active });
  errors.value = {};
  formOpen.value = true;
}

async function save() {
  errors.value = {};
  if (!form.name.trim()) errors.value.name = 'اسم الفرع مطلوب';
  if (!form.code.trim()) errors.value.code = 'رمز الفرع مطلوب (يُستخدم في ترقيم المستندات، مثال: RYD)';
  if (Object.keys(errors.value).length) return;
  saving.value = true;
  try {
    const payload = { name: form.name.trim(), code: form.code.trim(), address: form.address.trim() || undefined, phone: form.phone.trim() || undefined, receiptHeader: form.receiptHeader.trim() || undefined, active: form.active };
    if (editing.value) await updateBranch(editing.value.id, payload);
    else await createBranch(payload);
    await reload();
    toast.success(editing.value ? 'تم تحديث الفرع' : 'تمت إضافة الفرع — تم إنشاء حساب صندوق ومركز تكلفة خاص به تلقائياً');
    formOpen.value = false;
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

async function toggle(branch: Branch) {
  const ok = branch.active
    ? await confirm({ title: `إلغاء تفعيل "${branch.name}"؟`, message: 'يتطلب صفر مخزون وعدم وجود ورديات مفتوحة على هذا الفرع.', danger: true, confirmText: 'إلغاء التفعيل' })
    : true;
  if (!ok) return;
  try {
    if (branch.active) await deactivateBranch(branch.id);
    else await reactivateBranch(branch.id);
    await reload();
  } catch (err) {
    toast.error(err);
  }
}
</script>

<template>
  <div>
    <PageHeader title="الفروع" subtitle="كل فرع يحصل تلقائياً على حساب صندوق ومركز تكلفة خاص به" />
    <SettingsTabs />

    <SkeletonBlock v-if="loading" :lines="4" height="h-12" />
    <AppCard v-else padding="none">
      <template #actions>
        <AppButton v-if="canWrite" size="sm" :icon="Plus" @click="openCreate">إضافة فرع</AppButton>
      </template>
      <EmptyState v-if="!branches.length" title="لا توجد فروع" />
      <table v-else class="w-full text-body">
        <thead class="text-xs text-text-secondary">
          <tr class="border-b border-border">
            <th class="px-4 py-2 text-start font-medium">الفرع</th>
            <th class="px-2 py-2 text-start font-medium">الرمز</th>
            <th class="px-2 py-2 text-start font-medium">العنوان</th>
            <th class="px-2 py-2 text-start font-medium">نشط</th>
            <th class="px-4 py-2"></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="b in branches" :key="b.id" class="border-b border-border last:border-0">
            <td class="px-4 py-2 font-medium">{{ b.name }}</td>
            <td class="px-2 py-2"><span class="num text-text-secondary">{{ b.code }}</span></td>
            <td class="px-2 py-2 text-text-secondary">{{ b.address || '—' }}</td>
            <td class="px-2 py-2"><AppSwitch :model-value="b.active" :disabled="!canWrite" @update:model-value="() => toggle(b)" /></td>
            <td class="px-4 py-2 text-end">
              <button type="button" class="rounded-md p-1.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary" :disabled="!canWrite" @click="openEdit(b)">
                <Pencil class="size-4" />
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </AppCard>

    <AppModal v-model:open="formOpen" :title="editing ? 'تعديل فرع' : 'فرع جديد'" :persistent="saving">
      <form class="space-y-4" novalidate @submit.prevent="save">
        <AppInput v-model="form.name" label="اسم الفرع" required :error="errors.name" />
        <AppInput v-model="form.code" label="الرمز (لترقيم المستندات)" required ltr :error="errors.code" hint="مثال: RYD ← RYD-INV-00042" />
        <AppInput v-model="form.address" label="العنوان" />
        <AppInput v-model="form.phone" label="الهاتف" ltr />
        <AppTextarea v-model="form.receiptHeader" label="نص أعلى الإيصال" :rows="2" />
        <AppSwitch v-model="form.active" label="نشط" />
      </form>
      <template #footer>
        <AppButton :disabled="saving" @click="formOpen = false">إلغاء</AppButton>
        <AppButton variant="primary" :icon="Power" :loading="saving" @click="save">حفظ</AppButton>
      </template>
    </AppModal>
  </div>
</template>
