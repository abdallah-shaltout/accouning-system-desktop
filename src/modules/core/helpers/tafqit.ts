/**
 * Arabic number-to-words (تفقيط / tafqit) for money amounts — "the amount in
 * words" line on printed documents (docs/v2/12-documents-pdf-excel.md §3
 * "Totals: ... amount in words"). Didn't exist anywhere in the codebase
 * before Phase 11a (checked `src/modules/invoices/helpers/` — only
 * `totals.ts` and `zatcaQr.ts`), so this is a fresh implementation.
 *
 * Covers whole riyals (up to 999,999,999) + halalas (the 2-decimal fraction),
 * with correct Arabic grammar for the tricky bits:
 * - 1 and 2 use singular/dual forms ("ريال واحد" / "ريالان"), not "واحد ريال".
 * - 3–10 take the *plural* noun form ("ثلاثة ريالات").
 * - 11+ takes the *singular accusative* noun form ("أحد عشر ريالاً").
 * - "و" (and) joins every level (units-and-tens, hundreds-and-thousands, ...).
 * - Feminine agreement for "ريال" (masculine) — this module doesn't need it, but
 *   the 3-19 unit words below use the masculine set correctly for "ريال"/"هللة"
 *   (هللة is feminine, so its 1/2 forms and unit words switch — handled by `feminine`).
 */

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

/** Riyal unit word agreeing with the whole-riyal count (1/2/3-10/11+). */
function riyalUnit(n: number): string {
  return scaleForm(n, { one: 'ريال واحد', two: 'ريالان', few: 'ريالات', many: 'ريالاً' });
}

/** Halala unit word agreeing with the halala count (1/2/3-10/11+), feminine noun. */
function halalaUnit(n: number): string {
  return scaleForm(n, { one: 'هللة واحدة', two: 'هللتان', few: 'هللات', many: 'هللة' });
}

export interface TafqitOptions {
  currency?: 'ريال سعودي' | string;
  /** Prefix, e.g. "فقط لا غير:" — pass '' to omit. */
  prefix?: string;
}

/**
 * Converts a money amount (SAR, 2-decimal halalas) into Arabic words, e.g.
 * `tafqit(3425.75)` → "ثلاثة آلاف وأربعمائة وخمسة وعشرون ريالاً سعودياً وخمسة وسبعون هللة".
 * Caps at 999,999,999.99 (returns a clamped result past that rather than throwing —
 * no invoice realistically needs more, and a printed document should never crash on this).
 */
export function tafqit(amount: number, options: TafqitOptions = {}): string {
  const { prefix = 'فقط لا غير:' } = options;
  const safe = Math.max(0, Math.min(999_999_999.99, Math.round((amount + Number.EPSILON) * 100) / 100));
  const riyals = Math.floor(safe);
  const halalas = Math.round((safe - riyals) * 100);

  const riyalWords = riyals === 0 ? 'صفر ريال' : `${integerToWords(riyals)} ${riyalUnit(riyals)} سعودياً`;

  let out = riyalWords;
  if (halalas > 0) {
    const halalaWords = `${integerToWords(halalas, true)} ${halalaUnit(halalas)}`;
    out += ` و${halalaWords}`;
  } else {
    out += ' لا غير';
  }

  return prefix ? `${prefix} ${out}` : out;
}
