<script setup lang="ts">
/**
 * Cascading country-aware address picker (doc 18.E `phase-e-address-picker.md`): region → city →
 * district `AppCombobox`es backed by `geoService` (lazy per-country JSON), each with a "غير موجود في
 * القائمة؟ اكتب يدوياً" free-text fallback, plus the per-country street/building fields from
 * `countryProfiles.ts`'s `address` schema. Changing a parent level clears its children (both the
 * picked id/name and any free text under it), since a stale child no longer belongs to the new
 * parent. Fully keyboard-usable — it's built on `AppCombobox`, which already wires reka-ui's listbox
 * keyboard handling.
 *
 * Emits a plain `Address` object via `v-model` — callers (`PartyFormPage`, `StepCompany`,
 * `GeneralSettingsPage`, branch rows) own persistence; this block only edits the shape.
 */
import { computed, ref, watch } from 'vue';
import AppCombobox, { type ComboOption } from '@/modules/core/components/ui/AppCombobox.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import { getCities, getDistricts, getLabels, getRegions, type GeoCity, type GeoDistrict, type GeoRegion } from '@/modules/core/services/geoService';
import type { Address } from '@/modules/core/types/address';
import type { CountryCode } from '@/modules/core/helpers/countryProfiles';

const props = defineProps<{ country: CountryCode; disabled?: boolean }>();
const model = defineModel<Address>({ default: () => ({ country: 'EG' }) as Address });

const regions = ref<GeoRegion[]>([]);
const cities = ref<GeoCity[]>([]);
const districts = ref<GeoDistrict[]>([]);
// Dataset-driven labels ("المحافظة" for EG, "المنطقة" for SA…) — never hard-code a country's terms
// here, `eg.json`/`sa.json`'s own `labels` block is the single source (doc 18.E E1's normalized shape).
const labels = ref({ region: 'المنطقة', city: 'المدينة', district: 'الحي' as string | null });

const regionFreeform = ref(!!model.value.regionFreeText && !model.value.regionId);
const cityFreeform = ref(!!model.value.cityFreeText && !model.value.cityId);
const districtFreeform = ref(!!model.value.districtFreeText && !model.value.districtId);

const regionOptions = computed<ComboOption[]>(() => regions.value.map((r) => ({ value: r.id, label: r.ar, keywords: r.en })));
const cityOptions = computed<ComboOption[]>(() => cities.value.map((c) => ({ value: c.id, label: c.ar, keywords: c.en })));
const districtOptions = computed<ComboOption[]>(() => districts.value.map((d) => ({ value: d.id, label: d.ar, keywords: d.en })));

const isSaudi = computed(() => props.country === 'SA');
const hasDistrictLevel = computed(() => districts.value.length > 0 || districtFreeform.value || !!model.value.districtId);

async function loadRegions() {
  const [regionList, labelSet] = await Promise.all([getRegions(props.country), getLabels(props.country)]);
  regions.value = regionList;
  labels.value = labelSet;
}

async function loadCities() {
  cities.value = model.value.regionId ? await getCities(props.country, model.value.regionId) : [];
}

async function loadDistricts() {
  districts.value = model.value.cityId ? await getDistricts(props.country, model.value.cityId) : [];
}

watch(() => props.country, loadRegions, { immediate: true });
watch(() => model.value.regionId, loadCities, { immediate: true });
watch(() => model.value.cityId, loadDistricts, { immediate: true });

// Cascading clear on the *id* itself (not just the `@select` event) so a parent level being
// cleared via `AppCombobox`'s "X" button — which sets its own v-model to undefined without firing
// `@select` — still invalidates whatever child was picked under it, exactly like a fresh pick does.
watch(
  () => model.value.regionId,
  (regionId, oldRegionId) => {
    if (regionId === oldRegionId || (!model.value.cityId && !model.value.cityName)) return;
    model.value = { ...model.value, cityId: undefined, cityName: undefined, cityFreeText: undefined, districtId: undefined, districtName: undefined, districtFreeText: undefined };
    cityFreeform.value = false;
    districtFreeform.value = false;
  },
);
watch(
  () => model.value.cityId,
  (cityId, oldCityId) => {
    if (cityId === oldCityId || (!model.value.districtId && !model.value.districtName)) return;
    model.value = { ...model.value, districtId: undefined, districtName: undefined, districtFreeText: undefined };
    districtFreeform.value = false;
  },
);

function onSelectRegion(option: ComboOption) {
  model.value = { ...model.value, regionId: option.value, regionName: option.label, regionFreeText: undefined };
}

