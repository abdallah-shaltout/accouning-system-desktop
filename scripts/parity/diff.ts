/**
 * The parity diff (plan 21 Part 04, B-8, decision P4-6): a structural walk of the mock pass's
 * result tree against the Rust pass's, **exact by default**.
 *
 * - **Ids:** a bijection mock id ↔ Rust UUID, seeded from the importer's `idPairs` and extended
 *   whenever a mock string faces a UUID at the same path. A conflicting pair is a diff — including
 *   a mapped mock id that shows up *literally* on the Rust side (e.g. an imported link that still
 *   says `highlight: 'pay-135'` although `pay-135` became a UUID: the link no longer points at the
 *   row, which is a real bug, not a string match).
 * - **Derived ids (P4-13 b):** an id built from a parent id is translated through the parent's
 *   pair, never learned on its own. The two schemes both backends use:
 *   `<parent>-l<n>` (invoice/quotation line ids, `sales.ts` `${id}-l${i + 1}` ↔ Rust
 *   `format!("{}-l{}", invoice_id, position + 1)`) — equal iff the parents pair and the suffixes are
 *   identical; and `<transfer>-recv` (the transfer receive/reject journal source, `transfers.ts`
 *   ↔ Rust `receipt_source_id(transfer_id)`, the transfer UUID with its version nibble set to 8 and byte 8 forced into `0xc0..=0xdf`).
 *   A direct pair (the importer maps a seeded line/`-recv` row id too) is accepted as well. When the
 *   parent is paired with something else, the derived id is an `id-conflict`.
 * - **Object keys that are ids (P4-13 c)** (`stockByBranch.<branchId>`, `products.<productId>`) are
 *   translated through the same map before the values are compared: a mock key pairs with the Rust
 *   key its mapping names; an unmapped mock key pairs with an unknown Rust UUID key whose value diffs
 *   clean (learning the pair), or — when exactly one of each is left — with that key. Paths keep
 *   the **mock** key, so allow patterns are written against mock ids.
 * - **`null` ≡ absent (P4-13 a):** a key whose value is JSON `null` on one side and that is missing
 *   on the other is **equal** — both mean "no value" for these DTOs (the TS types declare such
 *   fields `field?: T` / `T | null` and every reader uses `?.`/`??`; serde's `skip_serializing_if`
 *   drops what the mock writes as `null` and vice versa). This is a normalization rule, not an
 *   allowlist: it never covers `[]`/`0`/`false`/`''` vs absent — those stay diffs, decided per field
 *   by the owning lane.
 * - **Imported `createdAt` (00-import D-8, P4-13 d):** a `createdAt` present only on Rust is marked
 *   allowed (`allowedBy` set here, still listed in the output) only when the mock object's `id` is a
 *   base-snapshot row that carried no `createdAt` (`DiffOptions.importedWithoutCreatedAt`) — the
 *   importer must fill the NOT NULL column (`import_base + i ms`). A row created during the case, or
 *   a snapshot row that had one, still diffs.
 * - **Instants** (ISO strings with a `Z`/offset) compare as epoch ms.
 * - **Numbers** compare exactly, except on the case's `epsilon` paths (`|a − b| ≤ 1e-9`).
 * - **Arrays** compare in order, except on the case's `unordered` paths (multiset).
 * - **Objects:** a key present on one side only is a diff (except the `null` rule above) —
 *   including `[]`/absent and default-vs-absent mismatches (allowlist them, citing a decision).
 * - **Errors** are ordinary `{ code, message }` records, so they compare byte for byte.
 *
 * In `--mock-only` mode the "rust" side is simply the mock's second run.
 */

export type DiffKind = 'value' | 'type' | 'missing' | 'extra' | 'id-conflict' | 'unmatched';

export interface Diff {
  path: string;
  kind: DiffKind;
  mock: unknown;
  rust: unknown;
  /** The step name when the path is under `steps.`. */
  step?: string;
  /** The allowlist reason that explains this diff, if any (set by `allow.ts`). */
  allowedBy?: string;
}

export const EPSILON = 1e-9;

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const INSTANT_RE = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(?::\d{2}(?:\.\d+)?)?(?:Z|[+-]\d{2}:?\d{2})$/;

export function isUuid(s: string): boolean {
  return UUID_RE.test(s);
}

/** Mock id ↔ Rust id bijection. */
export class IdMap {
  readonly m2r = new Map<string, string>();
  readonly r2m = new Map<string, string>();

