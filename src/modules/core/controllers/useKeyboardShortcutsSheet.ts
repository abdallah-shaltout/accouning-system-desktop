import { ref } from 'vue';

/**
 * Shared open state for the global keyboard-shortcuts sheet (`layout/KeyboardShortcutsSheet.vue`,
 * mounted once in `DefaultLayout.vue`), so other UI — `NavUser`'s dropdown item, docs/v2/17 Phase D —
 * can open it without owning its own copy of the state. Module-level ref, same lightweight pattern
 * as `useCommandPalette.ts`'s registries.
 */
export const keyboardShortcutsOpen = ref(false);

export function useKeyboardShortcutsSheet() {
  return {
    open: keyboardShortcutsOpen,
    show: () => (keyboardShortcutsOpen.value = true),
    toggle: () => (keyboardShortcutsOpen.value = !keyboardShortcutsOpen.value),
  };
}
