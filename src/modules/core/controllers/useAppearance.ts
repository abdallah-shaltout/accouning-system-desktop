import { ref, watch } from 'vue';

/**
 * Appearance settings v2 (docs/v2/14-platform.md §3) — font family, text size, density, accent,
 * base palette + radius (docs/v2/17 Phase E), date format, week start, table prefs, motion, sidebar
 * default. All **per-device/per-user**, stored in localStorage exactly like `useTheme.ts` and
 * `format.ts`'s numeral setting — there is no per-user record in the mock backend for UI prefs, so
 * this matches the existing pattern rather than inventing a new storage layer.
 */

// --- Font family ---------------------------------------------------------------------------

export type FontFamily = 'cairo' | 'ibm-plex-sans-arabic' | 'tajawal' | 'noto-naskh-arabic';

interface FontDef {
  label: string;
  /** Exact `font-family` name as declared in the package's @font-face rules. */
  cssName: string;
  /** Dynamic import loading that font's stylesheet (code-split, not bundled upfront). */
  load: () => Promise<unknown>;
}

export const FONTS: Record<FontFamily, FontDef> = {
  cairo: {
    label: 'Cairo',
    cssName: 'Cairo Variable',
    load: () => import('@fontsource-variable/cairo'),
  },
  'ibm-plex-sans-arabic': {
    label: 'IBM Plex Sans Arabic',
    cssName: 'IBM Plex Sans Arabic',
    load: () =>
      Promise.all([
        import('@fontsource/ibm-plex-sans-arabic/400.css'),
        import('@fontsource/ibm-plex-sans-arabic/500.css'),
        import('@fontsource/ibm-plex-sans-arabic/600.css'),
        import('@fontsource/ibm-plex-sans-arabic/700.css'),
      ]),
  },
  tajawal: {
    label: 'Tajawal',
    cssName: 'Tajawal',
    load: () =>
      Promise.all([
        import('@fontsource/tajawal/400.css'),
        import('@fontsource/tajawal/500.css'),
        import('@fontsource/tajawal/700.css'),
      ]),
  },
  'noto-naskh-arabic': {
    label: 'Noto Naskh Arabic',
    cssName: 'Noto Naskh Arabic',
    load: () =>
      Promise.all([
        import('@fontsource/noto-naskh-arabic/400.css'),
        import('@fontsource/noto-naskh-arabic/500.css'),
        import('@fontsource/noto-naskh-arabic/600.css'),
        import('@fontsource/noto-naskh-arabic/700.css'),
      ]),
  },
};

const FONT_FALLBACK = 'ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, Arial, sans-serif';
const loadedFonts = new Set<FontFamily>();

async function applyFont(font: FontFamily) {
  if (!loadedFonts.has(font)) {
    await FONTS[font].load();
    loadedFonts.add(font);
  }
  document.documentElement.style.setProperty('--font-sans', `"${FONTS[font].cssName}", ${FONT_FALLBACK}`);
}

// --- Text size ------------------------------------------------------------------------------

export type TextSize = 90 | 100 | 110 | 120 | 130;
export const TEXT_SIZES: TextSize[] = [90, 100, 110, 120, 130];

function applyTextSize(size: TextSize) {
  document.documentElement.style.setProperty('--font-scale', String(size / 100));
}

// --- Density ---------------------------------------------------------------------------------

export type Density = 'comfortable' | 'compact';

function applyDensity(density: Density) {
  document.documentElement.classList.toggle('density-compact', density === 'compact');
}

// --- Accent ------------------------------------------------------------------------------------

/**
 * 8 presets, each with light/dark values. Indigo matches the pre-Equal default primary so switching
 * to it is a no-op visually. Every preset's `--color-primary`/`--color-on-primary` pair (the actual
 * text-on-accent combination used by buttons/badges) is verified >= 4.5:1 by
 * `scripts/check-contrast.ts` (wired into `bun run check`), which reads the same hex values straight
 * out of design-system.css — so this doc comment states what the checker enforces, not a one-time
 * estimate that could drift. (The Equal green itself, logo mark #10886C, only clears ~4.4:1, which is
 * why both its shades are picked a touch darker — docs/v2/16-equal-rebrand-and-ui-kit.md Phase B.)
 */
export type AccentPreset = 'equal' | 'indigo' | 'teal' | 'rose' | 'amber' | 'blue' | 'violet' | 'orange';