  constructor(pairs?: Iterable<[string, string]>) {
    if (pairs) for (const [m, r] of pairs) this.set(m, r);
  }

  set(m: string, r: string): void {
    this.m2r.set(m, r);
    this.r2m.set(r, m);
  }

  clone(): IdMap {
    const c = new IdMap();
    for (const [m, r] of this.m2r) c.set(m, r);
    return c;
  }

  adopt(other: IdMap): void {
    for (const [m, r] of other.m2r) this.set(m, r);
  }

  /** Replaces every string that is a known mock id with its Rust id, recursively (bundle arg remap). */
  remap(value: unknown): unknown {
    if (typeof value === 'string') return this.m2r.get(value) ?? value;
    if (Array.isArray(value)) return value.map((v) => this.remap(v));
    if (value && typeof value === 'object') {
      const out: Record<string, unknown> = {};
      for (const [k, v] of Object.entries(value)) out[k] = this.remap(v);
      return out;
    }
    return value;
  }
}

/** Path pattern → regex. `*` one key segment, `[]` any index, `**` any depth. */
export function compilePattern(pattern: string, prefix: boolean): RegExp {
  let re = '';
  for (let i = 0; i < pattern.length; ) {
    if (pattern.startsWith('**', i)) {
      re += '.*';
      i += 2;
    } else if (pattern[i] === '*') {
      re += '[^.\\[\\]]+';
      i += 1;
    } else if (pattern.startsWith('[]', i)) {
      re += '\\[\\d+\\]';
      i += 2;
    } else {
      re += pattern[i].replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      i += 1;
    }
  }
  return new RegExp(prefix ? `^${re}(?:$|[.\\[])` : `^${re}$`);
}

export interface DiffOptions {
  epsilon?: string[];
  unordered?: string[];
  /** Mock ids of the base-snapshot rows that carried no `createdAt` (00-import D-8 — see header). */
  importedWithoutCreatedAt?: ReadonlySet<string>;
}

interface Ctx {
  ids: IdMap;
  epsilon: RegExp[];
  unordered: RegExp[];
  imported: ReadonlySet<string>;
  out: Diff[];
}

/** The reason stamped on an imported-row `createdAt` that only Rust has (header, P4-13 d). */
export const IMPORTED_CREATED_AT_REASON =
  '00-import D-8 / P4-13: a base-snapshot row with no createdAt gets import_base + i ms on Rust (the column is NOT NULL); the mock row has none';

function typeOf(v: unknown): string {
  if (v === null) return 'null';
  if (Array.isArray(v)) return 'array';
  return typeof v;
}

function stepOf(path: string): string | undefined {
  const m = /^steps\.([^.[\]]+)/.exec(path);
  return m?.[1];
}

function push(ctx: Ctx, path: string, kind: DiffKind, mock: unknown, rust: unknown, allowedBy?: string): void {
  const d: Diff = { path, kind, mock, rust, step: stepOf(path) };
  if (allowedBy) d.allowedBy = allowedBy;
  ctx.out.push(d);
}

function childPath(path: string, key: string): string {
  return path ? `${path}.${key}` : key;
}

function walk(a: unknown, b: unknown, path: string, ctx: Ctx): void {
  const ta = typeOf(a);
  const tb = typeOf(b);
  if (ta === 'string' && tb === 'string') return diffStrings(a as string, b as string, path, ctx);
  if (ta !== tb) return push(ctx, path, 'type', a, b);
  switch (ta) {
    case 'number': {
      const x = a as number;
      const y = b as number;
      if (x === y || (Number.isNaN(x) && Number.isNaN(y))) return;
      if (ctx.epsilon.some((r) => r.test(path)) && Math.abs(x - y) <= EPSILON) return;
      return push(ctx, path, 'value', a, b);
    }
    case 'array':
      if (ctx.unordered.some((r) => r.test(path))) return diffMultiset(a as unknown[], b as unknown[], path, ctx);
      return diffOrdered(a as unknown[], b as unknown[], path, ctx);
    case 'object':
      return diffObjects(a as Record<string, unknown>, b as Record<string, unknown>, path, ctx);
    default:
      if (a !== b) push(ctx, path, 'value', a, b);
  }
}

/** Learns `m ↔ r` when neither side is taken yet (and `r` is a UUID); true if the pair now holds. */
function learn(ids: IdMap, m: string, r: string): boolean {
  const known = ids.m2r.get(m);
  if (known !== undefined) return known === r;
  if (ids.r2m.has(r) || isUuid(m) || !isUuid(r)) return false;
  ids.set(m, r);
  return true;
}

