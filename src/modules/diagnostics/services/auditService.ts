/**
 * Read-side for the business audit trail (18.B4). The backend (`mocks/backend/core.ts`'s
 * `logAudit`/`logActivity`) is the only writer — this file only reads and filters `db.audit`, kept
 * seam-safe like every other service (pages never import `src/mocks/*` directly).
 */
import { clone, db, delay } from '@/mocks';
import type { AuditAction, AuditEntry } from '../types';
import { wrap } from './defineService';

export interface AuditFilter {
  userId?: string;
  entity?: string;
  action?: AuditAction;
  from?: string;
  to?: string;
  search?: string;
}

function matches(entry: AuditEntry, filter: AuditFilter): boolean {
  if (filter.userId && entry.userId !== filter.userId) return false;
  if (filter.entity && entry.entity !== filter.entity) return false;
  if (filter.action && entry.action !== filter.action) return false;
  if (filter.from && entry.at.slice(0, 10) < filter.from) return false;
  if (filter.to && entry.at.slice(0, 10) > filter.to) return false;
  if (filter.search) {
    const q = filter.search.trim().toLowerCase();
    if (q && !entry.message.toLowerCase().includes(q) && !entry.entityId.toLowerCase().includes(q) && !(entry.entityLabel ?? '').toLowerCase().includes(q)) {
      return false;
    }
  }
  return true;
}

export const getAuditEntries = wrap('diagnostics.getAuditEntries', async function getAuditEntries(filter: AuditFilter = {}): Promise<AuditEntry[]> {
  await delay(80);
  const all = clone(db.audit ?? []);
  return all.filter((e) => matches(e, filter)).sort((a, b) => b.at.localeCompare(a.at));
});

/** Distinct entity kinds seen so far, for the filter dropdown — avoids a hard-coded list that
 * would drift from what the backend actually writes. */
export const getAuditEntities = wrap('diagnostics.getAuditEntities', async function getAuditEntities(): Promise<string[]> {
  const all = db.audit ?? [];
  return [...new Set(all.map((e) => e.entity))].sort();
});

/** Alias kept for call sites written against either name. */
export const getAuditEntityKinds = getAuditEntities;
