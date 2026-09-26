/** Maps a repo path to its architectural location (area → module → layer). */
import type { Config } from '../config';
import type { Location } from '../types';

export function locate(file: string, config: Config): Location {
  const { modules, mocks, rust, docs, plans } = config.paths;
  if (file.startsWith(modules + '/')) {
    const [module, second, ...rest] = file.slice(modules.length + 1).split('/');
    // `routes.ts` / `commands.ts` at module root count as their own layer.
    const layer = rest.length ? second : second.replace(/\.(ts|vue)$/, '');
    return { area: 'module', module, layer };
  }
  if (file.startsWith(mocks + '/')) return { area: 'mocks' };
  if (file.startsWith(rust + '/')) return { area: 'rust' };
  if (file.startsWith(docs + '/') || file.startsWith(plans + '/')) return { area: 'docs' };
  return { area: 'app' };
}

/** Graph node name: a module, `mocks`, or `app` (router, main.ts, App.vue). */
export function nodeOf(file: string, config: Config): string {
  const loc = locate(file, config);
  return loc.area === 'module' ? loc.module! : loc.area;
}

export const stem = (file: string) => file.split('/').pop()!.replace(/\.[^.]+$/, '');
