import { ref } from 'vue';

/**
 * Number/date formatting for an Arabic RTL UI.
 * - Digits: Latin (1234) by default, Arabic-Indic (١٢٣٤) optional — a UI preference in localStorage.
 * - Calendar: always Gregorian (`ar-SA` alone would default to the Hijri calendar).
 * Wrap formatted numbers in an element with the `.num` class so they stay LTR inside RTL text.
 */
export type Numerals = 'latn' | 'arab';

const NUMERALS_KEY = 'app_numerals';

function loadNumerals(): Numerals {
  try {
    return localStorage.getItem(NUMERALS_KEY) === 'arab' ? 'arab' : 'latn';
  } catch {
    return 'latn';
  }
}

export const numeralSystem = ref<Numerals>(loadNumerals());

export function setNumerals(value: Numerals): void {
  numeralSystem.value = value;
  try {
    localStorage.setItem(NUMERALS_KEY, value);
  } catch {
    /* private mode — preference just won't persist */
  }
}

const cache = new Map<string, Intl.NumberFormat | Intl.DateTimeFormat | Intl.RelativeTimeFormat>();

/**
 * Intl's Arabic output embeds bidi controls (RLM between date parts, ALM/LRM before minus signs).
 * Inside our LTR-isolated `.num` spans those marks scramble the order, so strip them.
 */
const BIDI_MARKS = /[\u200e\u200f\u061c]/g;
function clean(s: string): string {
  return s.replace(BIDI_MARKS, '');
}

function locale(): string {
  return `ar-SA-u-ca-gregory-nu-${numeralSystem.value}`;
}

function numberFormat(options: Intl.NumberFormatOptions): Intl.NumberFormat {
  const key = `n|${locale()}|${JSON.stringify(options)}`;
  let f = cache.get(key) as Intl.NumberFormat | undefined;
  if (!f) {
    f = new Intl.NumberFormat(locale(), options);
    cache.set(key, f);
  }
  return f;
}

function dateFormat(options: Intl.DateTimeFormatOptions): Intl.DateTimeFormat {
  const key = `d|${locale()}|${JSON.stringify(options)}`;
  let f = cache.get(key) as Intl.DateTimeFormat | undefined;
  if (!f) {
    f = new Intl.DateTimeFormat(locale(), options);
    cache.set(key, f);
  }
  return f;
}

/** 1,234.50 — always two decimals. Negative values keep their minus sign. */
export function formatMoney(value: number | undefined | null): string {
  return clean(numberFormat({ minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(value ?? 0));
}

export function formatNumber(value: number | undefined | null, maxFractionDigits = 2): string {
  return clean(numberFormat({ maximumFractionDigits: maxFractionDigits }).format(value ?? 0));
}

export function formatPercent(value: number | undefined | null): string {
  return `${formatNumber(value ?? 0, 2)}%`;
}

/** Plain digits (no grouping) — for document numbers, barcodes, VAT numbers. */
export function formatDigits(value: string | number | undefined | null): string {
  const s = String(value ?? '');
  if (numeralSystem.value === 'latn') return s;
  return s.replace(/[0-9]/g, (d) => '٠١٢٣٤٥٦٧٨٩'[Number(d)]);
}

export function formatDate(iso: string | undefined | null): string {
  if (!iso) return '—';
  return clean(dateFormat({ year: 'numeric', month: '2-digit', day: '2-digit' }).format(new Date(iso)));
}

export function formatDateLong(iso: string | undefined | null): string {
  if (!iso) return '—';
  return clean(dateFormat({ year: 'numeric', month: 'long', day: 'numeric', weekday: 'long' }).format(new Date(iso)));
}

export function formatTime(iso: string | undefined | null): string {
  if (!iso) return '';
  // 24-hour clock: unambiguous on receipts and avoids mixing an Arabic AM/PM letter into LTR digits.
  return clean(dateFormat({ hour: '2-digit', minute: '2-digit', hourCycle: 'h23' }).format(new Date(iso)));
}

export function formatDateTime(iso: string | undefined | null): string {
  if (!iso) return '—';
  return `${formatDate(iso)} ${formatTime(iso)}`;
}

export function formatRelative(iso: string): string {
  const key = `r|${locale()}`;
  let f = cache.get(key) as Intl.RelativeTimeFormat | undefined;
  if (!f) {
    f = new Intl.RelativeTimeFormat(locale(), { numeric: 'auto' });
    cache.set(key, f);
  }
  const diffSec = (new Date(iso).getTime() - Date.now()) / 1000;
  const abs = Math.abs(diffSec);
  if (abs < 60) return 'الآن';
  if (abs < 3600) return clean(f.format(Math.round(diffSec / 60), 'minute'));
  if (abs < 86400) return clean(f.format(Math.round(diffSec / 3600), 'hour'));
  if (abs < 86400 * 7) return clean(f.format(Math.round(diffSec / 86400), 'day'));
  return formatDate(iso);
}

// ---------------------------------------------------------------------------------------------
// Date keys (YYYY-MM-DD, local time) for filters and <input type="date">.
// ---------------------------------------------------------------------------------------------

export function toDateKey(date: Date | string): string {
  const d = typeof date === 'string' ? new Date(date) : date;
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

export function todayKey(): string {
  return toDateKey(new Date());
}

export function daysAgoKey(days: number): string {
  const d = new Date();
  d.setDate(d.getDate() - days);
  return toDateKey(d);
}

export function startOfMonthKey(date = new Date()): string {
  return toDateKey(new Date(date.getFullYear(), date.getMonth(), 1));
}

/** Combine a YYYY-MM-DD key with the current time of day → ISO timestamp. */
export function dateKeyToIso(key: string): string {
  const now = new Date();
  const [y, m, d] = key.split('-').map(Number);
  return new Date(y, m - 1, d, now.getHours(), now.getMinutes(), now.getSeconds()).toISOString();
}
