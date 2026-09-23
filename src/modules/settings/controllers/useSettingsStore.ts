import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import * as settingsService from '../services/settingsService';
import type { PaymentMethod, StoreSettings, Tax } from '../types';

/** Store settings + taxes + payment methods, loaded once after login and shared by POS, printing and money display. */
export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<StoreSettings | null>(null);
  const taxes = ref<Tax[]>([]);
  const paymentMethods = ref<PaymentMethod[]>([]);
  const loaded = ref(false);

  const currency = computed(() => settings.value?.currency ?? 'SAR');
  const pricesIncludeTax = computed(() => settings.value?.pricesIncludeTax !== false);
  const salesTax = computed(
    () =>
      taxes.value.find((t) => t.id === settings.value?.defaultTaxId && t.active) ??
      taxes.value.find((t) => t.type === 'OUTPUT' && t.isDefault && t.active),
  );
  const salesTaxRate = computed(() => salesTax.value?.rate ?? 0);
  const purchaseTaxRate = computed(() => taxes.value.find((t) => t.type === 'INPUT' && t.isDefault && t.active)?.rate ?? 0);

  async function load(force = false) {
    if (loaded.value && !force) return;
    const [s, t, pm] = await Promise.all([settingsService.getSettings(), settingsService.getTaxes(), settingsService.getPaymentMethods()]);
    settings.value = s;
    taxes.value = t;
    paymentMethods.value = pm;
    loaded.value = true;
  }

  async function update(patch: Partial<StoreSettings>) {
    settings.value = await settingsService.updateSettings(patch);
  }

  async function reloadTaxes() {
    taxes.value = await settingsService.getTaxes();
  }

  async function reloadPaymentMethods() {
    paymentMethods.value = await settingsService.getPaymentMethods();
  }

  return {
    settings,
    taxes,
    paymentMethods,
    loaded,
    currency,
    pricesIncludeTax,
    salesTax,
    salesTaxRate,
    purchaseTaxRate,
    load,
    update,
    reloadTaxes,
    reloadPaymentMethods,
  };
});