export const ACCENTS: Record<AccentPreset, { label: string; light: string; lightHover: string; dark: string; darkHover: string; onAccent: string }> = {
  equal: { label: 'إيكوال (الأساسي)', light: '#0e7259', lightHover: '#0b6350', dark: '#0f7d63', darkHover: '#0d6b54', onAccent: '#ffffff' },
  indigo: { label: 'نيلي', light: '#4f46e5', lightHover: '#4338ca', dark: '#5e6ad2', darkHover: '#6b79e4', onAccent: '#ffffff' },
  teal: { label: 'فيروزي', light: '#0f766e', lightHover: '#115e59', dark: '#0f766e', darkHover: '#134e4a', onAccent: '#ffffff' },
  rose: { label: 'وردي', light: '#e11d48', lightHover: '#be123c', dark: '#be123c', darkHover: '#9f1239', onAccent: '#ffffff' },
  amber: { label: 'كهرماني', light: '#d97706', lightHover: '#b45309', dark: '#f0a33a', darkHover: '#f6b856', onAccent: '#1c1400' },
  blue: { label: 'أزرق', light: '#1d4ed8', lightHover: '#1e40af', dark: '#1d4ed8', darkHover: '#1e3a8a', onAccent: '#ffffff' },
  violet: { label: 'بنفسجي', light: '#6d28d9', lightHover: '#5b21b6', dark: '#6d28d9', darkHover: '#4c1d95', onAccent: '#ffffff' },
  orange: { label: 'برتقالي', light: '#ea580c', lightHover: '#c2410c', dark: '#fb923c', darkHover: '#fdba74', onAccent: '#1c1400' },
};

function applyAccent(accent: AccentPreset) {
  document.documentElement.setAttribute('data-accent', accent);
}

// --- Base palette (docs/v2/17 Phase E) -----------------------------------------------------

export type BasePalette = 'neutral' | 'zinc' | 'stone' | 'slate' | 'gray';

export const BASES: Record<BasePalette, { label: string; swatch: string; swatchDark: string }> = {
  neutral: { label: 'محايد', swatch: '#f4f4f5', swatchDark: '#17181c' },
  zinc: { label: 'زنك', swatch: '#f4f4f5', swatchDark: '#18181b' },
  stone: { label: 'حجري', swatch: '#f5f5f4', swatchDark: '#1c1917' },
  slate: { label: 'أردوازي', swatch: '#f1f5f9', swatchDark: '#1e293b' },
  gray: { label: 'رمادي', swatch: '#f3f4f6', swatchDark: '#1f2937' },
};

function applyBase(base: BasePalette) {
  if (base === 'neutral') {
    document.documentElement.removeAttribute('data-base');
  } else {
    document.documentElement.setAttribute('data-base', base);
  }
}

// --- Radius (docs/v2/17 Phase E) -------------------------------------------------------------

export type ThemeRadius = 0 | 0.25 | 0.375 | 0.5 | 0.75 | 1;
export const RADII: ThemeRadius[] = [0, 0.25, 0.375, 0.5, 0.75, 1];

function applyRadius(radius: ThemeRadius) {
  document.documentElement.style.setProperty('--radius', `${radius}rem`);
}

// --- Named theme presets (docs/v2/17 Phase E) -------------------------------------------------

export interface ThemePreset {
  label: string;
  base: BasePalette;
  accent: AccentPreset;
  radius: ThemeRadius;
}

export const THEME_PRESETS: Record<string, ThemePreset> = {
  equal: { label: 'إيكوال', base: 'slate', accent: 'equal', radius: 0.375 },
  classic: { label: 'كلاسيكي', base: 'slate', accent: 'indigo', radius: 0.5 },
  soft: { label: 'ناعم', base: 'stone', accent: 'teal', radius: 0.75 },
  sharp: { label: 'حاد', base: 'zinc', accent: 'equal', radius: 0 },
};

// --- Date format ---------------------------------------------------------------------------------

export type DateFormatStyle = 'dmy' | 'ymd';

// --- Week start -----------------------------------------------------------------------------------

export type WeekStart = 'sat' | 'sun' | 'mon';

// --- Storage helpers --------------------------------------------------------------------------

function makeSetting<T extends string | number | boolean>(key: string, fallback: T, parse: (raw: string) => T | undefined) {
  function load(): T {
    try {
      const raw = localStorage.getItem(key);
      if (raw === null) return fallback;
      const parsed = parse(raw);
      return parsed === undefined ? fallback : parsed;
    } catch {
      return fallback;
    }
  }
  const state = ref<T>(load()) as import('vue').Ref<T>;
  function set(value: T) {
    state.value = value;
    try {
      localStorage.setItem(key, String(value));
    } catch {
      /* private mode — preference just won't persist */
    }
  }
  return { state, set };
}

