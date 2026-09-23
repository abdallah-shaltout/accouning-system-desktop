<script setup lang="ts">
import { nextTick, ref } from 'vue';
import { Check, Pencil, Plus, Trash, X } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatNumber } from '@/modules/core/helpers/format';

/** Inline CRUD for small name-only lookup tables (categories, units). */
const props = defineProps<{
  title: string;
  itemLabel: string;
  items: { id: string; name: string; productCount: number }[];
  loading?: boolean;
  readonly?: boolean;
  save: (name: string, id?: string) => Promise<unknown>;
  remove: (id: string) => Promise<unknown>;
}>();
const emit = defineEmits<{ changed: [] }>();

const toast = useToast();
const confirm = useConfirm();
const editingId = ref<string | null>(null);
const draft = ref('');
const busy = ref(false);
const input = ref<HTMLInputElement[]>();

async function startEdit(id: string | 'new', name = '') {
  editingId.value = id;
  draft.value = name;
  await nextTick();
  input.value?.[0]?.focus();
}

async function commit() {
  if (!editingId.value) return;
  busy.value = true;
  try {
    await props.save(draft.value, editingId.value === 'new' ? undefined : editingId.value);
    toast.success(editingId.value === 'new' ? `تمت إضافة ${props.itemLabel}` : 'تم الحفظ', draft.value);
    editingId.value = null;
    emit('changed');
  } catch (err) {
    toast.error(err);
  } finally {
    busy.value = false;
  }
}

async function destroy(item: { id: string; name: string; productCount: number }) {
  const ok = await confirm({
    title: `حذف ${props.itemLabel} "${item.name}"؟`,
    message: item.productCount ? `مرتبط بـ ${item.productCount} منتج ولا يمكن حذفه قبل نقلها.` : 'لا يمكن التراجع عن هذا الإجراء.',
    confirmText: 'حذف',
    danger: true,
  });
  if (!ok) return;
  try {
    await props.remove(item.id);
    toast.success('تم الحذف', item.name);
    emit('changed');
  } catch (err) {
    toast.error(err);
  }
}
</script>

<template>
  <AppCard :title="title" padding="none">
    <template v-if="!readonly" #actions>
      <AppButton size="sm" :icon="Plus" @click="startEdit('new')">إضافة</AppButton>
    </template>
    <div v-if="loading && !items.length" class="p-4"><SkeletonBlock :lines="5" /></div>
    <EmptyState v-else-if="!items.length && editingId !== 'new'" :title="`لا توجد ${title}`" compact />
    <ul v-else class="divide-y divide-border">
      <li v-for="item in items" :key="item.id" class="group flex h-11 items-center gap-2 px-4">
        <template v-if="editingId === item.id">
          <input ref="input" v-model="draft" class="control h-8 flex-1" @keydown.enter.prevent="commit" @keydown.esc="editingId = null" />
          <AppButton size="sm" variant="primary" :icon="Check" :loading="busy" @click="commit" />
          <AppButton size="sm" variant="ghost" :icon="X" @click="editingId = null" />
        </template>
        <template v-else>
          <span class="flex-1 truncate text-[13px]">{{ item.name }}</span>
          <span class="text-xs text-text-secondary"><span class="num">{{ formatNumber(item.productCount) }}</span> منتج</span>
          <div v-if="!readonly" class="flex opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
            <button type="button" class="rounded p-1.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary" aria-label="تعديل" @click="startEdit(item.id, item.name)">
              <Pencil class="size-3.5" />
            </button>
            <button type="button" class="rounded p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger" aria-label="حذف" @click="destroy(item)">
              <Trash class="size-3.5" />
            </button>
          </div>
        </template>
      </li>
      <li v-if="editingId === 'new'" class="flex h-11 items-center gap-2 px-4">
        <input ref="input" v-model="draft" :placeholder="`اسم ${itemLabel}`" class="control h-8 flex-1" @keydown.enter.prevent="commit" @keydown.esc="editingId = null" />
        <AppButton size="sm" variant="primary" :icon="Check" :loading="busy" @click="commit" />
        <AppButton size="sm" variant="ghost" :icon="X" @click="editingId = null" />
      </li>
    </ul>
  </AppCard>
</template>
