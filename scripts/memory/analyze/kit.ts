/** Analyzer — the shared UI kit and core building blocks an agent must reuse before writing new ones. */
import type { Config } from '../config';
import type { ParsedRepo } from '../types';
import { stem } from './classify';

/** Shared-kit groups: label → directory under src/modules/core. */
const GROUPS: [string, string][] = [
  ['App*/ui components', 'components/ui'],
  ['Page blocks', 'components/blocks'],
  ['Page layouts', 'components/layouts'],
  ['App shell', 'components/layout'],
  ['Core controllers (composables/stores)', 'controllers'],
  ['Core helpers', 'helpers'],
  ['Core services', 'services'],
];

export function analyzeKit(repo: ParsedRepo, config: Config): Record<string, string[]> {
  const core = config.paths.coreKit.replace(/\/components$/, '');
  const kit: Record<string, string[]> = {};
  for (const [label, dir] of GROUPS) {
    const prefix = `${core}/${dir}/`;
    const names = repo.files.filter((f) => f.path.startsWith(prefix) && !f.path.slice(prefix.length).includes('/')).map((f) => stem(f.path));
    if (names.length) kit[label] = [...new Set(names)].sort();
  }
  const shadcnPrefix = `${config.paths.coreKit}/shadcn/`;
  const primitives = new Set(
    repo.files.filter((f) => f.path.startsWith(shadcnPrefix)).map((f) => f.path.slice(shadcnPrefix.length).split('/')[0]),
  );
  if (primitives.size) kit['shadcn primitives'] = [...primitives].sort();
  return kit;
}
