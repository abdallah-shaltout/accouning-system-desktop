<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
import { GripVertical, Pencil, Plus, Trash } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { useCatalogStore } from '@/modules/products/controllers/useCatalogStore';
import { applyUnitPreset, deleteCustomFieldDef, reorderCustomFieldDefs, saveCustomFieldDef } from '@/modules/products/services/catalogService';
import type { CustomFieldDef, CustomFieldType, UnitPresetKind } from '@/modules/products/types';
import SettingsTabs from '../components/SettingsTabs.vue';

/** v2 §7 (Settings → Products): custom fields shown on the product form's "إضافي" tab, plus a
 * quick way to apply a unit preset for the business type (§2). */
const catalog = useCatalogStore();
const auth = useAuthStore();
const toast = useToast();
const confirm = useConfirm();
const canWrite = computed(() => auth.can('inventory', 'write'));

const loading = ref(true);
onMounted(async () => {
  await catalog.load(true);
  loading.value = false;
});

const TYPE_LABEL: Record<CustomFieldType, string> = { text: 'نص', number: 'رقم', date: 'تاريخ', list: 'قائمة', yesno: 'نعم/لا' };
const TYPE_OPTIONS = (Object.keys(TYPE_LABEL) as CustomFieldType[]).map((t) => ({ value: t, label: TYPE_LABEL[t] }));

const modalOpen = ref(false);
const editing = ref<CustomFieldDef | null>(null);
const saving = ref(false);
const errors = ref<Record<string, string>>({});
const form = reactive({ name: '', type: 'text' as CustomFieldType, optionsText: '', active: true });

function openCreate() {
  editing.value = null;
  Object.assign(form, { name: '', type: 'text', optionsText: '', active: true });
  errors.value = {};
  modalOpen.value = true;
}
function openEdit(def: CustomFieldDef) {
  editing.value = def;
  Object.assign(form, { name: def.name, type: def.type, optionsText: (def.options ?? []).join('، '), active: def.active });
  errors.value = {};
  modalOpen.value = true;
}

