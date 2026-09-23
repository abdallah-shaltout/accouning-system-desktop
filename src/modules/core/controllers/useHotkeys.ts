import { onBeforeUnmount, onMounted } from 'vue';

/** Return `false` to mean "not handled" — the key then keeps its default behavior. */
type Handler = (event: KeyboardEvent) => boolean | void;

/**
 * Register page-level keyboard shortcuts, e.g. `{ F9: newSale, F12: pay, 'ctrl+Enter': pay }`.
 * Function keys fire even while typing in an input; plain keys are ignored inside form fields.
 */
export function useHotkeys(bindings: Record<string, Handler>, options: { enabled?: () => boolean } = {}) {
  function comboOf(e: KeyboardEvent): string {
    const parts: string[] = [];
    if (e.ctrlKey || e.metaKey) parts.push('ctrl');
    if (e.altKey) parts.push('alt');
    if (e.shiftKey && e.key.length > 1) parts.push('shift');
    parts.push(e.key);
    return parts.join('+');
  }

  function onKeydown(e: KeyboardEvent) {
    if (options.enabled && !options.enabled()) return;
    const handler = bindings[comboOf(e)];
    if (!handler) return;
    const target = e.target as HTMLElement | null;
    const typing = !!target && (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName));
    const isFunctionKey = /^F\d{1,2}$/.test(e.key) || e.key === 'Escape' || e.ctrlKey || e.metaKey;
    if (typing && !isFunctionKey) return;
    if (handler(e) !== false) e.preventDefault();
  }

  onMounted(() => window.addEventListener('keydown', onKeydown));
  onBeforeUnmount(() => window.removeEventListener('keydown', onKeydown));
}
