<script setup lang="ts">
import { computed } from 'vue';
import { Barcode, Plus, Sparkles, Trash } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { generateEan13 } from '../services/productService';
import { useCatalogStore } from '../controllers/useCatalogStore';
import type { ProductUnit } from '../types';

/**
 * v2 §2 "الوحدات والباركود" tab: base unit + additional units table (unit / factor / barcodes /
 * default-for-sale / default-for-purchase), "Generate EAN-13" per row, and the §2 validation rules
 * (exactly one factor === 1, factors > 0, a moved-stock unit's factor can't change — enforced
 * server-side in productService.validateUnits; this component just keeps the base row's factor
 * pinned to 1 and disables it).
 */
const props = defineProps<{ locked?: boolean; hasStock?: boolean }>();
const units = defineModel<ProductUnit[]>({ default: () => [] });

const toast = useToast();
const catalog = useCatalogStore();
let seq = 0;

const unitOptions = computed(() => catalog.units.map((u) => ({ value: u.id, label: u.symbol ? `${u.name} (${u.symbol})` : u.name })));

function addRow() {
  units.value = [
    ...units.value,
    { id: `new-${++seq}-${Date.now()}`, unitId: '', factor: units.value.length ? undefined! : 1, barcodes: [], price: 0, priceIsAuto: true, defaultForSale: units.value.length === 0, defaultForPurchase: false, active: true },
  ];
}

function removeRow(id: string) {
  units.value = units.value.filter((u) => u.id !== id);
}

function isLockedFactor(row: ProductUnit): boolean {
  // §2: once stock has moved, an *existing* (already-saved) unit's factor is locked.
  return !!props.hasStock && !row.id.startsWith('new-');
}

function addBarcode(row: ProductUnit) {
  row.barcodes = [...row.barcodes, ''];
}
function removeBarcode(row: ProductUnit, i: number) {
  row.barcodes = row.barcodes.filter((_, idx) => idx !== i);
}

async function generate(row: ProductUnit, i: number) {
  try {
    const code = await generateEan13();
    row.barcodes[i] = code;
  } catch (err) {
    toast.error(err);
  }
}

function setDefaultForSale(row: ProductUnit) {
  for (const u of units.value) u.defaultForSale = u.id === row.id;
}
function setDefaultForPurchase(row: ProductUnit) {
  for (const u of units.value) u.defaultForPurchase = u.id === row.id;
}
</script>

<template>
  <div class="space-y-3">
    <div v-if="!units.length" class="rounded-lg border border-dashed border-border p-4 text-center text-body text-text-secondary">
      يستخدم هذا المنتج وحدة واحدة فقط (الوحدة الأساسية في تبويب "أساسي"). أضف وحدات إضافية هنا لحالات مثل العلبة والشريط.
    </div>
    <div v-else class="overflow-x-auto rounded-lg border border-border">
      <table class="w-full text-body">
        <thead class="bg-surface text-xs text-text-secondary">
          <tr class="border-b border-border">
            <th class="px-3 py-2 text-start font-medium">الوحدة</th>
            <th class="px-3 py-2 text-start font-medium">يحتوي على (بالوحدة الأساسية)</th>
            <th class="px-3 py-2 text-start font-medium">الباركود</th>
            <th class="px-3 py-2 text-start font-medium">السعر</th>
            <th class="px-3 py-2 text-start font-medium">افتراضي للبيع</th>
            <th class="px-3 py-2 text-start font-medium">افتراضي للشراء</th>
            <th class="px-3 py-2 text-start font-medium">نشطة</th>
            <th class="w-10" />
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in units" :key="row.id" class="border-b border-border align-top last:border-0">
            <td class="min-w-40 px-3 py-2">
              <AppSelect v-model="row.unitId" placeholder="اختر وحدة…" :options="unitOptions" :disabled="locked" />
            </td>
            <td class="w-32 px-3 py-2">
              <input
                v-model.number="row.factor"
                type="number"
                min="0"
                step="any"
                class="control h-8 w-24"
                :disabled="locked || row.factor === 1 || isLockedFactor(row)"
                :title="isLockedFactor(row) ? 'لا يمكن تغيير العامل بعد تحرك المخزون — أضف وحدة جديدة بدلاً من ذلك' : undefined"
              />
              <p v-if="row.factor === 1" class="mt-0.5 text-tiny text-text-secondary">الوحدة الأساسية</p>
            </td>
            <td class="min-w-56 px-3 py-2">
              <div v-for="(_bc, i) in row.barcodes" :key="i" class="mb-1 flex items-center gap-1 last:mb-0">
                <input v-model="row.barcodes[i]" class="control h-8 flex-1" dir="ltr" placeholder="باركود" :disabled="locked" />
                <button v-if="!locked" type="button" class="rounded p-1 text-text-secondary hover:bg-danger/10 hover:text-danger" aria-label="حذف الباركود" @click="removeBarcode(row, i)">
                  <Trash class="size-3.5" />
                </button>
                <button v-if="!locked" type="button" class="rounded p-1 text-text-secondary hover:bg-surface-hover hover:text-primary" title="توليد EAN-13" @click="generate(row, i)">
                  <Sparkles class="size-3.5" />
                </button>
              </div>
              <AppButton v-if="!locked" size="sm" variant="ghost" :icon="Barcode" @click="addBarcode(row)">إضافة باركود</AppButton>
            </td>
            <td class="w-28 px-3 py-2">
              <input v-model.number="row.price" type="number" min="0" class="control h-8 w-24" :disabled="locked" @input="row.priceIsAuto = false" />
              <p v-if="row.priceIsAuto" class="mt-0.5 text-tiny text-primary">تلقائي</p>
            </td>
            <td class="px-3 py-2"><input type="radio" name="default-sale" :checked="row.defaultForSale" class="size-4" :disabled="locked" @change="setDefaultForSale(row)" /></td>
            <td class="px-3 py-2"><input type="radio" name="default-purchase" :checked="row.defaultForPurchase" class="size-4" :disabled="locked" @change="setDefaultForPurchase(row)" /></td>
            <td class="px-3 py-2"><AppSwitch v-model="row.active" :disabled="locked" /></td>
            <td class="px-2 py-2">
              <button v-if="!locked" type="button" class="rounded p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger" aria-label="حذف الوحدة" @click="removeRow(row.id)">
                <Trash class="size-3.5" />
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <AppButton v-if="!locked" size="sm" :icon="Plus" @click="addRow">إضافة وحدة</AppButton>
    <p class="text-xs leading-5 text-text-secondary">
      المخزون والتكلفة دائماً بالوحدة الأساسية (أصغر وحدة). الوحدات الأخرى مجرد عوامل تحويل ولها باركود وسعر خاص بها —
      لا يُشترط أن يكون السعر = سعر الأساسي × العامل.
    </p>
  </div>
</template>
