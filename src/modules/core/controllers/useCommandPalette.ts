import { computed, ref, shallowRef } from 'vue';
import { useRoute } from 'vue-router';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { findByCode } from '@/modules/products/services/productService';
import { normalizeArabic } from '../helpers/search';
import {
  PREFIX_GROUPS,
  type PaletteCommand,
  type PalettePermission,
  type PalettePrefix,
  type PaletteResult,
  type PaletteSearchProvider,
} from '../types/commandPalette';

/**
 * Command palette registry (docs/v2/14-platform.md §2). Modules register their `commands.ts`
 * exports here (typically from their routes file or a plugin-install step); this Phase 0 track
 * also registers a couple of example providers itself (see `commandPalette/exampleProviders.ts`)
 * to prove the pattern until each module adds its own.
 */

const commands: PaletteCommand[] = [];
const providers: PaletteSearchProvider[] = [];

export function registerCommands(list: PaletteCommand[]): void {
  commands.push(...list);
}

export function registerSearchProviders(list: PaletteSearchProvider[]): void {
  providers.push(...list);
}

const RECENTS_KEY = 'app_palette_recents';
const RECENTS_MAX = 8;

function recentsStorageKey(userId: string | undefined): string {
  return `${RECENTS_KEY}:${userId ?? 'anon'}`;
}

function loadRecents(userId: string | undefined): string[] {
  try {
    const raw = localStorage.getItem(recentsStorageKey(userId));
    return raw ? (JSON.parse(raw) as string[]) : [];
  } catch {
    return [];
  }
}

function saveRecents(userId: string | undefined, ids: string[]): void {
  try {
    localStorage.setItem(recentsStorageKey(userId), JSON.stringify(ids.slice(0, RECENTS_MAX)));
  } catch {
    /* ignore */
  }
}

function detectPrefix(raw: string): { prefix: PalettePrefix | null; rest: string } {
  const prefix = (['>', '@', '#', '$', '?'] as PalettePrefix[]).find((p) => raw.startsWith(p));
  return prefix ? { prefix, rest: raw.slice(1) } : { prefix: null, rest: raw };
}

/** An 8–14 digit string is looked up as a barcode first (docs/v2/14-platform.md §2). */
function looksLikeBarcode(q: string): boolean {
  return /^\d{8,14}$/.test(q.trim());
}

/**
 * Ranking: exact number match > prefix > word-prefix > contains; recent items are boosted.
 * Lower score = better; sorted ascending. `undefined` means "does not match".
 */
function rank(result: PaletteResult, query: string, isRecent: boolean): number | undefined {
  if (!query) return isRecent ? 0 : 1;
  const q = normalizeArabic(query);
  const title = normalizeArabic(result.title);
  const sub = normalizeArabic(result.subtitle);
  const keywords = normalizeArabic(result.keywords);

  let score: number | undefined;
  if (title === q || sub === q) score = 10;
  else if (title.startsWith(q) || sub.startsWith(q)) score = 20;
  else if (new RegExp(`(^|\\s)${q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`).test(title) || new RegExp(`(^|\\s)${q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`).test(sub)) {
    score = 30;
  } else if (title.includes(q) || sub.includes(q) || keywords.includes(q)) score = 40;

  if (score === undefined) return undefined;
  return isRecent ? score - 5 : score;
}

// Module-level (singleton) state: the palette is one global overlay, so every caller (the topbar
// pill, the Ctrl+K hotkey handler, the panel component itself) must share the same open/query/results
// state rather than each getting its own via a fresh composable call.
const open = ref(false);
const query = ref('');
const loading = ref(false);
const results = shallowRef<PaletteResult[]>([]);
const activeIndex = ref(0);

let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let abortController: AbortController | null = null;
let requestId = 0;
const recents = ref<string[]>([]);

