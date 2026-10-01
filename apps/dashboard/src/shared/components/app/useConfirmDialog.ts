import { reactive } from "vue";
import { copy } from "@/shared/config/copy";

export interface ConfirmOptions {
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  destructive: boolean;
}

interface ConfirmState {
  open: boolean;
  options: ConfirmOptions;
}

/** Single shared instance — one `<ConfirmDialog />` mounted at the app root serves every caller. */
export const confirmState: ConfirmState = reactive({
  open: false,
  options: {
    title: copy.confirmDelete.title,
    message: copy.confirmDelete.message,
    confirmLabel: copy.confirmDelete.confirm,
    cancelLabel: copy.confirmDelete.cancel,
    destructive: true,
  },
});

let pendingResolve: ((value: boolean) => void) | null = null;

export function resolveConfirm(value: boolean): void {
  confirmState.open = false;
  pendingResolve?.(value);
  pendingResolve = null;
}

/**
 * `const { confirm } = useConfirmDialog(); if (await confirm({ title, message })) { ... }`
 * Resolves `true` on confirm, `false` on cancel or dismiss. Defaults to the shared "confirm delete"
 * copy so a plain `confirm()` call already reads correctly for the common destructive-action case.
 */
export function useConfirmDialog() {
  function confirm(options: Partial<ConfirmOptions> = {}): Promise<boolean> {
    confirmState.options = {
      title: options.title ?? copy.confirmDelete.title,
      message: options.message ?? copy.confirmDelete.message,
      confirmLabel: options.confirmLabel ?? copy.confirmDelete.confirm,
      cancelLabel: options.cancelLabel ?? copy.confirmDelete.cancel,
      destructive: options.destructive ?? true,
    };
    confirmState.open = true;
    return new Promise<boolean>((resolve) => {
      pendingResolve = resolve;
    });
  }

  return { confirm };
}
