<script setup lang="ts">
/** docs/v2/05-onboarding.md §2 step 6: basic/standard/detailed CoA template picker with a live tree preview. */
import { computed } from 'vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import { previewCoaTemplate } from '../../services/setupService';
import type { WizardState } from '../../types';

const props = defineProps<{ state: WizardState }>();

const TEMPLATES: { value: 'basic' | 'standard' | 'detailed'; label: string; note: string }[] = [
  { value: 'basic', label: 'مبسّط', note: '~25 حساباً — لمشروع صغير بدون أصول ثابتة أو قروض' },
  { value: 'standard', label: 'قياسي (موصى به)', note: '~60 حساباً — يغطي معظم المتاجر' },
  { value: 'detailed', label: 'مفصّل', note: '~90 حساباً — شيكات، فروع، تأمينات اجتماعية' },
];

const preview = computed(() => previewCoaTemplate(props.state.coa.template, props.state.countryTax.country, props.state.businessType));
const roots = computed(() => preview.value.filter((a) => !a.parentId));
function children(parentId: string) {
  return preview.value.filter((a) => a.parentId === parentId);
}
</script>

<template>
  <div class="space-y-4">
    <div class="grid gap-3 sm:grid-cols-3">
      <button
        v-for="t in TEMPLATES"
        :key="t.value"
        type="button"
        class="rounded-xl border p-3 text-start transition-colors"
        :class="state.coa.template === t.value ? 'border-primary bg-primary/5' : 'border-border bg-surface hover:bg-surface-hover'"
        @click="state.coa.template = t.value"
      >
        <p class="text-sm font-semibold">{{ t.label }}</p>
        <p class="mt-1 text-xs leading-relaxed text-text-secondary">{{ t.note }}</p>
      </button>
    </div>

    <AppCard title="معاينة الشجرة" padding="sm">
      <div class="max-h-96 overflow-y-auto text-xs">
        <ul>
          <li v-for="root in roots" :key="root.id" class="py-0.5">
            <span class="font-semibold">{{ root.code }} — {{ root.name }}</span>
            <ul class="ms-4 border-s border-border ps-2">
              <li v-for="c in children(root.id)" :key="c.id" class="py-0.5">
                <span :class="c.isGroup ? 'font-medium' : 'text-text-secondary'">{{ c.code }} — {{ c.name }}</span>
                <ul class="ms-4 border-s border-border ps-2">
                  <li v-for="g in children(c.id)" :key="g.id" class="py-0.5 text-text-secondary">{{ g.code }} — {{ g.name }}</li>
                </ul>
              </li>
            </ul>
          </li>
        </ul>
      </div>
    </AppCard>
  </div>
</template>