function onSelectCity(option: ComboOption) {
  model.value = { ...model.value, cityId: option.value, cityName: option.label, cityFreeText: undefined };
}

function onSelectDistrict(option: ComboOption) {
  model.value = { ...model.value, districtId: option.value, districtName: option.label, districtFreeText: undefined };
}

function toggleRegionFreeform(v: boolean) {
  regionFreeform.value = v;
  if (v) model.value = { ...model.value, regionId: undefined, regionName: undefined };
  else model.value = { ...model.value, regionFreeText: undefined };
}

function toggleCityFreeform(v: boolean) {
  cityFreeform.value = v;
  if (v) model.value = { ...model.value, cityId: undefined, cityName: undefined };
  else model.value = { ...model.value, cityFreeText: undefined };
}

function toggleDistrictFreeform(v: boolean) {
  districtFreeform.value = v;
  if (v) model.value = { ...model.value, districtId: undefined, districtName: undefined };
  else model.value = { ...model.value, districtFreeText: undefined };
}
</script>

<template>
  <div class="grid gap-4 sm:grid-cols-3">
    <div class="sm:col-span-1">
      <AppCombobox
        v-if="!regionFreeform"
        v-model="model.regionId"
        :label="labels.region"
        :options="regionOptions"
        :disabled="disabled"
        clearable
        @select="onSelectRegion"
      />
      <AppInput v-else v-model="model.regionFreeText" :label="labels.region" :disabled="disabled" />
      <button
        type="button"
        class="mt-1 text-xs text-primary hover:underline"
        :disabled="disabled"
        @click="toggleRegionFreeform(!regionFreeform)"
      >
        {{ regionFreeform ? 'اختيار من القائمة' : 'غير موجود في القائمة؟ اكتب يدوياً' }}
      </button>
    </div>

    <div class="sm:col-span-1">
      <AppCombobox
        v-if="!cityFreeform"
        v-model="model.cityId"
        :label="labels.city"
        :options="cityOptions"
        :disabled="disabled || (!model.regionId && !regionFreeform)"
        clearable
        @select="onSelectCity"
      />
      <AppInput v-else v-model="model.cityFreeText" :label="labels.city" :disabled="disabled" />
      <button
        type="button"
        class="mt-1 text-xs text-primary hover:underline"
        :disabled="disabled"
        @click="toggleCityFreeform(!cityFreeform)"
      >
        {{ cityFreeform ? 'اختيار من القائمة' : 'غير موجود في القائمة؟ اكتب يدوياً' }}
      </button>
    </div>

    <div v-if="isSaudi || hasDistrictLevel" class="sm:col-span-1">
      <AppCombobox
        v-if="!districtFreeform"
        v-model="model.districtId"
        :label="labels.district ?? 'الحي'"
        :options="districtOptions"
        :disabled="disabled || (!model.cityId && !cityFreeform)"
        clearable
        @select="onSelectDistrict"
      />
      <AppInput v-else v-model="model.districtFreeText" :label="labels.district ?? 'الحي'" :disabled="disabled" />
      <button
        type="button"
        class="mt-1 text-xs text-primary hover:underline"
        :disabled="disabled"
        @click="toggleDistrictFreeform(!districtFreeform)"
      >
        {{ districtFreeform ? 'اختيار من القائمة' : 'غير موجود في القائمة؟ اكتب يدوياً' }}
      </button>
    </div>

    <AppInput v-model="model.street" label="الشارع" class="sm:col-span-2" :disabled="disabled" />

    <template v-if="isSaudi">
      <AppInput v-model="model.saBuildingNo" label="رقم المبنى" ltr :disabled="disabled" />
      <AppInput v-model="model.saAdditionalNo" label="الرقم الإضافي" ltr :disabled="disabled" />
      <AppInput v-model="model.saPostalCode" label="الرمز البريدي" ltr :disabled="disabled" />
      <AppInput v-model="model.saUnitNo" label="رقم الوحدة" ltr :disabled="disabled" />
      <AppInput v-model="model.saShortAddress" label="العنوان المختصر" ltr placeholder="RRRD2929" :disabled="disabled" />
    </template>
    <template v-else>
      <AppInput v-model="model.buildingNo" label="رقم المبنى" ltr :disabled="disabled" />
      <AppInput v-model="model.floor" label="الدور" ltr :disabled="disabled" />
      <AppInput v-model="model.apartment" label="الشقة" ltr :disabled="disabled" />
      <AppInput v-model="model.landmark" label="علامة مميزة" :disabled="disabled" />
      <AppInput v-model="model.postalCode" label="الرمز البريدي (اختياري)" ltr :disabled="disabled" />
    </template>
  </div>
</template>
