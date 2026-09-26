/**
 * Analyze stage: follows each service function's calls transitively (through services, helpers and
 * the mock backend), then suggests what the Rust backend does with it. The suggestion is a
 * heuristic — `config.overrides` (with a reason) is where Part 01 reviewers record the final call.
 */
import { config } from './config';
import type { Extracted } from './extract';
import type { Closure, Disposition, FnFacts, Inventory, ServiceFn } from './types';

const snake = (s: string) => s.replace(/([a-z0-9])([A-Z])/g, '$1_$2').toLowerCase();

function closureOf(startId: string, facts: Map<string, FnFacts>): Closure {
  const seen = new Set<string>();
  const stack = [startId];
  const acc = { tables: new Set<string>(), writes: new Set<string>(), mockFns: new Set<string>(), mockRefs: new Set<string>(), invokes: new Set<string>(), platform: new Set<string>(), capabilities: new Set<string>() };
  while (stack.length) {
    const id = stack.pop()!;
    if (seen.has(id)) continue;
    seen.add(id);
    const f = facts.get(id);
    if (!f) continue;
    f.tables.forEach((t) => acc.tables.add(t));
    f.writes.forEach((t) => acc.writes.add(t));
    f.mockRefs.forEach((t) => acc.mockRefs.add(t));
    f.invokes.forEach((t) => acc.invokes.add(t));
    f.platform.forEach((t) => acc.platform.add(t));
    if (id !== startId && f.file.startsWith(config.mocksDir + '/')) acc.mockFns.add(id);
    for (const [cap, ids] of Object.entries(config.capabilities)) if (ids.includes(id)) acc.capabilities.add(cap);
    stack.push(...f.calls);
  }
  const sorted = (s: Set<string>) => [...s].sort();
  return {
    tables: sorted(acc.tables), writes: sorted(acc.writes), mockFns: sorted(acc.mockFns), mockRefs: sorted(acc.mockRefs),
    invokes: sorted(acc.invokes), platform: sorted(acc.platform), capabilities: sorted(acc.capabilities),
  };
}

function suggest(c: Closure): { disposition: Disposition; reason: string } {
  if (c.tables.length || c.mockFns.length) return { disposition: 'port', reason: c.writes.length ? 'reads and writes backend data' : 'reads backend data' };
  if (c.mockRefs.length) return { disposition: 'port', reason: `uses mock state (${c.mockRefs.join(', ')})` };
  if (c.invokes.length) return { disposition: 'rust-existing', reason: `already calls Rust (${c.invokes.join(', ')})` };
  if (c.platform.length) return { disposition: 'frontend', reason: `webview/plugin only (${c.platform.join(', ')})` };
  return { disposition: 'frontend', reason: 'pure computation, no data access' };
}

export function analyze(x: Extracted): Inventory {
  const typeNames = new Set(x.types.map((t) => t.name));
  const services: ServiceFn[] = x.wrapped.map((w) => {
    const [module, fnName] = w.source.split('.');
    const closure = closureOf(w.fnId, x.facts);
    const override = config.overrides[w.source];
    const { disposition, reason } = override ?? suggest(closure);
    const words = `${w.params.map((p) => p.type).join(' ')} ${w.returns}`.match(/[A-Za-z_][A-Za-z0-9_]*/g) ?? [];
    return {
      source: w.source,
      module,
      service: w.file.split('/').pop()!.replace(/\.ts$/, ''),
      name: fnName,
      file: w.file,
      line: w.line,
      params: w.params,
      returns: w.returns,
      dtoTypes: [...new Set(words.filter((t) => typeNames.has(t)))].sort(),
      serviceCalls: [...new Set(w.calls.map((c) => x.wrappedByFnId.get(c)).filter((s): s is string => !!s && s !== w.source))].sort(),
      closure,
      disposition,
      dispositionReason: override ? `override: ${reason}` : reason,
      rustCommand: disposition === 'port' ? `${module}_${snake(fnName)}` : undefined,
    };
  });
  services.sort((a, b) => a.source.localeCompare(b.source));

  const mockFns = [...x.facts.values()]
    .filter((f) => f.file.startsWith(config.mockBackendDir + '/') || f.file === config.mockDbFile)
    .sort((a, b) => a.id.localeCompare(b.id));

  return {
    services,
    unwrapped: x.unwrapped.sort((a, b) => `${a.file}${a.name}`.localeCompare(`${b.file}${b.name}`)),
    mockFns,
    types: x.types.sort((a, b) => `${a.module}.${a.name}`.localeCompare(`${b.module}.${b.name}`)),
    pathLinks: x.pathLinks.sort((a, b) => `${a.file}:${String(a.line).padStart(5, '0')}`.localeCompare(`${b.file}:${String(b.line).padStart(5, '0')}`)),
    tables: x.tables,
  };
}
