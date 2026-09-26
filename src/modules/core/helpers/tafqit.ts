/**
 * Arabic number-to-words (تفقيط / tafqit) for money amounts — "the amount in
 * words" line on printed documents (docs/v2/12-documents-pdf-excel.md §3
 * "Totals: ... amount in words"). Didn't exist anywhere in the codebase
 * before Phase 11a (checked `src/modules/invoices/helpers/` — only
 * `totals.ts` and `zatcaQr.ts`), so this is a fresh implementation.
 *
 * Covers whole units (up to 999,999,999) + the minor unit (the 2-decimal
 * fraction), with correct Arabic grammar for the tricky bits:
 * - 1 and 2 use singular/dual forms ("ريال واحد" / "ريالان"), not "واحد ريال".
 * - 3–10 take the *plural* noun form ("ثلاثة ريالات").
 * - 11+ takes the *singular accusative* noun form ("أحد عشر ريالاً").
 * - "و" (and) joins every level (units-and-tens, hundreds-and-thousands, ...).
 * - Feminine agreement for the minor unit noun — "ريال"/"جنيه" are masculine,
 *   "هللة"/"قرش" flips (هللة feminine, قرش masculine) — see `TafqitCurrencyWords`.
 *
 * v2 doc 18.D: takes a `TafqitCurrencyWords` set (major/minor unit words +
 * nationality adjective + genders) instead of hard-coding "ريال"/"هللة"/"سعودياً",
 * so `countryProfiles.ts` supplies the right words per currency (EGP: جنيه/قرش,
 * SAR: ريال/هللة — unchanged wording/grammar for the existing SAR callers).
 * `TafqitCurrencyWords` is defined here (not in `countryProfiles.ts`) so
 * `countryProfiles.ts` importing it back is a type-only import — erased at
 * compile time, so there's no runtime import cycle between the two modules.
 */
import { profileByCurrency } from './countryProfiles';
import { round2 } from './numbers';

const ONES_M = ['', 'واحد', 'اثنان', 'ثلاثة', 'أربعة', 'خمسة', 'ستة', 'سبعة', 'ثمانية', 'تسعة'];
const ONES_F = ['', 'إحدى', 'اثنتان', 'ثلاث', 'أربع', 'خمس', 'ست', 'سبع', 'ثمان', 'تسع'];
const TEENS_M = ['عشرة', 'أحد عشر', 'اثنا عشر', 'ثلاثة عشر', 'أربعة عشر', 'خمسة عشر', 'ستة عشر', 'سبعة عشر', 'ثمانية عشر', 'تسعة عشر'];
const TEENS_F = ['عشرة', 'إحدى عشرة', 'اثنتا عشرة', 'ثلاث عشرة', 'أربع عشرة', 'خمس عشرة', 'ست عشرة', 'سبع عشرة', 'ثماني عشرة', 'تسع عشرة'];
const TENS = ['', '', 'عشرون', 'ثلاثون', 'أربعون', 'خمسون', 'ستون', 'سبعون', 'ثمانون', 'تسعون'];
const HUNDREDS = ['', 'مائة', 'مائتان', 'ثلاثمائة', 'أربعمائة', 'خمسمائة', 'ستمائة', 'سبعمائة', 'ثمانمائة', 'تسعمائة'];

interface ScaleWord {
  /** Singular ("ألف"), dual ("ألفان"), plural 3-10 ("آلاف"), plural 11+ ("ألفاً"). */
  one: string;
  two: string;
  few: string;
  many: string;
}

const THOUSAND: ScaleWord = { one: 'ألف', two: 'ألفان', few: 'آلاف', many: 'ألفاً' };
const MILLION: ScaleWord = { one: 'مليون', two: 'مليونان', few: 'ملايين', many: 'مليوناً' };

/** Words for an integer 0–999 (one "group" of 3 digits), masculine or feminine unit forms. */
function groupWords(n: number, feminine: boolean): string {
  if (n === 0) return '';
  const ones = feminine ? ONES_F : ONES_M;
  const teens = feminine ? TEENS_F : TEENS_M;
  const parts: string[] = [];

  const hundreds = Math.floor(n / 100);
  const rem = n % 100;
  if (hundreds > 0) parts.push(HUNDREDS[hundreds]);

  if (rem >= 10 && rem <= 19) {
    parts.push(teens[rem - 10]);
  } else {
    const tens = Math.floor(rem / 10);
    const unit = rem % 10;
    const unitWord = unit > 0 ? ones[unit] : '';
    const tensWord = tens > 0 ? TENS[tens] : '';
    if (unitWord && tensWord) parts.push(`${unitWord} و${tensWord}`);
    else if (tensWord) parts.push(tensWord);
    else if (unitWord) parts.push(unitWord);
  }

  return parts.join(' و');
}

