/**
 * `backend.ts`: the one switch point (21.02-F, F-3) deciding, per domain, whether a service
 * function talks to the mock backend (`src/mocks`) or the real Rust backend (Tauri IPC). Master
 * plan §5: pages never know which one answered — a service function's own body picks:
 *
 *   export const getProduct = wrap('products.getProduct', async function getProduct(id: string) {
 *     if (usesRust('products')) return backendCall('products_get_product', { id });
 *     // ...unchanged mock body below
 *   });
 *
 * Part 04 E-1 (2026-10-01): `RUST_DOMAINS` now lists every `BackendDomain` — the Tauri app runs
 * entirely on the real Rust/MariaDB backend (P4-1: every domain flips together, never a mix of
 * screens on different backends). This only takes effect inside a Tauri webview (`usesRust`
 * returns `false` outside Tauri regardless): the browser dev server and the browser e2e suite
 * keep running on the mock until D5 retires it. A dev build can still force the mock everywhere
 * (for UI work with no DB) via `localStorage['equal.backend'] = 'mock'`, the same pattern as
 * `equal.debug` (`diagnostics/services/logService.ts`).
 */
import { isTauri, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ApiError } from '@/mocks/utils';
import { emit as emitMockEvent, type MockEvent } from '@/mocks/events';

/** Re-exported so pages/components can catch a failed `backendCall` without importing
 * `@/mocks` directly (seam rule, CLAUDE.md) — this service is the one designated seam boundary
 * that may still reach into `src/mocks` until D5 retires it. */
export { ApiError };
import { wrap } from '@/modules/diagnostics/services/defineService';
import type { ApiErrorPayload, BackendChangedPayload, BackendStatus, ChangeCategory } from '@/modules/core/types/backend';
import type { IpcCommands } from '@/modules/core/types/gen/ipc.gen';

/** Every Rust domain (master §4) that could, in principle, have Rust-backed services — plus
 * `diagnostics` (C-10), which isn't a `modules/<domain>` folder but does have its own services
 * (`auditService`, `accountingDebugService`) that will eventually call through the same switch. */
export type BackendDomain =
  | 'accounting'
  | 'analytics'
  | 'approvals'
  | 'core'
  | 'dashboard'
  | 'diagnostics'
  | 'expenses'
  | 'invoices'
  | 'parties'
  | 'payments'
  | 'products'
  | 'purchases'
  | 'reports'
  | 'settings'
  | 'setup'
  | 'templates'
  | 'users'
  | 'vouchers';

/** Every member of `BackendDomain`, as a runtime list (plan 21 Part 04, E-1/E-3): `usesRustEverywhere()`
 * walks it, and E-1's flip is `RUST_DOMAINS = new Set(ALL_BACKEND_DOMAINS)` (P4-1: every domain flips
 * together). `_AllDomainsListed` below fails `vue-tsc` if a domain is added to the union but not here. */
export const ALL_BACKEND_DOMAINS = [
  'accounting',
  'analytics',
  'approvals',
  'core',
  'dashboard',
  'diagnostics',
  'expenses',
  'invoices',
  'parties',
  'payments',
  'products',
  'purchases',
  'reports',
  'settings',
  'setup',
  'templates',
  'users',
  'vouchers',
] as const satisfies readonly BackendDomain[];

/** Compile-time check: `never` only while `ALL_BACKEND_DOMAINS` lists every `BackendDomain`. */
type _AllDomainsListed = Exclude<BackendDomain, (typeof ALL_BACKEND_DOMAINS)[number]>;
const _allDomainsListed: [_AllDomainsListed] extends [never] ? true : never = true;
void _allDomainsListed;

/** Domains whose services call the real Rust backend. Every `BackendDomain` as of Part 04 E-1
 * (2026-10-01, P4-1: all domains flip together, behind this one switch). */
const RUST_DOMAINS: ReadonlySet<BackendDomain> = new Set<BackendDomain>(ALL_BACKEND_DOMAINS);

/** Dev-only escape hatch to force every domain back onto the mock — `localStorage['equal.backend']
 * = 'mock'` — for UI work with no database running. A comma list of domains is also accepted for
 * finer-grained debugging (only those domains skip Rust); anything else is ignored. Ignored in
 * production builds (`import.meta.env.DEV` gate), exactly like `equal.debug`. */
function devMockOverrideDomains(): ReadonlySet<BackendDomain> | 'all' | null {
  if (!import.meta.env.DEV) return null;
  let raw: string | null = null;
  try {
    raw = localStorage.getItem('equal.backend');
  } catch {
    return null;
  }
  if (!raw) return null;
  const trimmed = raw.trim();
  if (trimmed === 'mock') return 'all';
  const domains = trimmed
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean) as BackendDomain[];
  if (domains.length === 0) return null;
  if (domains.length > 0 && domains.length < ALL_BACKEND_DOMAINS.length) {
    // Mixed mode shows inconsistent books between screens still reading the mock and documents
    // already posted to MariaDB (P4-1) — allowed only for active debugging, never silently.
    console.warn(
      '[backend] equal.backend dev override forces the mock for a subset of domains — books will look inconsistent between screens; debugging use only.',
    );
  }
  return new Set(domains);
}

/**
 * Whether `domain`'s services should call the real Rust backend right now. Browser/e2e always use
 * the mock (master §5 — `isTauri()` is false there), even with the dev override set, since there is
 * no Tauri IPC to call outside a Tauri webview. **No fallback** to the mock after a Rust failure
 * (P2-34): once a domain uses Rust, a failed call surfaces as an `ApiError`, it does not retry on
 * the mock.
 */
