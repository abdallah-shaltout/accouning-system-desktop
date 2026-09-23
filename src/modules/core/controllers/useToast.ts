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
  };
}