const LINE_SUFFIX_RE = /^(.+)(-l\d+)$/;
const RECV_SUFFIX = '-recv';

/** Rust's `receipt_source_id` (`domains/products/service/transfers.rs`): the transfer UUID with its
 * version nibble (the first hex digit of the third group) set to 8 and byte 8 (the first two hex
 * digits of the fourth group) set to `(b & 0x1f) | 0xc0` — MariaDB refuses a v8 UUID whose byte 8 is
 * `0x01..=0x80`. */
export function receiptSourceId(transferUuid: string): string {
  const u = transferUuid.toLowerCase();
  const byte8 = ((parseInt(u.slice(19, 21), 16) & 0x1f) | 0xc0).toString(16).padStart(2, '0');
  return `${u.slice(0, 14)}8${u.slice(15, 19)}${byte8}${u.slice(21)}`;
}

/** Derived-id translation (header, P4-13 b). */
function diffDerived(a: string, b: string, ctx: Ctx): 'match' | 'conflict' | 'none' {
  const ma = LINE_SUFFIX_RE.exec(a);
  const mb = LINE_SUFFIX_RE.exec(b);
  if (ma && mb && ma[2] === mb[2]) {
    const [pa, pb] = [ma[1], mb[1]];
    const parent = ctx.ids.m2r.get(pa);
    if (parent !== undefined) return parent === pb ? 'match' : 'conflict';
    if (pa === pb) return 'none';
    return learn(ctx.ids, pa, pb) ? 'match' : 'conflict';
  }
  if (a.endsWith(RECV_SUFFIX) && isUuid(b)) {
    const parent = ctx.ids.m2r.get(a.slice(0, -RECV_SUFFIX.length));
    if (parent !== undefined && isUuid(parent)) return receiptSourceId(parent) === b.toLowerCase() ? 'match' : 'conflict';
  }
  return 'none';
}

/** Colon-joined composite ids (`<invoiceId>:<lineId>` VAT-detail rows, `year-end:<fyId>` /
 * `reorder-item:<productId>` insight keys, P4-13 b): same part count, and every part equal, mapped,
 * derived or learnable. Learned pairs are kept only when the whole composite matches. */
function diffComposite(a: string, b: string, ctx: Ctx): 'match' | 'conflict' | 'none' {
  const pa = a.split(':');
  const pb = b.split(':');
  if (pa.length < 2 || pa.length !== pb.length) return 'none';
  const t: Ctx = { ...ctx, ids: ctx.ids.clone(), out: [] };
  let idMismatch = false;
  let textMismatch = false;
  for (let i = 0; i < pa.length; i++) {
    const [x, y] = [pa[i], pb[i]];
    if (x === y) continue;
    // Same order as a whole-string id (diffStrings): a derived id through its parent first, then
    // the direct pair — the importer also maps each seeded line id to its own row UUID.
    const derived = diffDerived(x, y, t);
    if (derived === 'match') continue;
    const mapped = t.ids.m2r.get(x);
    if (mapped !== undefined) {
      if (mapped !== y) idMismatch = true;
      continue;
    }
    if (derived === 'conflict') {
      idMismatch = true;
      continue;
    }
    if (isUuid(y) && !isUuid(x)) {
      if (!learn(t.ids, x, y)) idMismatch = true;
      continue;
    }
    textMismatch = true;
  }
  if (!idMismatch && !textMismatch) {
    ctx.ids.adopt(t.ids);
    return 'match';
  }
  return idMismatch && !textMismatch ? 'conflict' : 'none';
}

function diffStrings(a: string, b: string, path: string, ctx: Ctx): void {
  const mapped = ctx.ids.m2r.get(a);
  if (mapped === b) return;
  // Composite first: `inv-634:inv-634-l1` also ends in `-l1`, and the whole string would otherwise
  // be read as one derived id whose "parent" (`inv-634:inv-634`) is never mapped.
  if (mapped === undefined && a.includes(':') && b.includes(':')) {
    const composite = diffComposite(a, b, ctx);
    if (composite === 'match') return;
    if (composite === 'conflict') return push(ctx, path, 'id-conflict', a, b);
  }
  const derived = diffDerived(a, b, ctx);
  if (derived === 'match') return;
  // `mapped !== undefined` also covers a mapped mock id that Rust shows literally (header).
  if (derived === 'conflict' || mapped !== undefined) return push(ctx, path, 'id-conflict', a, b);
  if (a === b) return;
  if (isUuid(b) && !isUuid(a)) {
    if (ctx.ids.r2m.has(b)) return push(ctx, path, 'id-conflict', a, b); // b already belongs to another mock id
    ctx.ids.set(a, b);
    return;
  }
  if (INSTANT_RE.test(a) && INSTANT_RE.test(b) && Date.parse(a) === Date.parse(b)) return;
  push(ctx, path, 'value', a, b);
}

