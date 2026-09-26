import { computed, ref } from 'vue';
import type { Component } from 'vue';
import type { AppRoute } from '@/modules/core/types/route';
import { AlertTriangle, ShieldAlert, Truck } from '@lucide/vue';
import { db } from '@/mocks/db';
import { on } from '@/mocks/events';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { useInsights } from './useInsights';
import type { Insight } from '../services/insightTypes';

/**
 * Notifications drawer (docs/v2/14-platform.md §6 "Bell drawer: insights (by severity) + events").
 * Composes the insight engine (which already covers "a recurring entry is due" — see
 * `recurringJournalDueRule`/`recurringExpenseDueRule` in `insightRules.ts`) with three event kinds
 * that aren't insight-engine rules because they're one-shot occurrences tied to a specific record,
 * not recomputed conditions: a stock transfer arriving at this user's branch, an approval request
 * waiting on this manager, and the last automatic backup having failed.
 */

export type NotificationSeverity = Insight['severity'];

export interface AppNotification {
  id: string;
  severity: NotificationSeverity;
  message: string;
  actionLabel: string;
  actionTo: AppRoute;
  icon: Component;
  createdAt: string;
}

const READ_KEY = 'app_notifications_read';

function readStorageKey(userId: string | undefined): string {
  return `${READ_KEY}:${userId ?? 'anon'}`;
}

function loadRead(userId: string | undefined): Set<string> {
  try {
    const raw = localStorage.getItem(readStorageKey(userId));
    return new Set(raw ? (JSON.parse(raw) as string[]) : []);
  } catch {
    return new Set();
  }
}

function saveRead(userId: string | undefined, ids: Set<string>) {
  try {
    localStorage.setItem(readStorageKey(userId), JSON.stringify([...ids]));
  } catch {
    /* ignore */
  }
}

function insightToNotification(i: Insight): AppNotification {
  return { id: i.id, severity: i.severity, message: i.message, actionLabel: i.actionLabel, actionTo: i.actionTo, icon: i.icon, createdAt: i.createdAt };
}

/** Stock transfers `SENT` (in transit) toward a branch this user can see — "a transfer arrived" per the doc (shown as pending-receipt, since that's the actionable moment). */
function transferEvents(homeBranch: string | undefined): AppNotification[] {
  return db.stockTransfers
    .filter((t) => t.status === 'SENT' && (!homeBranch || t.toBranchId === homeBranch))
    .map((t) => ({
      id: `transfer:${t.id}`,
      severity: 'info' as const,
      message: `تحويل مخزون ${t.number} في الطريق — بانتظار الاستلام`,
      actionLabel: 'استلام التحويل',
      actionTo: { name: 'transfers', query: { highlight: t.id } },
      icon: Truck,
      createdAt: t.sentAt ?? t.date,
    }));
}

/** Pending async approval requests (docs/v2/14-platform.md §6 "An approval is requested") — managers/admins only. */
function approvalEvents(canApprove: boolean): AppNotification[] {
  if (!canApprove) return [];
  return db.approvalRequests
    .filter((r) => r.status === 'pending')
    .map((r) => ({
      id: `approval:${r.id}`,
      severity: 'warning' as const,
      message: `طلب اعتماد بانتظار المراجعة: ${r.summary}`,
      actionLabel: 'مراجعة الطلب',
      actionTo: { name: 'approvals' },
      icon: ShieldAlert,
      createdAt: r.requestedAt,
    }));
}

/** "An automatic backup failed" (docs/v2/14-platform.md §6) — distinct from the `backup-overdue` insight. */
function backupFailedEvent(canSeeBackup: boolean): AppNotification[] {
  const failedAt = db.settings.backup?.lastBackupFailedAt;
  if (!canSeeBackup || !failedAt) return [];
  return [
    {
      id: `backup-failed:${failedAt}`,
      severity: 'critical',
      message: 'فشلت آخر محاولة نسخ احتياطي تلقائي',
      actionLabel: 'فتح إعدادات النسخ الاحتياطي',
      actionTo: { name: 'settings-backup' },
      icon: AlertTriangle,
      createdAt: failedAt,
    },
  ];
}

const SEVERITY_ORDER: Record<NotificationSeverity, number> = { critical: 0, warning: 1, info: 2, positive: 3 };

export function useNotifications() {
  const auth = useAuthStore();
  const refreshToken = ref(0);
  on('ledger:changed', () => refreshToken.value++);
  on('catalog:changed', () => refreshToken.value++);

  const { insights } = useInsights({ limit: 100 });

  const all = computed<AppNotification[]>(() => {
    void refreshToken.value;
    const canApprove = auth.can('approvals', 'write');
    const canSeeBackup = auth.can('settings', 'write');
    const list = [
      ...insights.value.filter((i) => i.severity !== 'positive').map(insightToNotification),
      ...transferEvents(auth.user?.homeBranch),
      ...approvalEvents(canApprove),
      ...backupFailedEvent(canSeeBackup),
    ];
    return list.sort((a, b) => SEVERITY_ORDER[a.severity] - SEVERITY_ORDER[b.severity] || b.createdAt.localeCompare(a.createdAt));
  });

  const readIds = ref(loadRead(auth.user?.id));
  const unread = computed(() => all.value.filter((n) => !readIds.value.has(n.id)));
  const unreadCount = computed(() => unread.value.length);

  function markRead(id: string) {
    readIds.value.add(id);
    saveRead(auth.user?.id, readIds.value);
  }
  function markAllRead() {
    for (const n of all.value) readIds.value.add(n.id);
    saveRead(auth.user?.id, readIds.value);
  }
  function isRead(id: string): boolean {
    return readIds.value.has(id);
  }

  return { all, unread, unreadCount, markRead, markAllRead, isRead };
}
