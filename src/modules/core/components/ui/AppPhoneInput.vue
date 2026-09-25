<script setup lang="ts">
/**
 * Phone input used by every phone field in the app (docs/v2/08-customers-and-suppliers.md §2).
 * - RTL layout: the country button sits at the *end* of the field; the number itself is LTR.
 * - Storage: E.164 (`+9665XXXXXXXX`), never the display-formatted string.
 * - Display formatting + validation: `libphonenumber-js/min`.
 * - Arabic-Indic digits are converted to Latin as the user types or pastes.
 * - Paste: strips spaces/dashes and a leading `00` or `+`; switches country if the pasted number
 *   carries a different dial code.
 */
import { computed, nextTick, onBeforeUnmount, ref, useId, watch } from 'vue';
import { AsYouType, isValidPhoneNumber, parsePhoneNumberFromString } from 'libphonenumber-js/min';
import { ChevronDown, Search } from '@lucide/vue';
import { COUNTRIES, countryByCode, countryByDialCode, DEFAULT_COUNTRY_CODE, type CountryInfo } from '../../helpers/countries';
import { matchesSearch } from '../../helpers/search';

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
  }>(),
  { defaultCountry: DEFAULT_COUNTRY_CODE, validate: true },
);

/** E.164, e.g. "+966567891234". Empty string / undefined when cleared. */
const model = defineModel<string | undefined>();

const id = useId();
const countryCode = ref(props.defaultCountry);
const nationalDigits = ref('');
const open = ref(false);
const query = ref('');
const localError = ref('');
const numberInput = ref<HTMLInputElement>();
const trigger = ref<HTMLButtonElement>();
const searchInput = ref<HTMLInputElement>();

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
    const currentE164 = nationalDigits.value ? `+${country.value.dialCode}${nationalDigits.value}` : '';
    if (v === currentE164) return; // change came from us, not the outside
    syncFromModel();
  },
);

function pushModel() {
  if (!nationalDigits.value) {
    model.value = undefined;
    localError.value = '';
    return;
  }
  const e164 = `+${country.value.dialCode}${nationalDigits.value}`;
  model.value = e164;
  if (props.validate) {
    // Validate the local `e164` string, not `model.value` — `defineModel` round-trips through the
    // parent's `v-model` binding, and reading it back synchronously in the same tick isn't
    // guaranteed to reflect the write yet (observed as `isValidPhoneNumber` receiving `undefined`
    // and libphonenumber-js throwing "A text for parsing must be a string").
    localError.value = isValidPhoneNumber(e164) ? '' : `رقم الجوال غير صحيح لـ${country.value.nameAr}`;
  }
}

function onInput(e: Event) {
  const raw = (e.target as HTMLInputElement).value;
  nationalDigits.value = toLatinDigits(raw).replace(/\D/g, '').slice(0, country.value.nsnLength + 2);
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
  nationalDigits.value = nationalDigits.value.slice(0, country.value.nsnLength + 2);
  pushModel();
}

const displayValue = computed(() => {
  if (!nationalDigits.value) return '';
  return new AsYouType(country.value.code as any).input(nationalDigits.value);
});

function selectCountry(c: CountryInfo) {
  countryCode.value = c.code;
  hide();
  pushModel();
  nextTick(() => numberInput.value?.focus());
}

function onBlur() {
  if (props.validate && nationalDigits.value) pushModel();
}

const filteredCountries = computed(() => {
  const q = query.value.trim();
  if (!q) return COUNTRIES;
  return COUNTRIES.filter((c) => matchesSearch([c.nameAr, c.nameEn, `+${c.dialCode}`, c.dialCode], q));
});

async function show() {
  if (props.disabled) return;
  open.value = true;
  query.value = '';
  await nextTick();
  searchInput.value?.focus();
}

function hide(refocus = false) {
  open.value = false;
  if (refocus) trigger.value?.focus();
}

function onDocClick(e: MouseEvent) {
  if (!open.value) return;
  const target = e.target as Node;
  if (trigger.value?.contains(target)) return;
  hide();
}

watch(open, (v) => {
  if (v) window.addEventListener('mousedown', onDocClick, true);
  else window.removeEventListener('mousedown', onDocClick, true);
});
onBeforeUnmount(() => window.removeEventListener('mousedown', onDocClick, true));

const shownError = computed(() => props.error || localError.value || undefined);

/** WhatsApp link for this number (digits only, no "+") — used by the party page's WhatsApp button. */
const whatsappHref = computed(() => (model.value ? `https://wa.me/${model.value.replace(/\D/g, '')}` : undefined));

defineExpose({ whatsappHref, focus: () => numberInput.value?.focus() });
</script>

<template>
  <div>
    <label v-if="label" :for="id" class="field-label">
      {{ label }}<span v-if="required" class="text-danger"> *</span>
    </label>
    <div class="relative flex items-stretch" dir="rtl">
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
        :maxlength="country.nsnLength + 4"
        :aria-invalid="!!shownError || undefined"
        class="control flex-1 rounded-e-none border-e-0 text-end"
        @input="onInput"
        @paste="onPaste"
        @blur="onBlur"
      />
      <button
        ref="trigger"
        type="button"
        :disabled="disabled"
        class="control flex shrink-0 items-center gap-1.5 rounded-s-none px-2.5"
        :class="disabled && 'cursor-not-allowed opacity-60'"
        @click="open ? hide() : show()"
      >
        <span class="text-base leading-none">{{ country.flag }}</span>
        <span class="num text-xs text-text-secondary">+{{ country.dialCode }}</span>
        <ChevronDown class="size-3.5 text-text-secondary" />
      </button>

      <div
        v-if="open"
        dir="rtl"
        class="absolute top-full z-50 mt-1 w-64 overflow-hidden rounded-lg border border-border bg-background shadow-xl"
        :class="'end-0'"
      >
        <div class="flex items-center gap-2 border-b border-border px-2.5">
          <Search class="size-4 text-text-secondary" />
          <input
            ref="searchInput"
            v-model="query"
            placeholder="بحث عن دولة…"
            class="h-9 w-full bg-transparent text-body outline-none placeholder:text-text-secondary"
          />
        </div>
        <ul class="max-h-64 overflow-y-auto p-1">
          <li v-for="c in filteredCountries" :key="c.code">
            <button
              type="button"
              class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-start text-body hover:bg-surface-hover"
              :class="c.code === country.code && 'bg-surface-hover'"
              @click="selectCountry(c)"
            >
              <span class="text-base leading-none">{{ c.flag }}</span>
              <span class="min-w-0 flex-1 truncate">{{ c.nameAr }}</span>
              <span class="num text-xs text-text-secondary">+{{ c.dialCode }}</span>
            </button>
          </li>
          <li v-if="!filteredCountries.length" class="px-2 py-6 text-center text-xs text-text-secondary">لا توجد نتائج</li>
        </ul>
      </div>
    </div>
    <p v-if="shownError" class="mt-1 text-xs text-danger">{{ shownError }}</p>
    <p v-else-if="hint" class="mt-1 text-xs text-text-secondary">{{ hint }}</p>
  </div>
</template>