function diffOrdered(a: unknown[], b: unknown[], path: string, ctx: Ctx): void {
  const n = Math.min(a.length, b.length);
  for (let i = 0; i < n; i++) walk(a[i], b[i], `${path}[${i}]`, ctx);
  for (let i = n; i < a.length; i++) push(ctx, `${path}[${i}]`, 'missing', a[i], undefined);
  for (let i = n; i < b.length; i++) push(ctx, `${path}[${i}]`, 'extra', undefined, b[i]);
}

/** Walks `a` against `b` on a scratch copy of the id map (optionally after learning `learnKey`
 * first). Clean (no unexplained diff) → the learned pairs and any pre-allowed diffs are kept and
 * true is returned; otherwise nothing is learned or recorded. */
function trial(a: unknown, b: unknown, path: string, ctx: Ctx, learnKey?: [string, string]): boolean {
  const t: Ctx = { ...ctx, ids: ctx.ids.clone(), out: [] };
  if (learnKey && !learn(t.ids, learnKey[0], learnKey[1])) return false;
  walk(a, b, path, t);
  if (t.out.some((d) => !d.allowedBy)) return false;
  ctx.ids.adopt(t.ids);
  ctx.out.push(...t.out);
  return true;
}

/** Multiset: each mock element pairs with the first unmatched Rust element it diffs clean against
 * (ids learned during a successful trial are kept; a failed trial learns nothing). */
function diffMultiset(a: unknown[], b: unknown[], path: string, ctx: Ctx): void {
  const used = new Array<boolean>(b.length).fill(false);
  for (let i = 0; i < a.length; i++) {
    let matched = false;
    for (let j = 0; j < b.length && !matched; j++) {
      if (used[j]) continue;
      if (trial(a[i], b[j], `${path}[${i}]`, ctx)) {
        used[j] = true;
        matched = true;
      }
    }
    if (!matched) push(ctx, `${path}[${i}]`, 'unmatched', a[i], undefined);
  }
  for (let j = 0; j < b.length; j++) if (!used[j]) push(ctx, `${path}[${j}]`, 'unmatched', undefined, b[j]);
}

const has = (o: object, k: string) => Object.prototype.hasOwnProperty.call(o, k);

/** The harness's own containers — the root (`steps`/`books`/`invariants`), step names, `ok`/`value`/
 * `error`, book parts — whose keys are names a case or the runner chose, never ids (a case may well
 * name a step `prd-1`). Their keys compare literally. */
const HARNESS_KEYS_RE = /^(?:|steps|books|steps\.[^.[\]]+|books\.[^.[\]]+)$/;

/** A derived-id key (`<parent>-lN`, e.g. `InvoiceDetail.returnedQty.inv-635-l1`, P4-13 b) translated
 * through its parent's pair, like a derived id value (`diffDerived`). */
function derivedKey(ka: string, ctx: Ctx): string | undefined {
  const m = LINE_SUFFIX_RE.exec(ka);
  const parent = m ? ctx.ids.m2r.get(m[1]) : undefined;
  return m && parent !== undefined ? `${parent}${m[2]}` : undefined;
}

/** Pairs each mock key with a Rust key (header, P4-13 c): its mapped id if Rust has that key, else
 * the same literal key; then unmapped mock keys against unknown Rust UUID keys by a clean value
 * trial, and finally a lone leftover on each side with each other. */
