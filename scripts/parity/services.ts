/**
 * Loads every `src/modules/*\/services/*.ts` so `defineService.ts`'s registry is complete before a
 * bundle replay looks an action's `source` up in it (B-10). Same walk as `scripts/verify/replay.ts`'s
 * `importAllServices` — that script runs its CLI at import time, so it can't be imported from here.
 */
import { readdirSync } from 'node:fs';
import { join } from 'node:path';

let loaded = false;

export async function importAllServices(): Promise<void> {
  if (loaded) return;
  loaded = true;
  const modulesDir = join(import.meta.dirname, '../../src/modules');
  for (const moduleName of readdirSync(modulesDir, { withFileTypes: true })) {
    if (!moduleName.isDirectory()) continue;
    const servicesDir = join(modulesDir, moduleName.name, 'services');
    let files: string[];
    try {
      files = readdirSync(servicesDir).filter((f) => f.endsWith('.ts'));
    } catch {
      continue; // module has no services/ dir
    }
    for (const file of files) await import(join(servicesDir, file));
  }
}
