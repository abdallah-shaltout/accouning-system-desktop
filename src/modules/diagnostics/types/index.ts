/**
 * Diagnostics types (18.B1) — one `LogEntry` schema shared by all 5 channels. Everything reads
 * and writes this shape: `logService` (in-process API), the Rust `diag_*` commands (opaque JSON
 * lines on disk), the IndexedDB ring buffer, and the dev pages.
 */

export type LogChannel = 'error' | 'perf' | 'debug' | 'audit' | 'accounting';
export type LogLevel = 'debug' | 'info' | 'warn' | 'error';

export interface LogErrorInfo {
  name: string;
  message: string;
  stack?: string;
  /** Hash of name + message (numbers/ids stripped) + first app stack frame — same bug, same fingerprint. */
  fingerprint: string;
}

export interface LogContext {
  sessionId: string;
  correlationId?: string;
  route?: string;
  userId?: string;
  branchId?: string;
  appVersion: string;
}

export interface LogEntry {
  v: 1;
  ts: string;
  channel: LogChannel;
  level: LogLevel;
  /** e.g. 'invoices.createSale', 'ui.AppPhoneInput', 'rust.render_pdf'. */
  source: string;
  msg: string;
  data?: unknown;
  err?: LogErrorInfo;
  ctx: LogContext;
}

export interface FingerprintGroup {
  fingerprint: string;
  name: string;
  message: string;
  source: string;
  count: number;
  firstSeen: string;
  lastSeen: string;
  sample: LogEntry;
}

export interface PerfStat {
  source: string;
  kind: 'service' | 'route' | 'longtask' | 'startup' | 'snapshot';
  count: number;
  p50: number;
  p95: number;
  max: number;
  budgetMs?: number;
  breaches: number;
}

/**
 * Business audit record (18.B4) — who did what to which business entity. Written by the mock
 * backend at the same points that call `logActivity()`, and by a real backend the same way.
 * Append-only: there is no update/delete API in `auditService`. Lives in `db.audit`, is included
 * in `backupArchive` and is NOT a diagnostic log — see README "Business audit ≠ diagnostic logs."
 */
export type AuditAction = 'create' | 'update' | 'post' | 'void' | 'reverse' | 'delete' | 'login' | 'settings';

/** One changed field, `before`/`after` only (never the whole document) so audit rows stay small and readable. */
export interface AuditFieldDiff {
  field: string;
  before?: unknown;
  after?: unknown;
}

export interface AuditEntry {
  id: string;
  /** Business entity kind, e.g. 'invoice', 'purchaseOrder', 'branch', 'user'. */
  entity: string;
  entityId: string;
  entityLabel?: string;
  action: AuditAction;
  before?: AuditFieldDiff[];
  after?: AuditFieldDiff[];
  userId: string;
  branchId?: string;
  at: string;
  reason?: string;
  /** The Arabic activity-feed message this record also produces, so the feed keeps reading the same. */
  message: string;
  link?: string;
}