/** Picks singular/dual/few(3-10)/many(11-99) agreement for a scale word (ألف/مليون) or a named unit. */
function scaleForm(n: number, scale: ScaleWord): string {
  if (n === 1) return scale.one;
  if (n === 2) return scale.two;
  if (n >= 3 && n <= 10) return scale.few;
  return scale.many;
}

/** Full integer → Arabic words, up to 999,999,999. */
function integerToWords(value: number, feminine = false): string {
  if (value === 0) return 'صفر';
  const million = Math.floor(value / 1_000_000);
  const thousand = Math.floor((value % 1_000_000) / 1000);
  const rest = value % 1000;

  const parts: string[] = [];

  if (million > 0) {
    if (million <= 2) {
      parts.push(scaleForm(million, MILLION));
    } else {
      parts.push(`${groupWords(million, false)} ${scaleForm(million, MILLION)}`);
    }
  }

  if (thousand > 0) {
    if (thousand <= 2) {
      parts.push(scaleForm(thousand, THOUSAND));
    } else {
      parts.push(`${groupWords(thousand, false)} ${scaleForm(thousand, THOUSAND)}`);
    }
  }

  if (rest > 0) {
    parts.push(groupWords(rest, feminine));
  }

  return parts.join(' و');
}

/** One noun's singular/dual/plural(3-10)/plural(11+) forms, e.g. { one: 'ريال واحد', two: 'ريالان', few: 'ريالات', many: 'ريالاً' }. */
export type TafqitUnitWords = ScaleWord;

/**
 * Per-currency word set `tafqit()` needs: the major/minor unit's four agreement forms, each unit's
 * grammatical gender (feminine units use the feminine 1-19 word set — هللة does, قرش/ريال/جنيه don't),
 * and the nationality adjective appended after the major unit ("...سعودياً" / "...مصرياً").
 */
export interface TafqitCurrencyWords {
  /** Bare major-unit noun for the "zero" case ("ريال" / "جنيه" — no agreement suffix). */
  majorNoun: string;
  major: TafqitUnitWords;
  majorFeminine: boolean;
  minor: TafqitUnitWords;
  minorFeminine: boolean;
  nationality: string;
}

/** The original SAR word set — unchanged wording, kept as the default so every pre-18.D caller (no `currency` passed) reads identically. */
const SAR_WORDS: TafqitCurrencyWords = {
  majorNoun: 'ريال',
  major: { one: 'ريال واحد', two: 'ريالان', few: 'ريالات', many: 'ريالاً' },
  majorFeminine: false,
  minor: { one: 'هللة واحدة', two: 'هللتان', few: 'هللات', many: 'هللة' },
  minorFeminine: true,
  nationality: 'سعودياً',
};

export interface TafqitOptions {
  /** Currency code (e.g. 'SAR', 'EGP') — looks up the word set via `countryProfiles.ts`. Omitted = SAR wording (unchanged). */
  currency?: string;
  /** Prefix, e.g. "فقط لا غير:" — pass '' to omit. */
  prefix?: string;
}

/**
 * Converts a money amount into Arabic words for the given currency (SAR by default, unchanged
 * wording), e.g. `tafqit(3425.75)` → "ثلاثة آلاف وأربعمائة وخمسة وعشرون ريالاً سعودياً وخمسة وسبعون
 * هللة"، or `tafqit(3425.75, { currency: 'EGP' })` → "...جنيهاً مصرياً وخمسة وسبعون قرشاً".
 * Caps at 999,999,999.99 (returns a clamped result past that rather than throwing —
 * no invoice realistically needs more, and a printed document should never crash on this).
 */
export function tafqit(amount: number, options: TafqitOptions = {}): string {
  const { prefix = 'فقط لا غير:', currency } = options;
  const words: TafqitCurrencyWords = currency ? profileByCurrency(currency).currency.words : SAR_WORDS;

  const safe = Math.max(0, Math.min(999_999_999.99, round2(amount)));
  const major = Math.floor(safe);
  const minor = Math.round(round2(safe - major) * 100);

  const majorUnitWord = scaleForm(major, words.major);
  const majorWords = major === 0 ? `صفر ${words.majorNoun}` : `${integerToWords(major, words.majorFeminine)} ${majorUnitWord} ${words.nationality}`;

  let out = majorWords;
  if (minor > 0) {
    const minorUnitWord = scaleForm(minor, words.minor);
    out += ` و${integerToWords(minor, words.minorFeminine)} ${minorUnitWord}`;
  } else {
    out += ' لا غير';
  }

  return prefix ? `${prefix} ${out}` : out;
}
