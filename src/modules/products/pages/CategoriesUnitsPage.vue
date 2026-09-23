<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import { errorMessage } from '@/modules/core/controllers/useToast';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import NamedListManager from '../components/NamedListManager.vue';
import { useCatalogStore } from '../controllers/useCatalogStore';
import { deleteCategory, deleteUnit, saveCategory, saveUnit } from '../services/catalogService';

const catalog = useCatalogStore();
const auth = useAuthStore();
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
</script>

<template>
  <div>
    <PageHeader title="التصنيفات والوحدات" subtitle="تنظيم المنتجات ووحدات البيع" />
    <ErrorState v-if="error" :message="error" @retry="load" />
    <div v-else class="grid items-start gap-5 lg:grid-cols-2">
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
    </div>
  </div>
</template>