function pairKeys(a: Record<string, unknown>, b: Record<string, unknown>, path: string, ctx: Ctx) {
  const taken = new Set<string>();
  const pairs: [string, string][] = [];
  let leftA: string[] = [];
  if (HARNESS_KEYS_RE.test(path)) {
    for (const ka of Object.keys(a)) (has(b, ka) ? (taken.add(ka), pairs.push([ka, ka])) : leftA.push(ka));
    return { pairs, leftA, leftB: Object.keys(b).filter((kb) => !taken.has(kb)), literal: true };
  }
  for (const ka of Object.keys(a)) {
    const mapped = ctx.ids.m2r.get(ka) ?? derivedKey(ka, ctx);
    if (mapped !== undefined && mapped !== ka && has(b, mapped) && !taken.has(mapped)) {
      pairs.push([ka, mapped]);
      taken.add(mapped);
    } else if (has(b, ka) && !taken.has(ka)) {
      pairs.push([ka, ka]);
      taken.add(ka);
    } else leftA.push(ka);
  }
  const open = () => Object.keys(b).filter((kb) => !taken.has(kb) && isUuid(kb) && !ctx.ids.r2m.has(kb));
  const pairable = (ka: string) => !ctx.ids.m2r.has(ka) && !isUuid(ka);
  for (const ka of leftA.filter(pairable)) {
    const kb = open().find((cand) => trial(a[ka], b[cand], childPath(path, ka), ctx, [ka, cand]));
    if (kb !== undefined) {
      taken.add(kb);
      leftA = leftA.filter((k) => k !== ka);
    }
  }
  const lastA = leftA.filter(pairable);
  const lastB = open();
  if (lastA.length === 1 && lastB.length === 1 && learn(ctx.ids, lastA[0], lastB[0])) {
    pairs.push([lastA[0], lastB[0]]);
    taken.add(lastB[0]);
    leftA = leftA.filter((k) => k !== lastA[0]);
  }
  const leftB = Object.keys(b).filter((kb) => !taken.has(kb));
  return { pairs, leftA, leftB, literal: false };
}

function diffObjects(a: Record<string, unknown>, b: Record<string, unknown>, path: string, ctx: Ctx): void {
  const { pairs, leftA, leftB, literal } = pairKeys(a, b, path, ctx);
  for (const [ka, kb] of pairs) {
    const p = childPath(path, ka);
    const mapped = ctx.ids.m2r.get(ka);
    // A mapped mock id used literally as a Rust key: the key no longer names the row.
    if (!literal && ka === kb && mapped !== undefined && mapped !== ka) push(ctx, p, 'id-conflict', ka, kb);
    walk(a[ka], b[kb], p, ctx);
  }
  // `null` ≡ absent (P4-13 a).
  for (const ka of leftA) if (a[ka] !== null) push(ctx, childPath(path, ka), 'missing', a[ka], undefined);
  const mockId = typeof a.id === 'string' ? a.id : undefined;
  for (const kb of leftB) {
    if (b[kb] === null) continue;
    const importedCreatedAt = kb === 'createdAt' && typeof b[kb] === 'string' && mockId !== undefined && ctx.imported.has(mockId);
    push(ctx, childPath(path, ctx.ids.r2m.get(kb) ?? kb), 'extra', undefined, b[kb], importedCreatedAt ? IMPORTED_CREATED_AT_REASON : undefined);
  }
}

/** Diffs two result trees. `ids` is extended in place with every pair learned. */
export function diffTrees(mock: unknown, rust: unknown, ids: IdMap, opts: DiffOptions = {}, rootPath = ''): Diff[] {
  const ctx: Ctx = {
    ids,
    epsilon: (opts.epsilon ?? []).map((p) => compilePattern(p, false)),
    unordered: (opts.unordered ?? []).map((p) => compilePattern(p, false)),
    imported: opts.importedWithoutCreatedAt ?? new Set(),
    out: [],
  };
  walk(mock, rust, rootPath, ctx);
  return ctx.out;
}

/** Mock ids of every row in a snapshot's `data` that has a string `id` and no `createdAt`
 * (`null`/absent) — the rows the importer stamps with `import_base + i ms` (00-import D-8). */
export function idsWithoutCreatedAt(data: unknown): Set<string> {
  const out = new Set<string>();
  const visit = (v: unknown): void => {
    if (!v || typeof v !== 'object') return;
    if (Array.isArray(v)) return v.forEach(visit);
    const o = v as Record<string, unknown>;
    if (typeof o.id === 'string' && (o.createdAt === undefined || o.createdAt === null)) out.add(o.id);
    Object.values(o).forEach(visit);
  };
  visit(data);
  return out;
}

/** JSON-normalizes a recorded value: `undefined` keys dropped, `Date`s to ISO strings, a top-level
 * `undefined` to `null` (a void mock service and a unit Rust command both record `null`). */
export function normalize(value: unknown): unknown {
  if (value === undefined) return null;
  return JSON.parse(JSON.stringify(value)) as unknown;
}
