import { nextTick, type Ref } from 'vue';

const FOCUSABLE = 'input:not([type=hidden]), select, textarea, button';

interface Options {
  /** Element holding the line rows — a `<tbody>`, or a wrapper `<div>` (then pass `rowSelector`). */
  container: Ref<HTMLElement | undefined>;
  /** Selector for the rows, relative to `container`. */
  rowSelector?: string;
  /** Appends a blank line. Omit for grids with a fixed set of rows. */
  addRow?: () => void;
  /** Whether line `index` has content. Tab off an empty last line leaves the grid instead of adding another. */
  isFilled?: (index: number) => boolean;
}

/**
 * Spreadsheet-style Tab for line-item grids: Tab off a
 * row's last field jumps to the next row's first field (skipping the row's action buttons), and off
 * the last row adds a new line — so a whole document can be keyed in without the mouse. Mark action
 * buttons (delete, duplicate…) with `data-grid-skip`. Bind the returned handler to the container's
 * `@keydown`.
 */
export function useGridTab({ container, rowSelector = ':scope > tr', addRow, isFilled = () => true }: Options) {
  const rows = () => Array.from(container.value?.querySelectorAll<HTMLElement>(rowSelector) ?? []);
  const fields = (row: HTMLElement) =>
    Array.from(row.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
      (el) => !el.closest('[data-grid-skip]') && !(el as HTMLInputElement).disabled && el.getClientRects().length > 0,
    );

  /** Focus a row's first field; on an empty row an AppCombobox trigger is opened so the user can type straight away. */
  function focusRow(row: HTMLElement | undefined, empty: boolean) {
    const el = row && fields(row)[0];
    if (!el) return;
    if (empty && el.getAttribute('aria-haspopup') === 'listbox') {
      el.click();
      return;
    }
    el.focus();
    if (el instanceof HTMLInputElement) el.select();
  }

  return function onGridKeydown(e: KeyboardEvent) {
    if (e.key !== 'Tab' || e.shiftKey || e.ctrlKey || e.altKey || e.metaKey) return;
    const target = e.target as HTMLElement;
    const all = rows();
    const index = all.findIndex((r) => r.contains(target));
    if (index < 0 || fields(all[index]).at(-1) !== target) return;
    if (index < all.length - 1) {
      e.preventDefault();
      focusRow(all[index + 1], !isFilled(index + 1));
    } else if (addRow && isFilled(index)) {
      e.preventDefault();
      addRow();
      nextTick(() => focusRow(rows()[index + 1], true));
    }
  };
}
