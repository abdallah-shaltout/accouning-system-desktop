<script setup lang="ts">
/**
 * Phone input used by every phone field in the app (docs/v2/08-customers-and-suppliers.md §2).
 * - RTL layout: the country button sits at the *end* of the field; the number itself is LTR.
 * - Storage: E.164 (`+9665XXXXXXXX`), never the display-formatted string.
 * - Display formatting + parsing: `libphonenumber-js/min`, which strips a typed national trunk
 *   prefix (EG/SA leading `0`) correctly — this component never concatenates `+dial+digits` itself.
 * - Validation only runs after the field has been touched (blurred once), so no error flashes while
 *   the user is still typing (18.A1 #2).
 * - Arabic-Indic digits are converted to Latin as the user types or pastes.
 * - Paste: strips spaces/dashes and a leading `00` or `+`; switches country if the pasted number
 *   carries a different dial code.
 * - Country dropdown rebuilt on shadcn's Combobox (reka-ui), like `AppCombobox` — real arrow-key
 *   nav, Escape/Tab handling and listbox ARIA instead of a hand-rolled `<ul>` + window listener
 *   (18.A1 #6).
 */
import { computed, nextTick, ref, useId, watch } from 'vue';
import { AsYouType, isValidPhoneNumber, parsePhoneNumberFromString } from 'libphonenumber-js/min';
import { Check, ChevronDown } from '@lucide/vue';
import {
  Combobox,
  ComboboxAnchor,
  ComboboxTrigger,
  ComboboxInput,
  ComboboxList,
  ComboboxViewport,
  ComboboxEmpty,
  ComboboxItem,
  ComboboxItemIndicator,
} from '@/modules/core/components/shadcn/combobox';
import { COUNTRIES, countryByCode, countryByDialCode, DEFAULT_COUNTRY_CODE, type CountryInfo } from '../../helpers/countries';
import { matchesSearch } from '../../helpers/search';
import CountryFlag from './CountryFlag.vue';

const props = withDefaults(
  defineProps<{
    label?: string;
    placeholder?: string;
    error?: string;
    hint?: string;
    required?: boolean;
    disabled?: boolean;
    /** Default country (ISO alpha-2) when the value is empty — normally the store's country. */
    defaultCountry?: string;
    /** Validate on blur and show the inline Arabic error if the number is incomplete/invalid. */
    validate?: boolean;
    /**
     * 'mobile' requires a mobile-type number (landlines rejected); 'any' (default) accepts a
     * company landline too. Only changes validation + wording (18.A1 #5) — storage is unaffected.
     */
    kind?: 'mobile' | 'any';
  }>(),
  { defaultCountry: DEFAULT_COUNTRY_CODE, validate: true, kind: 'any' },
);

/** E.164, e.g. "+966567891234". Empty string / undefined when cleared. */
const model = defineModel<string | undefined>();

const id = useId();
const countryCode = ref(props.defaultCountry);
const nationalDigits = ref('');
const touched = ref(false);
const localError = ref('');
const numberInput = ref<HTMLInputElement>();
const open = ref(false);
const query = ref('');

const ARABIC_INDIC = '٠١٢٣٤٥٦٧٨٩';
function toLatinDigits(s: string): string {
  return s.replace(/[٠-٩]/g, (d) => String(ARABIC_INDIC.indexOf(d)));
}

const country = computed<CountryInfo>(() => countryByCode(countryCode.value) ?? countryByCode(DEFAULT_COUNTRY_CODE)!);

/** Parse the incoming E.164 model value into { countryCode, nationalDigits } without feedback loops. */
function syncFromModel() {
  const v = model.value?.trim();
  if (!v) {
    nationalDigits.value = '';
    return;
  }
  const parsed = parsePhoneNumberFromString(v);
  if (parsed) {
    countryCode.value = parsed.country ?? countryByDialCode(parsed.countryCallingCode)?.code ?? countryCode.value;
    nationalDigits.value = parsed.nationalNumber;
  } else {
    // Not (yet) a valid E.164 — keep the raw digits so the user doesn't lose what they typed.
    nationalDigits.value = toLatinDigits(v).replace(/^\+?\d{0,4}/, (m) => (v.startsWith('+') ? m.replace(/^\+/, '') : ''));
  }
}
syncFromModel();
watch(
  model,
  (v, old) => {
    if (v === old) return;
    const currentE164 = nationalDigits.value ? parsePhoneNumberFromString(nationalDigits.value, country.value.code as any)?.number : '';
    if (v === currentE164) return; // change came from us, not the outside
    syncFromModel();
  },
);

function validateNow() {
  if (!nationalDigits.value) {
    localError.value = '';
    return;
  }
  const parsed = parsePhoneNumberFromString(nationalDigits.value, country.value.code as any);
  const valid = !!parsed && isValidPhoneNumber(parsed.number);
  const kindOk = valid && (props.kind !== 'mobile' || parsed!.getType() === 'MOBILE' || parsed!.getType() === 'FIXED_LINE_OR_MOBILE');
  localError.value = kindOk ? '' : props.kind === 'mobile' ? `رقم الجوال غير صحيح لـ${country.value.nameAr}` : `رقم الهاتف غير صحيح لـ${country.value.nameAr}`;
}

function pushModel() {
  if (!nationalDigits.value) {
    model.value = undefined;
    if (touched.value) validateNow();
    return;
  }
  // Parsing (not concatenation) strips a typed national trunk prefix, e.g. EG "01012345678" or
  // SA "0501234567", correctly (18.A1 #3).
  const parsed = parsePhoneNumberFromString(nationalDigits.value, country.value.code as any);
  model.value = parsed?.number ?? `+${country.value.dialCode}${nationalDigits.value}`;
  if (touched.value && props.validate) validateNow();
}

