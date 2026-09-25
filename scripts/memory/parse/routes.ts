/** Parser — route records declared in `src/modules/<m>/routes{.ts,/index.ts}`. */
import type { RouteInfo, SourceFile } from '../types';

const ROUTE_FILE = /^src\/modules\/[^/]+\/routes(\.ts|\/[^/]+\.ts)$/;
/** A record starts at `{ path:` — but not `({ path:` (redirect targets / location objects). */
const RECORD = /(\(\s*)?\{\s*path:\s*['"]([^'"]*)['"]/g;
const LAZY_PAGE = /import\(\s*['"]([^'"]+)['"]\s*\)/;

const field = (text: string, key: string) => text.match(new RegExp(`\\b${key}:\\s*['"]([^'"]+)['"]`))?.[1];
const pageName = (spec?: string) => spec?.split('/').pop();

/** File-level helpers: `const list = () => import('../pages/X.vue')` and `const meta = (title) => ({ area: 'x' })`. */
function localDefinitions(content: string) {
  const pages = new Map<string, string>();
  const metaAreas = new Map<string, string | undefined>();
  for (const m of content.matchAll(/^const\s+(\w+)\s*=\s*(.+)$/gm)) {
    const page = m[2].match(LAZY_PAGE)?.[1];
    if (page) pages.set(m[1], page);
    else if (/=>\s*\(\{/.test(m[2])) metaAreas.set(m[1], field(m[2], 'area'));
  }
  return { pages, metaAreas };
}

export function parseRoutes(files: SourceFile[]): RouteInfo[] {
  const routes: RouteInfo[] = [];
  for (const file of files.filter((f) => ROUTE_FILE.test(f.path))) {
    const { pages, metaAreas } = localDefinitions(file.content);
    const starts = [...file.content.matchAll(RECORD)].filter((m) => !m[1]);
    starts.forEach((m, i) => {
      const segment = file.content.slice(m.index, starts[i + 1]?.index ?? file.content.length);
      const componentRef = segment.match(/\bcomponent:\s*(\w+)\s*[,}]/)?.[1];
      const metaCall = segment.match(/\bmeta:\s*(\w+)\(\s*['"]([^'"]+)['"]/);
      const redirect = /\bredirect:/.test(segment);
      routes.push({
        file: file.path,
        path: m[2],
        name: field(segment, 'name'),
        title: field(segment, 'title') ?? metaCall?.[2],
        area: field(segment, 'area') ?? (metaCall ? metaAreas.get(metaCall[1]) : undefined),
        component: redirect
          ? '(redirect)'
          : pageName(segment.match(LAZY_PAGE)?.[1] ?? (componentRef ? pages.get(componentRef) : undefined)),
      });
    });
  }
  return routes;
}
