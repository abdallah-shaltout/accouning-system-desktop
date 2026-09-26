import { onBeforeUnmount, onMounted } from 'vue';
import { comboOfEvent, normalizeCombo } from '../helpers/keyCode';
import { getCombo, registerShortcut } from './useKeybindings';

/** Return `false` to mean "not handled" — the key then keeps its default behavior. */
type Handler = (event: KeyboardEvent) => boolean | void;

/** A plain binding is a fixed combo. A customizable binding resolves its live combo from `useKeybindings`. */
type Binding = Handler | { id: string; label: string; group: string; handler: Handler };

/**
 * Register page-level keyboard shortcuts, e.g. `{ F9: newSale, 'ctrl+Enter': pay }`.
 *
 * Combo keys are matched against `KeyboardEvent.code` (the physical key), not `.key` (the character
 * the active keyboard layout produces) — so `ctrl+KeyK` fires the same on an English or Arabic
 * layout. Plain letters/digits use their `Key*`/`Digit*` code implicitly when written as a single
 * lowercase letter or digit (e.g. `'k'` → `KeyK`, `'j'` → `KeyJ`); everything else (F-keys, Escape,
 * Enter, Tab, symbols) is matched by its own code name or common symbol alias (`+`, `-`, `?`, `/`).
 *
 * A binding written as `{ id, label, group, handler }` instead of a bare handler is registered with
 * `useKeybindings` and becomes rebindable from Settings → اختصارات لوحة المفاتيح; its live combo (the
 * user's override, or the object key as default) is looked up on every keystroke instead of the
 * literal key, so a rebind takes effect immediately without remounting.
 *
 * Function keys fire even while typing in an input; plain keys are ignored inside form fields.
 */
export function useHotkeys(bindings: Record<string, Binding>, options: { enabled?: () => boolean } = {}) {
  const resolved = Object.entries(bindings).map(([combo, binding]) => {
    const normalizedDefault = normalizeCombo(aliasCombo(combo));
    if (typeof binding === 'function') {
      return { comboOf: () => normalizedDefault, handler: binding };
    }
    registerShortcut({ id: binding.id, label: binding.label, group: binding.group, defaultCombo: normalizedDefault });
    return { comboOf: () => getCombo(binding.id) ?? normalizedDefault, handler: binding.handler };
  });

  function onKeydown(e: KeyboardEvent) {
    if (options.enabled && !options.enabled()) return;
    const live = comboOfEvent(e);
    const match = resolved.find((r) => r.comboOf() === live);
    if (!match) return;
    const target = e.target as HTMLElement | null;
    const typing = !!target && (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName));
    const isFunctionKey = /^F\d{1,2}$/.test(e.code) || e.code === 'Escape' || e.ctrlKey || e.metaKey;
    if (typing && !isFunctionKey) return;
    if (match.handler(e) !== false) e.preventDefault();
  }

  onMounted(() => window.addEventListener('keydown', onKeydown));
  onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown));
}

const SYMBOL_ALIASES: Record<string, string> = {
  '+': 'Equal',
  '-': 'Minus',
  '=': 'Equal',
  '?': 'Slash',
  '/': 'Slash',
  '`': 'Backquote',
};

/** Accepts the old `.key`-style literals (`'k'`, `'Enter'`, `'F1'`, `'+'`) and maps them to a `code`. */
function aliasCombo(combo: string): string {
  const segments = combo.split('+');
  const last = segments.pop() ?? '';
  const mapped = SYMBOL_ALIASES[last] ?? (/^[a-zA-Z]$/.test(last) ? `Key${last.toUpperCase()}` : /^\d$/.test(last) ? `Digit${last}` : last);
  return [...segments, mapped].join('+');
}