function onInput(e: Event) {
  const raw = (e.target as HTMLInputElement).value;
  nationalDigits.value = toLatinDigits(raw).replace(/\D/g, '');
  pushModel();
}

function onPaste(e: ClipboardEvent) {
  const pasted = e.clipboardData?.getData('text') ?? '';
  if (!pasted) return;
  e.preventDefault();
  let digits = toLatinDigits(pasted).replace(/[^\d+]/g, '');
  if (digits.startsWith('+')) digits = digits.slice(1);
  else if (digits.startsWith('00')) digits = digits.slice(2);

  // Try to detect a different country's dial code in the pasted text.
  const matchedCountry = COUNTRIES.filter((c) => digits.startsWith(c.dialCode)).sort((a, b) => b.dialCode.length - a.dialCode.length)[0];
  if (matchedCountry && digits.length > matchedCountry.dialCode.length) {
    countryCode.value = matchedCountry.code;
    nationalDigits.value = digits.slice(matchedCountry.dialCode.length);
  } else {
    nationalDigits.value = digits.startsWith(country.value.dialCode) ? digits.slice(country.value.dialCode.length) : digits;
  }
  touched.value = true;
  pushModel();
}

const displayValue = computed(() => {
  if (!nationalDigits.value) return '';
  return new AsYouType(country.value.code as any).input(nationalDigits.value);
});

function onSelectCountry(v: unknown) {
  const c = countryByCode(v as string);
  if (!c) return;
  countryCode.value = c.code;
  open.value = false;
  pushModel();
  nextTick(() => numberInput.value?.focus());
}

function onBlur() {
  touched.value = true;
  if (props.validate) validateNow();
}

const filteredCountries = computed(() => {
  const q = query.value.trim();
  if (!q) return COUNTRIES;
  return COUNTRIES.filter((c) => matchesSearch([c.nameAr, c.nameEn, `+${c.dialCode}`, c.dialCode], q));
});

const shownError = computed(() => props.error || localError.value || undefined);
const errorId = computed(() => (shownError.value ? `${id}-error` : undefined));

/** WhatsApp link for this number (digits only, no "+") — used by the party page's WhatsApp button. */
const whatsappHref = computed(() => (model.value ? `https://wa.me/${model.value.replace(/\D/g, '')}` : undefined));

defineExpose({ whatsappHref, focus: () => numberInput.value?.focus() });
</script>

<template>
  <div>
    <label v-if="label" :for="id" class="field-label">
      {{ label }}<span v-if="required" class="text-danger" aria-hidden="true"> *</span>
    </label>
    <div class="flex items-stretch" dir="rtl">
      <input
        :id="id"
        ref="numberInput"
        type="tel"
        inputmode="numeric"
        autocomplete="tel-national"
        dir="ltr"
        :value="displayValue"
        :placeholder="placeholder ?? country.placeholder"
        :disabled="disabled"
        :aria-invalid="!!shownError || undefined"
        :aria-describedby="errorId"
        :aria-required="required || undefined"
        class="control w-auto min-w-0 flex-1 rounded-e-none border-e-0 text-end"
        @input="onInput"
        @paste="onPaste"
        @blur="onBlur"
      />

      <Combobox
        :model-value="countryCode"
        :open="open"
        ignore-filter
        :disabled="disabled"
        @update:model-value="onSelectCountry"
        @update:open="(v) => { open = v; if (v) query = ''; }"
      >
        <ComboboxAnchor class="w-auto shrink-0">
          <ComboboxTrigger as-child aria-haspopup="listbox" :aria-label="`دولة الرقم: ${country.nameAr}`">
            <button
              type="button"
              :disabled="disabled"
              class="control flex w-auto shrink-0 items-center gap-1.5 rounded-s-none px-2.5"
              :class="disabled && 'cursor-not-allowed opacity-60'"
            >
              <CountryFlag :code="country.code" />
              <span class="num text-xs text-text-secondary">+{{ country.dialCode }}</span>
              <ChevronDown class="size-3.5 text-text-secondary" />
            </button>
          </ComboboxTrigger>
        </ComboboxAnchor>

        <ComboboxList dir="rtl" class="w-64 min-w-60 rounded-lg border-border bg-background shadow-xl">
          <ComboboxInput v-model="query" placeholder="بحث عن دولة…" class="text-body" />
          <ComboboxViewport class="max-h-64 p-1">
            <ComboboxItem
              v-for="c in filteredCountries"
              :key="c.code"
              :value="c.code"
              class="rounded-md px-2 py-1.5 text-body"
            >
              <CountryFlag :code="c.code" />
              <span class="min-w-0 flex-1 truncate">{{ c.nameAr }}</span>
              <span class="num text-xs text-text-secondary">+{{ c.dialCode }}</span>
              <ComboboxItemIndicator>
                <Check class="size-4 text-primary" />
              </ComboboxItemIndicator>
            </ComboboxItem>
            <ComboboxEmpty class="text-xs text-text-secondary">لا توجد نتائج</ComboboxEmpty>
          </ComboboxViewport>
        </ComboboxList>
      </Combobox>
    </div>
    <p v-if="shownError" :id="errorId" class="mt-1 text-xs text-danger" role="alert">{{ shownError }}</p>
    <p v-else-if="hint" class="mt-1 text-xs text-text-secondary">{{ hint }}</p>
  </div>
</template>