const fontSetting = makeSetting<FontFamily>('app_font_family', 'cairo', (raw) => (raw in FONTS ? (raw as FontFamily) : undefined));
const textSizeSetting = makeSetting<TextSize>('app_text_size', 100, (raw) => {
  const n = Number(raw);
  return TEXT_SIZES.includes(n as TextSize) ? (n as TextSize) : undefined;
});
const densitySetting = makeSetting<Density>('app_density', 'comfortable', (raw) => (raw === 'compact' || raw === 'comfortable' ? raw : undefined));
const accentSetting = makeSetting<AccentPreset>('app_accent', 'equal', (raw) => (raw in ACCENTS ? (raw as AccentPreset) : undefined));
const baseSetting = makeSetting<BasePalette>('app_base', 'slate', (raw) => (raw in BASES ? (raw as BasePalette) : undefined));
const radiusSetting = makeSetting<ThemeRadius>('app_radius', 0.375, (raw) => {
  const n = Number(raw);
  return RADII.includes(n as ThemeRadius) ? (n as ThemeRadius) : undefined;
});
const dateFormatSetting = makeSetting<DateFormatStyle>('app_date_format', 'dmy', (raw) => (raw === 'dmy' || raw === 'ymd' ? raw : undefined));
const hijriSetting = makeSetting<boolean>('app_show_hijri', false, (raw) => (raw === 'true' ? true : raw === 'false' ? false : undefined));
const weekStartSetting = makeSetting<WeekStart>('app_week_start', 'sun', (raw) => (raw === 'sat' || raw === 'sun' || raw === 'mon' ? raw : undefined));
const rowsPerPageSetting = makeSetting<25 | 50 | 100>('app_rows_per_page', 25, (raw) => {
  const n = Number(raw);
  return n === 25 || n === 50 || n === 100 ? (n as 25 | 50 | 100) : undefined;
});
const zebraRowsSetting = makeSetting<boolean>('app_zebra_rows', false, (raw) => (raw === 'true' ? true : raw === 'false' ? false : undefined));
const sidebarCollapsedDefaultSetting = makeSetting<boolean>('app_sidebar_collapsed_default', false, (raw) => (raw === 'true' ? true : raw === 'false' ? false : undefined));

export const fontFamily = fontSetting.state;
export const textSize = textSizeSetting.state;
export const density = densitySetting.state;
export const accent = accentSetting.state;
export const base = baseSetting.state;
export const radius = radiusSetting.state;
export const dateFormatStyle = dateFormatSetting.state;
export const showHijri = hijriSetting.state;
export const weekStart = weekStartSetting.state;
export const rowsPerPage = rowsPerPageSetting.state;
export const zebraRows = zebraRowsSetting.state;
/** Read by `DefaultLayout.vue` only to seed the sidebar's own per-session collapsed state. */
export const sidebarCollapsedDefault = sidebarCollapsedDefaultSetting.state;

export function setFontFamily(v: FontFamily) {
  fontSetting.set(v);
}
export function setTextSize(v: TextSize) {
  textSizeSetting.set(v);
}
export function setDensity(v: Density) {
  densitySetting.set(v);
}
export function setAccent(v: AccentPreset) {
  accentSetting.set(v);
}
export function setBase(v: BasePalette) {
  baseSetting.set(v);
}
export function setRadius(v: ThemeRadius) {
  radiusSetting.set(v);
}
/** Sets base + accent + radius together from a named preset (the customizer's preset cards). */
export function applyThemePreset(name: keyof typeof THEME_PRESETS) {
  const preset = THEME_PRESETS[name];
  setBase(preset.base);
  setAccent(preset.accent);
  setRadius(preset.radius);
}
export function setDateFormatStyle(v: DateFormatStyle) {
  dateFormatSetting.set(v);
}
export function setShowHijri(v: boolean) {
  hijriSetting.set(v);
}
export function setWeekStart(v: WeekStart) {
  weekStartSetting.set(v);
}
export function setRowsPerPage(v: 25 | 50 | 100) {
  rowsPerPageSetting.set(v);
}
export function setZebraRows(v: boolean) {
  zebraRowsSetting.set(v);
}
export function setSidebarCollapsedDefault(v: boolean) {
  sidebarCollapsedDefaultSetting.set(v);
}

/** Applies every appearance setting to the document and wires reactive re-application. Call once at boot. */
export function initAppearance() {
  void applyFont(fontFamily.value);
  applyTextSize(textSize.value);
  applyDensity(density.value);
  applyAccent(accent.value);
  applyBase(base.value);
  applyRadius(radius.value);

  watch(fontFamily, (v) => void applyFont(v));
  watch(textSize, applyTextSize);
  watch(density, applyDensity);
  watch(accent, applyAccent);
  watch(base, applyBase);
  watch(radius, applyRadius);

  // Motion always runs at full power — remove the stale localStorage key if an earlier
  // version of the app left it behind (docs/v2/17-ui-system-rtl-themes.md Phase B).
  try {
    localStorage.removeItem('app_reduce_motion');
  } catch {
    /* private mode — nothing to clean up */
  }
}
