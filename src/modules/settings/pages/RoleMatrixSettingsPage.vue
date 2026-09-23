<script setup lang="ts">
import { computed, ref } from 'vue';
import { Minus, Pencil, RotateCcw } from '@lucide/vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { ROLE_LABEL } from '@/modules/core/helpers/labels';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { ROLE_ACCESS } from '@/modules/users/helpers/permissions';
import type { Access, Area, Role } from '@/modules/users/types';
import SettingsTabs from '../components/SettingsTabs.vue';
import { useSettingsStore } from '../controllers/useSettingsStore';

/**
 * v2 §6 "Role matrix editor" (docs/v2/07-products-and-inventory.md, docs/v2/01-personas.md §5): a
 * straightforward permission-toggle grid, not a full ACL builder. Presets (`ROLE_ACCESS`) stay the
 * baseline; clicking a cell cycles it through none → read → write and stores only the *changed*
 * cells as an override in `settings.roleAccessOverrides` (see `permissions.ts`).
 */
const store = useSettingsStore();
const auth = useAuthStore();
const toast = useToast();
const canWrite = computed(() => auth.can('users', 'write'));
const saving = ref(false);

const ROLES: Role[] = ['admin', 'manager', 'accountant', 'cashier', 'storekeeper'];
const AREA_LABEL: Record<Area, string> = {
  dashboard: 'الرئيسية', pos: 'نقطة البيع', sales: 'الفواتير والمرتجعات', inventory: 'المنتجات والمخزون',
  parties: 'العملاء والموردين', purchases: 'المشتريات', expenses: 'المصروفات', accounting: 'الحسابات والقيود', payments: 'سندات القبض والصرف',
  reports: 'التقارير', users: 'إدارة المستخدمين', settings: 'الإعدادات',
};
const AREAS = Object.keys(AREA_LABEL) as Area[];

const CYCLE: Access[] = ['none', 'read', 'write'];
const ACCESS_LABEL: Record<Access, string> = { none: '—', read: 'عرض', write: 'كامل' };

const overrides = computed(() => store.settings?.roleAccessOverrides ?? {});

function current(role: Role, area: Area): Access {
  return overrides.value[role]?.[area] ?? ROLE_ACCESS[role][area];
}
function isOverridden(role: Role, area: Area): boolean {
  return overrides.value[role]?.[area] !== undefined;
}

async function cycle(role: Role, area: Area) {
  if (!canWrite.value || saving.value) return;
  const preset = ROLE_ACCESS[role][area];
  const now = current(role, area);
  const next = CYCLE[(CYCLE.indexOf(now) + 1) % CYCLE.length];
  const nextOverrides = structuredClone(overrides.value);
  if (next === preset) {
    // Back to the preset value — drop the override entirely, don't store a no-op.
    delete nextOverrides[role]?.[area];
    if (nextOverrides[role] && Object.keys(nextOverrides[role]!).length === 0) delete nextOverrides[role];
  } else {
    nextOverrides[role] = { ...nextOverrides[role], [area]: next };
  }
  saving.value = true;
  try {
    await store.update({ roleAccessOverrides: nextOverrides });
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

async function resetRole(role: Role) {
  if (!canWrite.value) return;
  const nextOverrides = structuredClone(overrides.value);
  delete nextOverrides[role];
  saving.value = true;
  try {
    await store.update({ roleAccessOverrides: nextOverrides });
    toast.success(`تمت إعادة "${ROLE_LABEL[role]}" لصلاحيات القالب الافتراضية`);
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div>
    <PageHeader title="المستخدمون والأدوار" subtitle="مصفوفة الصلاحيات لكل دور — انقر خلية للتبديل بين لا يوجد / عرض / كامل" />
    <SettingsTabs />

    <AppCard padding="none">
      <div class="overflow-x-auto">
        <table class="w-full text-body">
          <thead class="bg-surface text-xs text-text-secondary">
            <tr class="border-b border-border">
              <th class="min-w-40 px-4 py-2.5 text-start font-medium">القسم</th>
              <th v-for="role in ROLES" :key="role" class="px-3 py-2.5 text-start font-medium">
                <div class="flex items-center gap-1.5">
                  {{ ROLE_LABEL[role] }}
                  <button
                    v-if="canWrite"
                    type="button"
                    class="rounded p-0.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary"
                    :title="`إعادة ${ROLE_LABEL[role]} للافتراضي`"
                    @click="resetRole(role)"
                  >
                    <RotateCcw class="size-3" />
                  </button>
                </div>
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="area in AREAS" :key="area" class="border-b border-border last:border-0">
              <td class="px-4 py-2 font-medium">{{ AREA_LABEL[area] }}</td>
              <td v-for="role in ROLES" :key="role" class="px-3 py-1.5">
                <button
                  type="button"
                  class="inline-flex h-7 min-w-16 items-center justify-center gap-1 rounded-md border px-2 text-xs transition-colors"
                  :class="[
                    current(role, area) === 'write' ? 'border-success/40 bg-success/10 text-success' : current(role, area) === 'read' ? 'border-border bg-surface-hover text-text-primary' : 'border-transparent text-text-secondary',
                    isOverridden(role, area) && 'ring-1 ring-primary/50',
                    !canWrite && 'cursor-not-allowed opacity-70',
                  ]"
                  :disabled="!canWrite"
                  :title="isOverridden(role, area) ? 'معدّلة عن القالب الافتراضي' : undefined"
                  @click="cycle(role, area)"
                >
                  <Pencil v-if="current(role, area) === 'write'" class="size-3" />
                  <Minus v-else-if="current(role, area) === 'none'" class="size-3" />
                  {{ ACCESS_LABEL[current(role, area)] }}
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </AppCard>
    <p class="mt-3 text-xs text-text-secondary">الخلايا المحاطة بإطار أزرق مُعدَّلة عن قالب الدور الافتراضي.</p>
  </div>
</template>
