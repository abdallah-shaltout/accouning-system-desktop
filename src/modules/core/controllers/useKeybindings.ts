import { reactive } from 'vue';
import { normalizeCombo } from '../helpers/keyCode';

/**
 * User-customizable keyboard shortcuts. Per-device preference, stored in localStorage exactly like
 * `useAppearance.ts` — there is no per-user record in the mock backend for UI prefs. Each shortcut
 * that should be rebindable registers a default combo once via `registerShortcut`; the app then reads
 * its *effective* combo (override if the user set one, else the default) through `getCombo`.
 */

export interface ShortcutDef {
  id: string;
  /** Arabic label shown in the shortcuts settings page and the F1 sheet. */
  label: string;
  /** Group heading in the settings page (e.g. "عام", "نقطة البيع"). */
  group: string;
  /** Default physical-key combo, e.g. `ctrl+KeyK`, `F9`, `shift+F8`. */
  defaultCombo: string;
}

const STORAGE_KEY = 'app_keybinding_overrides';

const registry = new Map<string, ShortcutDef>();

function loadOverrides(): Record<string, string> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as Record<string, string>) : {};
  } catch {
    return {};
  }
}

function saveOverrides(overrides: Record<string, string>) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(overrides));
  } catch {
    /* private mode — preference just won't persist */
  }
}

const overrides = reactive<Record<string, string>>(loadOverrides());

/** Registers a shortcut's default binding. Safe to call on every mount — re-registering is a no-op. */
export function registerShortcut(def: ShortcutDef): void {
  if (registry.has(def.id)) return;
  registry.set(def.id, { ...def, defaultCombo: normalizeCombo(def.defaultCombo) });
}

/** The shortcut's currently effective combo (override if set, else its default). */
export function getCombo(id: string): string | undefined {
  return overrides[id] ?? registry.get(id)?.defaultCombo;
}

/** Every combo currently bound to `id`'s action is unavailable for other actions, and vice versa. */
export function findConflict(combo: string, excludeId: string): ShortcutDef | undefined {
  const normalized = normalizeCombo(combo);
  for (const def of registry.values()) {
    if (def.id !== excludeId && getCombo(def.id) === normalized) return def;
  }
  return undefined;
}

export function setCombo(id: string, combo: string): void {
  overrides[id] = normalizeCombo(combo);
  saveOverrides(overrides);
}

export function resetCombo(id: string): void {
  delete overrides[id];
  saveOverrides(overrides);
}

export function resetAllCombos(): void {
  for (const key of Object.keys(overrides)) delete overrides[key];
  saveOverrides(overrides);
}

/**
 * Every customizable shortcut's identity (id/label/group/default combo), so the settings page can
 * list them all up front instead of only what happens to already be registered by a mounted page's
 * `useHotkeys` call. Must be kept in sync with each `{ id, label, group, handler }` binding declared
 * across the app — `registerShortcut` is idempotent, so this only ever sets the default once; the
 * live `useHotkeys` call on the owning page is still what actually matches key events.
 */
export function registerDefaultShortcuts(): void {
  const defaults: ShortcutDef[] = [
    { id: 'palette.toggle', label: 'فتح لوحة الأوامر', group: 'عام', defaultCombo: 'ctrl+KeyK' },
    { id: 'shortcuts.sheet.toggle', label: 'عرض اختصارات لوحة المفاتيح', group: 'عام', defaultCombo: 'F1' },
    { id: 'print.document', label: 'طباعة المستند', group: 'الطباعة', defaultCombo: 'ctrl+KeyP' },
    { id: 'journal.post', label: 'ترحيل القيد', group: 'القيود المالية', defaultCombo: 'ctrl+Enter' },
    { id: 'journal.saveDraft', label: 'حفظ كمسودة', group: 'القيود المالية', defaultCombo: 'ctrl+KeyS' },
    { id: 'pos.search', label: 'البحث / الباركود', group: 'نقطة البيع', defaultCombo: 'F2' },
    { id: 'pos.pickCustomer', label: 'اختيار العميل', group: 'نقطة البيع', defaultCombo: 'F4' },
    { id: 'pos.holdResume', label: 'تعليق / استئناف البيع', group: 'نقطة البيع', defaultCombo: 'F6' },
    { id: 'pos.return', label: 'إرجاع', group: 'نقطة البيع', defaultCombo: 'F7' },
    { id: 'pos.lineDiscount', label: 'خصم الصنف', group: 'نقطة البيع', defaultCombo: 'F8' },
    { id: 'pos.invoiceDiscount', label: 'خصم الفاتورة', group: 'نقطة البيع', defaultCombo: 'shift+F8' },
    { id: 'pos.newSale', label: 'بيع جديد', group: 'نقطة البيع', defaultCombo: 'F9' },
    { id: 'pos.cashInOut', label: 'إيداع/سحب نقدي', group: 'نقطة البيع', defaultCombo: 'F10' },
    { id: 'pos.checkout', label: 'الدفع', group: 'نقطة البيع', defaultCombo: 'F12' },
    { id: 'pos.reprintReceipt', label: 'إعادة طباعة الإيصال', group: 'نقطة البيع', defaultCombo: 'ctrl+KeyP' },
  ];
  for (const def of defaults) registerShortcut(def);
}

/** Reactive view of every registered shortcut, grouped, for the settings page. */
export function useKeybindings() {
  function list(): (ShortcutDef & { combo: string; isCustom: boolean })[] {
    return [...registry.values()]
      .map((def) => ({ ...def, combo: getCombo(def.id)!, isCustom: def.id in overrides }))
      .sort((a, b) => a.group.localeCompare(b.group, 'ar') || a.label.localeCompare(b.label, 'ar'));
  }

  return { list, overrides, setCombo, resetCombo, resetAllCombos, findConflict };
}
