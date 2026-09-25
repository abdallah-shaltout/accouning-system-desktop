import type { ToastAction } from './useNotificationStore';
import { useNotificationStore } from './useNotificationStore';

export function errorMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === 'string') return err;
  return 'حدث خطأ غير متوقع';
}

/** Thin, ergonomic wrapper over the notification store. */
export function useToast() {
  const store = useNotificationStore();
  return {
    success: (title: string, message?: string) => store.addNotification({ type: 'success', title, message }),
    info: (title: string, message?: string) => store.addNotification({ type: 'info', title, message }),
    warning: (title: string, message?: string) => store.addNotification({ type: 'warning', title, message }),
    error: (err: unknown, title = 'تعذر إتمام العملية') =>
      store.addNotification({ type: 'error', title, message: errorMessage(err) }),
    /**
     * Phase 14: an error toast with inline actions (e.g. "إعادة الطباعة" +
     * a PDF fallback offer) and no auto-dismiss timeout, since the user
     * needs time to notice and click an action rather than have it vanish.
     */
    errorWithActions: (title: string, message: string | undefined, actions: ToastAction[]) =>
      store.addNotification({ type: 'error', title, message, actions, duration: 0 }),
    /** Phase 17 Phase C: a success toast with inline actions (e.g. "فتح المجلد" after a save). */
    successWithActions: (title: string, message: string | undefined, actions: ToastAction[]) =>
      store.addNotification({ type: 'success', title, message, actions }),
  };
}
