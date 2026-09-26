<script setup lang="ts">
/**
 * Entry point for the template designer: lists saved invoice templates and
 * opens the designer for one. Kept as its own page (not merged into any
 * settings/pages/*.vue file) per Phase 11a's scope — the designer route/page
 * must be wholly separate from the settings pages the concurrent Phase 13a
 * backup work touches.
 */
import { ref } from 'vue';
import { useRouter } from 'vue-router';
import { FilePlus2, Star } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import { formatDateTime } from '@/modules/core/helpers/format';
import { createTemplate, listTemplates } from '../services/templateService';

const router = useRouter();
const templates = ref(listTemplates('invoice'));

function openTemplate(id: string) {
  router.push({ name: 'settings-template-designer', params: { id } });
}

function newTemplate() {
  const created = createTemplate('invoice', 'invoice_standard', `قالب جديد ${templates.value.length + 1}`);
  router.push({ name: 'settings-template-designer', params: { id: created.id } });
}
</script>

<template>
  <div>
    <PageHeader title="قوالب الطباعة" subtitle="فاتورة ضريبية — قياسية ومبسطة" :back="{ name: 'settings-general' }">
      <template #actions>
        <AppButton variant="primary" :icon="FilePlus2" @click="newTemplate">قالب جديد</AppButton>
      </template>
    </PageHeader>

    <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
      <button
        v-for="t in templates"
        :key="t.id"
        type="button"
        class="text-start"
        @click="openTemplate(t.id)"
      >
        <AppCard padding="sm" class="h-full transition-colors hover:border-primary">
          <div class="flex items-start justify-between gap-2">
            <p class="font-medium text-text-primary">{{ t.name }}</p>
            <Star v-if="t.isDefault" class="size-4 shrink-0 fill-current text-primary" />
          </div>
          <p class="mt-1 text-xs text-text-secondary">{{ t.baseTemplateId === 'invoice_simplified' ? 'فاتورة مبسطة' : 'فاتورة قياسية' }}</p>
          <p class="num mt-2 text-xs text-text-secondary">آخر تحديث: {{ formatDateTime(t.updatedAt) }}</p>
        </AppCard>
      </button>
    </div>
  </div>
</template>
