/**
 * Arabic-aware search normalization, used by every search box in the app (`core/helpers/search.ts`
 * per docs/v2/14-platform.md §2). Normalize both the query and the searched text before comparing
 * so "احمد" finds "أحمد" and "٤٢" finds "INV-42".
 *
 * - Removes tashkeel (diacritics) and tatweel (kashida).
 * - Unifies letter variants: أ/إ/آ → ا, ة → ه, ى → ي, ؤ → و, ئ → ي.
 * - Converts Arabic-Indic digits (٠-٩) to Latin.
 * - Lower-cases Latin text.
 */

// Tashkeel (combining marks U+064B–U+065F, U+0670) + tatweel (U+0640).
const TASHKEEL_TATWEEL = /[ً-ٰٟـ]/g;

const LETTER_MAP: Record<string, string> = {
  'أ': 'ا',
  'إ': 'ا',
  'آ': 'ا',
  'ة': 'ه',
  'ى': 'ي',
  'ؤ': 'و',
  'ئ': 'ي',
};

const ARABIC_INDIC_DIGITS = '٠١٢٣٤٥٦٧٨٩';

export function normalizeArabic(value: string | undefined | null): string {
  if (!value) return '';
  let s = value.replace(TASHKEEL_TATWEEL, '');
  s = s.replace(/[أإآةىؤئ]/g, (ch) => LETTER_MAP[ch] ?? ch);
  s = s.replace(/[٠-٩]/g, (d) => String(ARABIC_INDIC_DIGITS.indexOf(d)));
  return s.trim().toLowerCase();
}

/** True if `needle` (normalized) is found inside any of `haystack` (each normalized). */
export function matchesSearch(haystack: (string | undefined | null)[], needle: string | undefined | null): boolean {
  const q = normalizeArabic(needle);
  if (!q) return true;
  return haystack.some((h) => normalizeArabic(h).includes(q));
}

/**
 * Place-name normalization on top of `normalizeArabic` (doc 18.E `AddressFields`): also strips the
 * definite article "ال" and the leading "حي " ("district") so "الرياض" matches "رياض" and "حي العليا"
 * matches "العليا"/"عليا". Only used for geo place matching — general search stays `normalizeArabic`,
 * since stripping "ال" from arbitrary product/party names would over-match.
 */
export function normalizePlaceName(value: string | undefined | null): string {
  let s = normalizeArabic(value);
  s = s.replace(/^حي\s+/, '');
  s = s.replace(/\bال(?=\w)/g, '');
  return s.trim();
}

/** Like `matchesSearch`, but place-name tolerant (strips "ال"/"حي "). */
export function matchesPlaceSearch(haystack: (string | undefined | null)[], needle: string | undefined | null): boolean {
  const q = normalizePlaceName(needle);
  if (!q) return true;
  return haystack.some((h) => normalizePlaceName(h).includes(q));
}