export function useCommandPalette() {
  const auth = useAuthStore();
  const route = useRoute();

  function hasPermission(permission?: PalettePermission): boolean {
    if (!permission) return true;
    return auth.can(permission.area, permission.access ?? 'read');
  }

  function show() {
    open.value = true;
    query.value = '';
    activeIndex.value = 0;
    recents.value = loadRecents(auth.user?.id);
    void runSearch();
  }

  function hide() {
    open.value = false;
    abortController?.abort();
  }

  function toggle() {
    if (open.value) hide();
    else show();
  }

  function recordRecent(id: string) {
    const next = [id, ...recents.value.filter((x) => x !== id)].slice(0, RECENTS_MAX);
    recents.value = next;
    saveRecents(auth.user?.id, next);
  }

  function choose(result: PaletteResult): { to?: PaletteResult['to'] } {
    recordRecent(result.id);
    result.run?.();
    hide();
    return { to: result.to };
  }

  async function runSearch() {
    const id = ++requestId;
    abortController?.abort();
    const controller = new AbortController();
    abortController = controller;

    const { prefix, rest } = detectPrefix(query.value.trim());
    const allowedGroups = prefix ? PREFIX_GROUPS[prefix] : null;
    const q = rest.trim();

    // Empty query: show recents (resolved against commands + nothing else — providers aren't re-queried for recents).
    if (!q && !prefix) {
      const recentResults = recents.value
        .map((rid) => commands.find((c) => c.id === rid))
        .filter((c): c is PaletteCommand => !!c && hasPermission(c.permission) && (!c.when || c.when(route)));
      results.value = recentResults;
      activeIndex.value = 0;
      return;
    }

    loading.value = true;
    try {
      const matchedCommands = commands
        .filter((c) => !allowedGroups || allowedGroups.includes(c.group))
        .filter((c) => hasPermission(c.permission))
        .filter((c) => !c.when || c.when(route))
        .map((c) => ({ result: c as PaletteResult, score: rank(c, q, recents.value.includes(c.id)) }))
        .filter((x): x is { result: PaletteResult; score: number } => x.score !== undefined);

      const activeProviders = providers
        .filter((p) => !allowedGroups || allowedGroups.includes(p.group))
        .filter((p) => hasPermission(p.permission));

      // Barcode-first (docs/v2/14-platform.md §2): an 8–14 digit query is looked up directly against
      // Phase 6/7's barcode/SKU index (`findByCode`, the same exact-match lookup POS scanning uses)
      // before the fuzzy providers run, so a scanned barcode jumps straight to its product.
      const isBarcode = looksLikeBarcode(q);
      let barcodeResult: { result: PaletteResult; score: number }[] = [];
      if (isBarcode) {
        try {
          const product = await findByCode(q);
          if (!controller.signal.aborted && product) {
            barcodeResult = [
              {
                result: {
                  id: `product:${product.id}`,
                  group: 'products' as const,
                  title: product.name,
                  subtitle: product.barcode ?? product.sku,
                  keywords: `${product.sku} ${product.barcode ?? ''}`,
                  to: { name: 'product', params: { id: product.id } },
                },
                score: -200, // always first — an exact scanned-barcode match beats everything else
              },
            ];
          }
        } catch {
          /* no match — fall through to the fuzzy providers below */
        }
      }

      let providerResults: { result: PaletteResult; score: number }[] = [];
      if (q) {
        const settled = await Promise.all(
          activeProviders.map(async (p) => {
            try {
              const list = await p.search(q, controller.signal);
              return list.map((r) => ({ result: r, score: rank(r, q, recents.value.includes(r.id)) }));
            } catch {
              return [];
            }
          }),
        );
        providerResults = settled.flat().filter((x): x is { result: PaletteResult; score: number } => x.score !== undefined);
        if (isBarcode) providerResults = providerResults.map((x) => ({ ...x, score: x.score - 100 }));
      }
      providerResults = [...barcodeResult, ...providerResults];

      if (id !== requestId || controller.signal.aborted) return;

      const combined = [...matchedCommands, ...providerResults].sort((a, b) => a.score - b.score);
      // 5 per group + a "see all" affordance is a UI concern the panel component handles by group slicing.
      results.value = combined.map((x) => x.result);
      activeIndex.value = 0;
    } finally {
      if (id === requestId) loading.value = false;
    }
  }

  function onQueryChange() {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => void runSearch(), 120);
  }

  const grouped = computed(() => {
    const map = new Map<string, PaletteResult[]>();
    for (const r of results.value) {
      const list = map.get(r.group) ?? [];
      list.push(r);
      map.set(r.group, list);
    }
    return map;
  });

  return {
    open,
    query,
    loading,
    results,
    grouped,
    activeIndex,
    show,
    hide,
    toggle,
    onQueryChange,
    choose,
    detectPrefix,
  };
}
