import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import * as settingsService from '../services/settingsService';
import type { StoreSettings, Tax } from '../types';

/** Store settings + taxes, loaded once after login and shared by POS, printing and money display. */
export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<StoreSettings | null>(null);
  const taxes = ref<Tax[]>([]);
  const loaded = ref(false);

  const currency = computed(() => settings.value?.currency ?? 'SAR');
  const salesTax = computed(
    () =>
      taxes.value.find((t) => t.id === settings.value?.defaultTaxId && t.active) ??
      taxes.value.find((t) => t.type === 'OUTPUT' && t.isDefault && t.active),
  );
  const salesTaxRate = computed(() => salesTax.value?.rate ?? 0);
  const purchaseTaxRate = computed(() => taxes.value.find((t) => t.type === 'INPUT' && t.isDefault && t.active)?.rate ?? 0);

  async function load(force = false) {
    if (loaded.value && !force) return;
    const [s, t] = await Promise.all([settingsService.getSettings(), settingsService.getTaxes()]);
    settings.value = s;
    taxes.value = t;
    loaded.value = true;
  }

  async function update(patch: Partial<StoreSettings>) {
    settings.value = await settingsService.updateSettings(patch);
  }

  async function reloadTaxes() {
    taxes.value = await settingsService.getTaxes();
  }

  return { settings, taxes, loaded, currency, salesTax, salesTaxRate, purchaseTaxRate, load, update, reloadTaxes };
});