export function usesRust(domain: BackendDomain): boolean {
  if (parityTransport) return true;
  if (!isTauri()) return false;
  const override = devMockOverrideDomains();
  if (override === 'all') return false;
  if (override && override.has(domain)) return false;
  return RUST_DOMAINS.has(domain);
}

/**
 * Whether every domain uses the real Rust backend (plan 21 Part 04, E-3): the app is in "Rust mode".
 * Then the mock must neither load nor write the browser's own IndexedDB snapshot (`main.ts` skips
 * `bootMockDb()`, `mocks/persist.ts` refuses snapshot writes and resets), because that snapshot is the
 * user's legacy data waiting for its one-time import (P4-9). `false` in the browser build and in e2e
 * (`usesRust` is false outside Tauri), and while `RUST_DOMAINS` is empty unless a dev override puts
 * every domain on Rust.
 */
export function usesRustEverywhere(): boolean {
  return ALL_BACKEND_DOMAINS.every((domain) => usesRust(domain));
}

/** Plan 21 Part 04 B-7: a headless transport to the Rust `parity_host` (`scripts/parity/transport.ts`),
 * set only by the parity runner. Same signature as Tauri's `invoke` for the calls `backendCall` makes. */
export type ParityTransport = (command: string, payload?: { args: unknown }) => Promise<unknown>;

let parityTransport: ParityTransport | null = null;

/**
 * Parity-harness hook (plan 21 Part 04, B-7, decision P4-2): while a transport is set, every
 * `usesRust()` answers `true` and `backendCall` sends through the transport instead of Tauri's
 * `invoke`, with the same `ApiError` conversion — so `bun run parity` drives the real services
 * against Rust without touching any of them. Refuses inside Tauri: the real app can never use it.
 */
export function setParityTransport(transport: ParityTransport | null): void {
  if (isTauri()) throw new Error('setParityTransport is a headless parity-harness hook and is never available inside the Tauri app');
  parityTransport = transport;
}

/** Generic Arabic message for a Rust-side failure with no structured `{code, message}` payload
 * (a panic, a transport error) — mirrors the mock's own generic wording style. */
const GENERIC_INTERNAL_MESSAGE = 'حدث خطأ غير متوقع — حاول مرة أخرى';

function isApiErrorPayload(value: unknown): value is ApiErrorPayload {
  return (
    typeof value === 'object' &&
    value !== null &&
    'code' in value &&
    'message' in value &&
    typeof (value as { code: unknown }).code === 'string' &&
    typeof (value as { message: unknown }).message === 'string'
  );
}

/**
 * Calls a Rust command through Tauri IPC, typed by the generated `IpcCommands` manifest
 * (`core/types/gen/ipc.gen.ts`) so an unknown command name or a wrong return type fails
 * `vue-tsc` at the call site (F-4). Converts any rejection into the same `ApiError` shape the
 * mock throws — a `{code, message}` payload (an `AppError` serialized by Tauri) keeps its code;
 * anything else (a panic, a transport error) becomes `'INTERNAL'` with a generic Arabic message,
 * never the raw (possibly English, possibly sensitive) underlying error text.
 */
export async function backendCall<K extends keyof IpcCommands>(
  command: K,
  ...args: IpcCommands[K]['args'] extends undefined ? [] : [args: IpcCommands[K]['args']]
): Promise<IpcCommands[K]['returns']> {
  try {
    const payload = args.length > 0 ? { args: args[0] } : undefined;
    if (parityTransport) return (await parityTransport(command, payload)) as IpcCommands[K]['returns'];
    return await invoke<IpcCommands[K]['returns']>(command, payload);
  } catch (err) {
    if (isApiErrorPayload(err)) {
      throw new ApiError(err.message, err.code);
    }
    throw new ApiError(GENERIC_INTERNAL_MESSAGE, 'INTERNAL');
  }
}

/**
 * The real backend's own connection/role/schema/server status (F-2's `core_backend_status`,
 * no args). Not gated by `usesRust()` — this call only exists in Tauri and has no mock
 * equivalent (there is no "mock backend status" to fall back to), so it's a literal `invoke`
 * through `backendCall` rather than a per-domain switch-point service.
 */
export const getBackendStatus = wrap('core.getBackendStatus', async function getBackendStatus(): Promise<BackendStatus> {
  return backendCall('core_backend_status');
});

/** Maps a `ChangeCategory` to the mock event bus's matching event name — the two lists are kept in
 * lockstep by design (F-2/F-3): every category the Rust side can report has a corresponding mock
 * event so a subscriber never has to know which backend fired it. */
function mockEventFor(category: ChangeCategory): MockEvent {
  return `${category}:changed` as MockEvent;
}

let bridgeInitialized = false;

/**
 * Bridges the real backend's `backend:changed` Tauri event onto the existing mock event bus
 * (`src/mocks/events.ts`), so `onLedgerChanged`/`onCatalogChanged`/parties subscribers keep working
 * unchanged regardless of which backend answered (cross-cutting.md §5). A no-op outside Tauri. Called
 * once from `src/main.ts`, next to `initDiagnostics()`.
 *
 * (Moving `ApiError` and the event bus out of `src/mocks` is a D5 prerequisite, not done here —
 * see the entry file's §10 note; this bridge reaches into `src/mocks` only from a composition
 * root, which the seam rule's `compositionRoots` allowance already covers for `src/main.ts`.)
 */
export async function initBackendBridge(): Promise<void> {
  if (bridgeInitialized) return;
  if (!isTauri()) return;
  bridgeInitialized = true;

  await listen<BackendChangedPayload>('backend:changed', (event) => {
    for (const category of event.payload.categories) {
      emitMockEvent(mockEventFor(category));
    }
  });
}
