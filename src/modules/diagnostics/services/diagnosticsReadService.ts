/**
 * Aggregation for the `/dev/diagnostics` page (18.B5): groups raw `error`-channel entries by
 * fingerprint (count, first/last seen) and reduces `perf`-channel entries into p50/p95 per
 * source, plus long tasks and budget breaches. Pure functions over already-read `LogEntry[]` —
 * reading the entries themselves goes through `diagnosticsService.readLogs`/`exportAll`.
 */
import { readLogs } from './diagnosticsService';
import { PERF_BUDGET_MS } from '../config';
import type { FingerprintGroup, LogEntry, PerfStat } from '../types';

function today(): string {
  return new Date().toISOString().slice(0, 10);
}

const EPOCH = '2000-01-01';

export async function loadChannel(channel: LogEntry['channel'], days = 30): Promise<LogEntry[]> {
  const from = new Date(Date.now() - days * 86_400_000).toISOString().slice(0, 10);
  return readLogs(channel, from < EPOCH ? EPOCH : from, today());
}

export function groupByFingerprint(entries: LogEntry[]): FingerprintGroup[] {
  const groups = new Map<string, FingerprintGroup>();
  for (const e of entries) {
    if (!e.err) continue;
    const fp = e.err.fingerprint;
    const existing = groups.get(fp);
    if (existing) {
      existing.count += 1;
      if (e.ts < existing.firstSeen) existing.firstSeen = e.ts;
      if (e.ts > existing.lastSeen) {
        existing.lastSeen = e.ts;
        existing.sample = e;
      }
    } else {
      groups.set(fp, {
        fingerprint: fp,
        name: e.err.name,
        message: e.err.message,
        source: e.source,
        count: 1,
        firstSeen: e.ts,
        lastSeen: e.ts,
        sample: e,
      });
    }
  }
  return [...groups.values()].sort((a, b) => b.count - a.count || b.lastSeen.localeCompare(a.lastSeen));
}

function percentile(sorted: number[], p: number): number {
  if (!sorted.length) return 0;
  const idx = Math.min(sorted.length - 1, Math.floor((p / 100) * sorted.length));
  return sorted[idx];
}

function budgetFor(kind: PerfStat['kind']): number | undefined {
  switch (kind) {
    case 'service':
      return PERF_BUDGET_MS.service;
    case 'route':
      return PERF_BUDGET_MS.route;
    case 'longtask':
      return PERF_BUDGET_MS.longTask;
    case 'startup':
      return PERF_BUDGET_MS.startup;
    default:
      return undefined;
  }
}

function kindOf(entry: LogEntry): PerfStat['kind'] {
  const data = entry.data as { kind?: PerfStat['kind'] } | undefined;
  if (data?.kind) return data.kind;
  if (entry.source === 'startup') return 'startup';
  if (entry.source.startsWith('route.')) return 'route';
  if (entry.source === 'longtask') return 'longtask';
  if (entry.source === 'snapshot') return 'snapshot';
  return 'service';
}

export function computePerfStats(entries: LogEntry[]): PerfStat[] {
  const bySource = new Map<string, { kind: PerfStat['kind']; durations: number[] }>();
  for (const e of entries) {
    const data = e.data as { durationMs?: number } | undefined;
    const durationMs = typeof data?.durationMs === 'number' ? data.durationMs : undefined;
    if (durationMs === undefined) continue;
    const kind = kindOf(e);
    const bucket = bySource.get(e.source) ?? { kind, durations: [] };
    bucket.durations.push(durationMs);
    bySource.set(e.source, bucket);
  }
  const stats: PerfStat[] = [];
  for (const [source, { kind, durations }] of bySource) {
    const sorted = [...durations].sort((a, b) => a - b);
    const budgetMs = budgetFor(kind);
    stats.push({
      source,
      kind,
      count: sorted.length,
      p50: percentile(sorted, 50),
      p95: percentile(sorted, 95),
      max: sorted.at(-1) ?? 0,
      budgetMs,
      breaches: budgetMs ? sorted.filter((d) => d > budgetMs).length : 0,
    });
  }
  return stats.sort((a, b) => b.p95 - a.p95);
}

export function slowestLongTasks(entries: LogEntry[], limit = 20): LogEntry[] {
  return entries
    .filter((e) => kindOf(e) === 'longtask')
    .sort((a, b) => {
      const da = (a.data as { durationMs?: number } | undefined)?.durationMs ?? 0;
      const db_ = (b.data as { durationMs?: number } | undefined)?.durationMs ?? 0;
      return db_ - da;
    })
    .slice(0, limit);
}
