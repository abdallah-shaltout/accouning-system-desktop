<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import { errorMessage, useToast } from '@/modules/core/controllers/useToast';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import CategoryDefaultsCard from '../components/CategoryDefaultsCard.vue';
import NamedListManager from '../components/NamedListManager.vue';
import { useCatalogStore } from '../controllers/useCatalogStore';
import { deleteCategory, deleteUnit, saveCategory, saveUnit } from '../services/catalogService';

const catalog = useCatalogStore();
const auth = useAuthStore();
const toast = useToast();
const readonly = computed(() => !auth.can('inventory', 'write'));
const loading = ref(true);
const error = ref<string | null>(null);

async function load() {
  loading.value = true;
  error.value = null;
  try {
    await catalog.load(true);
  } catch (err) {
    error.value = errorMessage(err);
  } finally {
    loading.value = false;
  }
}
onMounted(load);

async function updateUnitExtra(id: string, symbol: string, allowsDecimals: boolean) {
  try {
    const unit = catalog.units.find((u) => u.id === id)!;
    await saveUnit(unit.name, id, { symbol: symbol || undefined, allowsDecimals });
    await catalog.load(true);
  } catch (err) {
    toast.error(err);
  }
}
</script>

<template>
  <div>
    <PageHeader title="التصنيفات والوحدات" subtitle="تنظيم المنتجات ووحدات البيع" />
    <ErrorState v-if="error" :message="error" @retry="load" />
    <div v-else class="space-y-5">
      <div class="grid items-start gap-5 lg:grid-cols-2">
        <NamedListManager
          title="التصنيفات"
          item-label="التصنيف"
          :items="catalog.categories"
          :loading="loading"
          :readonly="readonly"
          :save="saveCategory"
          :remove="deleteCategory"
          @changed="load"
        />
        <div class="space-y-5">
          <NamedListManager
            title="الوحدات"
            item-label="الوحدة"
            :items="catalog.units"
            :loading="loading"
            :readonly="readonly"
            :save="saveUnit"
            :remove="deleteUnit"
            @changed="load"
          />
          <AppCard v-if="catalog.units.length" title="رمز الوحدة والكسور" padding="none">
            <ul class="divide-y divide-border">
              <li v-for="u in catalog.units" :key="u.id" class="flex items-center gap-3 px-4 py-2">
                <span class="min-w-24 flex-1 truncate text-body">{{ u.name }}</span>
                <input
                  :value="u.symbol"
                  class="control h-8 w-20"
                  dir="ltr"
                  placeholder="رمز"
                  :disabled="readonly"
                  @change="updateUnitExtra(u.id, ($event.target as HTMLInputElement).value, !!u.allowsDecimals)"
                />
                <AppSwitch :model-value="!!u.allowsDecimals" :disabled="readonly" @update:model-value="(v) => updateUnitExtra(u.id, u.symbol ?? '', v)" />
                <span class="text-tiny text-text-secondary">كسور</span>
              </li>
            </ul>
          </AppCard>
        </div>
      </div>
      <CategoryDefaultsCard v-if="catalog.categories.length" :categories="catalog.categories" :readonly="readonly" @changed="load" />
    </div>
  </div>
</template>
