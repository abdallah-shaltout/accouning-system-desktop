/**
 * Keyboard shortcuts sheet registry (docs/v2/14-platform.md §7 "Keyboard help: F1 / ? opens the
 * keyboard-shortcuts sheet for the current page"). Keyed by route `name`, so `KeyboardShortcutsSheet`
 * can show only what's relevant to the page the user is actually on. These aren't new bindings —
 * every entry here documents a shortcut a page already wires with its own `useHotkeys`/`@keydown`
 * (POS's own F1 sheet already covers POS itself; this registry surfaces the rest: journal grid,
 * journal list, invoice desk form).
 */

export interface ShortcutEntry {
  keys: string;
  label: string;
}

export const ROUTE_SHORTCUTS: Record<string, ShortcutEntry[]> = {
  journal: [
    { keys: 'J / K', label: 'التنقل بين القيود' },
    { keys: 'Enter', label: 'فتح القيد المحدد' },
    { keys: 'N', label: 'قيد يدوي جديد' },
  ],
  'journal-new': [
    { keys: 'Enter', label: 'الانتقال للخلية التالية (يضيف سطراً جديداً في آخر سطر)' },
    { keys: '=', label: 'في خلية مدين/دائن: توازن تلقائي بالفرق المتبقي' },
    { keys: 'Ctrl+D', label: 'تكرار السطر الحالي' },
    { keys: 'لصق من Excel', label: 'الصق عدة خلايا مفصولة بـ Tab في أي خلية بالجدول' },
    { keys: 'Ctrl+Enter', label: 'ترحيل القيد' },
    { keys: 'Ctrl+S', label: 'حفظ كمسودة' },
  ],
  'invoice-new': [
    { keys: 'Ctrl+Enter', label: 'إضافة سطر جديد' },
    { keys: 'Ctrl+D', label: 'تكرار السطر الحالي' },
    { keys: 'Ctrl+Delete', label: 'حذف السطر الحالي' },
  ],
  'quotation-new': [
    { keys: 'Ctrl+Enter', label: 'إضافة سطر جديد' },
    { keys: 'Ctrl+D', label: 'تكرار السطر الحالي' },
    { keys: 'Ctrl+Delete', label: 'حذف السطر الحالي' },
  ],
};

/** Global shortcuts that apply everywhere, shown at the bottom of the sheet regardless of page. */
export const GLOBAL_SHORTCUTS: ShortcutEntry[] = [
  { keys: 'Ctrl+K', label: 'فتح لوحة الأوامر (بحث أو تنفيذ أمر)' },
  { keys: 'F1 / ?', label: 'عرض اختصارات لوحة المفاتيح لهذه الصفحة' },
];