async function save() {
  errors.value = {};
  if (!form.name.trim()) errors.value.name = 'اسم الحقل مطلوب';
  if (form.type === 'list' && !form.optionsText.trim()) errors.value.optionsText = 'أضف خياراً واحداً على الأقل';
  if (Object.keys(errors.value).length) return;
  saving.value = true;
  try {
    await saveCustomFieldDef(
      {
        name: form.name.trim(),
        type: form.type,
        options: form.type === 'list' ? form.optionsText.split(/[،,]/).map((o) => o.trim()).filter(Boolean) : undefined,
        active: form.active,
      },
      editing.value?.id,
    );
    await catalog.load(true);
    toast.success(editing.value ? 'تم حفظ الحقل' : 'تمت إضافة الحقل');
    modalOpen.value = false;
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

async function remove(def: CustomFieldDef) {
  const ok = await confirm({ title: `حذف حقل "${def.name}"؟`, confirmText: 'حذف', danger: true });
  if (!ok) return;
  try {
    await deleteCustomFieldDef(def.id);
    await catalog.load(true);
    toast.success('تم حذف الحقل');
  } catch (err) {
    toast.error(err);
  }
}

// --- drag-to-reorder ---
const dragIndex = ref<number | null>(null);
async function onDrop(index: number) {
  if (dragIndex.value === null || dragIndex.value === index) return;
  const next = [...catalog.customFieldDefs];
  const [moved] = next.splice(dragIndex.value, 1);
  next.splice(index, 0, moved);
  dragIndex.value = null;
  try {
    await reorderCustomFieldDefs(next.map((f) => f.id));
    await catalog.load(true);
  } catch (err) {
    toast.error(err);
  }
}

// --- unit presets ---
const PRESET_OPTIONS: { value: UnitPresetKind; label: string }[] = [
  { value: 'pharmacy', label: 'صيدلية (علبة، شريط، قرص، زجاجة، أمبول)' },
  { value: 'clothing', label: 'ملابس (قطعة، طقم، درزن)' },
  { value: 'supermarket', label: 'سوبرماركت (حبة، كرتون، كيلو، جرام، لتر)' },
];
const applyingPreset = ref(false);
async function applyPreset(kind: UnitPresetKind) {
  applyingPreset.value = true;
  try {
    const created = await applyUnitPreset(kind);
    await catalog.load(true);
    toast.success(created.length ? `تمت إضافة ${created.length} وحدة` : 'جميع وحدات هذا القالب موجودة بالفعل');
  } catch (err) {
    toast.error(err);
  } finally {
    applyingPreset.value = false;
  }
}
</script>

<template>
  <div>
    <PageHeader title="إعدادات المنتجات" subtitle="حقول مخصصة تظهر في تبويب «إضافي» ببطاقة المنتج، وقوالب وحدات جاهزة">
      <template v-if="canWrite" #actions>
        <AppButton variant="primary" :icon="Plus" @click="openCreate">حقل جديد</AppButton>
      </template>
    </PageHeader>
    <SettingsTabs />

    <div class="grid items-start gap-5 lg:grid-cols-[1fr_320px]">
      <AppCard title="الحقول المخصصة" padding="none">
        <SkeletonBlock v-if="loading" class="p-4" :lines="4" />
        <EmptyState v-else-if="!catalog.customFieldDefs.length" title="لا توجد حقول مخصصة بعد" compact />
        <ul v-else class="divide-y divide-border">
          <li
            v-for="(f, i) in catalog.customFieldDefs"
            :key="f.id"
            class="flex items-center gap-2 px-4 py-2.5"
            :draggable="canWrite"
            @dragstart="dragIndex = i"
            @dragover.prevent
            @drop="onDrop(i)"
          >
            <GripVertical v-if="canWrite" class="size-4 shrink-0 cursor-grab text-text-secondary" />
            <div class="min-w-0 flex-1">
              <span class="block truncate">{{ f.name }}</span>
              <span class="block text-tiny text-text-secondary">{{ TYPE_LABEL[f.type] }}<template v-if="f.options?.length"> — {{ f.options.join('، ') }}</template></span>
            </div>
            <StatusBadge :tone="f.active ? 'success' : 'neutral'" :label="f.active ? 'نشط' : 'موقوف'" />
            <div v-if="canWrite" class="flex gap-1">
              <button type="button" class="rounded p-1.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary" @click="openEdit(f)"><Pencil class="size-3.5" /></button>
              <button type="button" class="rounded p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger" @click="remove(f)"><Trash class="size-3.5" /></button>
            </div>
          </li>
        </ul>
      </AppCard>

      <AppCard title="قوالب وحدات جاهزة" subtitle="اختيارية — تضيف وحدات مقترحة حسب نوع النشاط">
        <div class="space-y-2">
          <AppButton v-for="p in PRESET_OPTIONS" :key="p.value" size="sm" block :loading="applyingPreset" :disabled="!canWrite" @click="applyPreset(p.value)">
            {{ p.label }}
          </AppButton>
        </div>
        <p class="mt-3 text-xs text-text-secondary">
          إدارة الوحدات نفسها من <RouterLink :to="{ name: 'categories' }" class="text-primary hover:underline">التصنيفات والوحدات</RouterLink>.
        </p>
      </AppCard>
    </div>

    <AppModal v-model:open="modalOpen" :title="editing ? 'تعديل حقل' : 'حقل جديد'" size="sm">
      <form id="cf-form" class="space-y-4" novalidate @submit.prevent="save">
        <AppInput v-model="form.name" label="اسم الحقل" required :error="errors.name" placeholder="مثال: المادة الفعالة" />
        <AppSelect v-model="form.type" label="النوع" :options="TYPE_OPTIONS" />
        <AppInput v-if="form.type === 'list'" v-model="form.optionsText" label="الخيارات" :error="errors.optionsText" placeholder="افصل بينها بفاصلة" />
        <AppSwitch v-model="form.active" label="نشط" description="الحقول الموقوفة لا تظهر في بطاقة المنتج" />
      </form>
      <template #footer>
        <AppButton :disabled="saving" @click="modalOpen = false">إلغاء</AppButton>
        <AppButton type="submit" form="cf-form" variant="primary" :loading="saving">حفظ</AppButton>
      </template>
    </AppModal>
  </div>
</template>
