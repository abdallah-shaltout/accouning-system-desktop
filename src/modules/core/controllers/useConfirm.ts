import { reactive } from 'vue';

export interface ConfirmOptions {
  title: string;
  message?: string;
  confirmText?: string;
  cancelText?: string;
  /** Destructive actions get a filled danger button (design_system.md → Danger button). */
  danger?: boolean;
}

interface ConfirmState extends ConfirmOptions {
  open: boolean;
  resolve?: (value: boolean) => void;
}

/** Single global confirm dialog, rendered once by <ConfirmDialog /> in App.vue. */
export const confirmState = reactive<ConfirmState>({ open: false, title: '' });

export function useConfirm() {
  return (options: ConfirmOptions): Promise<boolean> =>
    new Promise((resolve) => {
      confirmState.resolve?.(false);
      Object.assign(confirmState, { confirmText: undefined, cancelText: undefined, message: undefined, danger: false }, options, {
        open: true,
        resolve,
      });
    });
}

export function settleConfirm(value: boolean) {
  const resolve = confirmState.resolve;
  confirmState.open = false;
  confirmState.resolve = undefined;
  resolve?.(value);
}
