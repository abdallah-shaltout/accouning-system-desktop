import { defineStore } from 'pinia';
import { ref } from 'vue';

export interface ToastNotification {
  id: string;
  type: 'success' | 'error' | 'info' | 'warning';
  title: string;
  message?: string;
  /** ms; 0 = sticky. */
  duration?: number;
}

export const useNotificationStore = defineStore('notification', () => {
  const notifications = ref<ToastNotification[]>([]);
  let seq = 0;

  function addNotification(notification: Omit<ToastNotification, 'id'>) {
    const id = `t${++seq}`;
    notifications.value.push({ ...notification, id });
    // Keep the stack short.
    if (notifications.value.length > 4) notifications.value.shift();
    const duration = notification.duration ?? (notification.type === 'error' ? 6000 : 3500);
    if (duration !== 0) setTimeout(() => removeNotification(id), duration);
    return id;
  }

  function removeNotification(id: string) {
    notifications.value = notifications.value.filter((n) => n.id !== id);
  }

  return { notifications, addNotification, removeNotification };
});
