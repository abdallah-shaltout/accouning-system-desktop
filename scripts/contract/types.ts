/** Shapes shared by the backend-contract inventory stages (plans/pending/21-rust-backend, Part 01). */

/** What the Rust backend does with a service function. `port` = becomes a Rust command. */
export type Disposition = 'port' | 'rust-existing' | 'frontend' | 'dev-only' | 'drop';

export interface Param {
  name: string;
  type: string;
  optional: boolean;
}

/** Facts about one function body (service, helper or mock), before following its calls. */
export interface FnFacts {
  /** `<repo-relative file>#<name>` */
  id: string;
  file: string;
  name: string;
  line: number;
  /** `db.<table>` referenced anywhere in the body. */
  tables: string[];
  /** Tables the body writes (explicit mutators, aliases, anything inside a `mutate()` callback). A lower bound. */
  writes: string[];
  /** Ids of functions (inside src/) this body calls. */
  calls: string[];
  /** Non-function values imported from src/mocks (e.g. `session`) — state the Rust side must own. */
  mockRefs: string[];
  /** Tauri commands called through `invoke('<cmd>')` or `backendCall('<cmd>', …)`. */
  invokes: string[];
  /** Browser / Tauri-plugin APIs used directly. */
  platform: string[];
  round2: number;
  round4: number;
  /** `backendCall('<cmd>', …)` sites found directly in this function's own body (not through calls
   * into other functions) — the switch-line coverage check (21.04 phase A, A-2) walks these without
   * following the closure, since the guard must be textually local to the `port` function itself. */
  switchSites: SwitchSite[];
}

/** One `backendCall('<cmd>', …)` call site: which command, and which `usesRust('<domain>')` guard (if
 * any) directly encloses it. `guardDomain` is `undefined` when the call is not inside any
 * `usesRust(...)` condition (an ungated call — A-2 requires these to be on the allowlist). */
export interface SwitchSite {
  command: string;
  guardDomain?: string;
  line: number;
}

/** A function's facts after following every call it makes (transitively). */
export interface Closure {
  tables: string[];
  writes: string[];
  mockFns: string[];
  mockRefs: string[];
  invokes: string[];
  platform: string[];
  /** Shared-manager capabilities reached (config.capabilities keys: ledger, stock, audit…). */
  capabilities: string[];
  /** `backendCall` sites reached through local (non-service) helpers too — e.g. `computeAll()` inside
   * `insightEngine.ts` — but not through another `wrap()`-registered service (that service's own
   * switch line is verified independently; A-2 doesn't want to double-count across a service boundary). */
  switchSites: SwitchSite[];
}

export interface ServiceFn {
  /** The `wrap('<module>.<fn>', …)` source name — the key the replay registry and the Rust command use. */
  source: string;
  module: string;
  service: string;
  name: string;
  file: string;
  line: number;
  params: Param[];
  returns: string;
  /** Exported types (from `src/modules/*\/types`) named in params/returns — the DTOs this call needs. */
  dtoTypes: string[];
  /** Other wrapped service functions this one calls. */
  serviceCalls: string[];
  /** `backendCall(...)` sites in this function's own body (A-2) — not the transitive closure. */
  switchSites: SwitchSite[];
  closure: Closure;
  disposition: Disposition;
  dispositionReason: string;
  /** Suggested Rust command name (`<module>_<snake_fn>`), only for `port`. */
  rustCommand?: string;
}

export interface TypeField {
  name: string;
  type: string;
  optional: boolean;
  /** decimal | uuid | date | route | enum — hints for the Rust DTO / entity column. */
  hint?: string;
}

export interface TypeDecl {
  module: string;
  file: string;
  line: number;
  name: string;
  kind: 'interface' | 'type';
  /** Present for object shapes; a union/alias keeps its text in `text`. */
  fields?: TypeField[];
  text?: string;
}

export interface PathLink {
  file: string;
  line: number;
  text: string;
}

export interface Inventory {
  services: ServiceFn[];
  /** Exports of service files that are NOT wrapped (helpers, constants, re-exports). */
  unwrapped: { module: string; file: string; name: string }[];
  mockFns: FnFacts[];
  types: TypeDecl[];
  pathLinks: PathLink[];
  /** `MockDb` table names (from `src/mocks/db.ts`). */
  tables: string[];
}
