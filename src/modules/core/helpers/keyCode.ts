/**
 * Physical-key combo helpers, built on `KeyboardEvent.code` (the physical key position) instead of
 * `.key` (the character the layout produces). This is what makes Ctrl+K fire the same whether the
 * user's keyboard is set to English or Arabic — `.key` would give `'ك'` on an Arabic layout and
 * silently never match `'k'`. Combos are normalized strings like `ctrl+KeyK`, `shift+F8`, `Escape`.
 */

const MODIFIER_ORDER = ['ctrl', 'alt', 'shift'] as const;

/** Builds the canonical combo string for a live keydown event. */
export function comboOfEvent(e: KeyboardEvent): string {
  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push('ctrl');
  if (e.altKey) parts.push('alt');
  if (e.shiftKey) parts.push('shift');
  parts.push(e.code);
  return parts.join('+');
}

/** Normalizes a combo string (e.g. from storage or a static binding table) into canonical order. */
export function normalizeCombo(combo: string): string {
  const segments = combo.split('+');
  const code = segments.pop() ?? '';
  const mods = new Set(segments.map((s) => s.toLowerCase()));
  const parts: string[] = MODIFIER_ORDER.filter((m) => mods.has(m));
  parts.push(code);
  return parts.join('+');
}

const CODE_LABELS: Record<string, string> = {
  ControlLeft: 'Ctrl',
  ControlRight: 'Ctrl',
  AltLeft: 'Alt',
  AltRight: 'Alt',
  ShiftLeft: 'Shift',
  ShiftRight: 'Shift',
  Escape: 'Esc',
  Enter: 'Enter',
  Equal: '+',
  Minus: '-',
  Slash: '/',
  Backquote: '`',
};

function codeLabel(code: string): string {
  if (CODE_LABELS[code]) return CODE_LABELS[code];
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit\d$/.test(code)) return code.slice(5);
  if (/^F\d{1,2}$/.test(code)) return code;
  return code;
}

/** Human-readable label for display (e.g. `ctrl+KeyK` → `Ctrl+K`). */
export function comboLabel(combo: string): string {
  const segments = normalizeCombo(combo).split('+');
  const code = segments.pop() ?? '';
  const mods = segments.map((s) => s[0].toUpperCase() + s.slice(1));
  return [...mods, codeLabel(code)].join('+');
}
