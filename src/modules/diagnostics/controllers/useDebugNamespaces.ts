import { reactive } from 'vue';

/**
 * Debug-channel namespace toggles (18.B5) — off by default, per device, read by `logService`'s
 * `debugEnabled()` from `localStorage['equal.debug']` (comma-separated, `pos.*` prefix wildcard).
 * This controller is the single place that reads/writes that key, so the dev menu, the command
 * palette and the `/dev/diagnostics` "التتبع" tab always agree on what's enabled.
 */
const STORAGE_KEY = 'equal.debug';

/** A short list of namespaces the app actually uses `log.debug(namespace, …)` with today, plus
 * whatever the user already typed in by hand (`custom` entries below) — not an exhaustive registry,
 * just enough for the toggle UI to suggest something instead of a blank text box. */
export const KNOWN_NAMESPACES = ['posting', 'pos.*', 'accounting', 'inventory', 'sync'];

function load(): Set<string> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return new Set();
    return new Set(raw.split(',').map((s) => s.trim()).filter(Boolean));
  } catch {
    return new Set();
  }
}

function persist(set: Set<string>): void {
  try {
    if (set.size === 0) localStorage.removeItem(STORAGE_KEY);
    else localStorage.setItem(STORAGE_KEY, [...set].join(','));
  } catch {
    /* private mode — preference just won't persist */
  }
}

const state = reactive({ namespaces: load() });

export function useDebugNamespaces() {
  function isEnabled(namespace: string): boolean {
    return state.namespaces.has(namespace);
  }

  function toggle(namespace: string): void {
    if (state.namespaces.has(namespace)) state.namespaces.delete(namespace);
    else state.namespaces.add(namespace);
    persist(state.namespaces);
  }

  function enable(namespace: string): void {
    state.namespaces.add(namespace);
    persist(state.namespaces);
  }

  function disable(namespace: string): void {
    state.namespaces.delete(namespace);
    persist(state.namespaces);
  }

  function clear(): void {
    state.namespaces.clear();
    persist(state.namespaces);
  }

  return { namespaces: state.namespaces, isEnabled, toggle, enable, disable, clear };
}
