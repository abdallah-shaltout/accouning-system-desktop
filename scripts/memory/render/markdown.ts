/** Stage 4 — assemble AGENT_MEMORY.md. Section order is the reading order an agent should follow. */
import type { Config } from '../config';
import type { Analysis, ParsedRepo } from '../types';
import * as s from './sections';

const ORDER = [
  s.header, s.landmarks, s.architecture, s.domainMap, s.serviceApi, s.routes, s.dependencies,
  s.ipc, s.mocks, s.kit, s.boundaries, s.stack, s.docs,
];

export function render(repo: ParsedRepo, analysis: Analysis, config: Config): string {
  const ctx = { repo, analysis, config };
  return ORDER.map((fn) => fn(ctx)).join('\n\n') + '\n';
}
