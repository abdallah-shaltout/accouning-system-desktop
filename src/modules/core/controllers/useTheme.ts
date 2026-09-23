import { ref } from 'vue';

export type ThemeMode = 'light' | 'dark' | 'system';

const THEME_KEY = 'app_theme';
const media = typeof window !== 'undefined' ? window.matchMedia('(prefers-color-scheme: dark)') : null;

function load(): ThemeMode {
  try {
    const saved = localStorage.getItem(THEME_KEY);
    return saved === 'dark' || saved === 'system' ? saved : 'light';
  } catch {
    return 'light';
  }
}

export const themeMode = ref<ThemeMode>(load());
export const resolvedTheme = ref<'light' | 'dark'>('light');

function apply() {
  const dark = themeMode.value === 'dark' || (themeMode.value === 'system' && !!media?.matches);
  resolvedTheme.value = dark ? 'dark' : 'light';
  const root = document.documentElement;
  root.classList.toggle('dark', dark);
  root.classList.toggle('light', !dark);
}

export function initTheme() {
  apply();
  media?.addEventListener('change', () => {
    if (themeMode.value === 'system') apply();
  });
}

export function setTheme(mode: ThemeMode) {
  themeMode.value = mode;
  try {
    localStorage.setItem(THEME_KEY, mode);
  } catch {
    /* ignore */
  }
  apply();
}

export function toggleTheme() {
  setTheme(resolvedTheme.value === 'dark' ? 'light' : 'dark');
}
