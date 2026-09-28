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
 * `RUST_DOMAINS` is empty in this wave — Part 04 flips domains on one at a time as their Rust
 * commands land. Dev builds can still exercise the switch early via
 * `localStorage['equal.backend']` (a comma list of domains, or `*` for all), the same pattern as
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

/** Domains whose services call the real Rust backend. Empty in this wave (Part 04 flips domains
 * one at a time as their commands land) — every service still runs on the mock until then. */
const RUST_DOMAINS: ReadonlySet<BackendDomain> = new Set<BackendDomain>([]);

/** Dev-only override so a domain's Rust path can be exercised before `RUST_DOMAINS` flips it on
 * generally — `localStorage['equal.backend'] = 'products,invoices'` or `'*'` for every domain.
 * Ignored in production builds (`import.meta.env.DEV` gate), exactly like `equal.debug`. */
function devOverrideDomains(): ReadonlySet<BackendDomain> | '*' | null {
  if (!import.meta.env.DEV) return null;
  let raw: string | null = null;
  try {
    raw = localStorage.getItem('equal.backend');
  } catch {
    return null;
  }
  if (!raw) return null;
  const trimmed = raw.trim();
  if (trimmed === '*') return '*';
  const domains = trimmed
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean) as BackendDomain[];
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
  if (!isTauri()) return false;
  if (RUST_DOMAINS.has(domain)) return true;
  const override = devOverrideDomains();
  if (override === '*') return true;
  if (override && override.has(domain)) return true;
  return false;
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
    return await invoke<IpcCommands[K]['returns']>(command, args.length > 0 ? { args: args[0] } : undefined);
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
 * unchanged regardless of which backend answered (cross-cutting.md §5). A no-op outside Tauri, and
 * a no-op when no domain uses Rust yet — there is nothing to listen for until then. Called once
 * from `src/main.ts`, next to `initDiagnostics()`.
 *
 * (Moving `ApiError` and the event bus out of `src/mocks` is a D5 prerequisite, not done here —
 * see the entry file's §10 note; this bridge reaches into `src/mocks` only from a composition
 * root, which the seam rule's `compositionRoots` allowance already covers for `src/main.ts`.)
 */
export async function initBackendBridge(): Promise<void> {
  if (bridgeInitialized) return;
  if (!isTauri()) return;
  const anyRustDomain = RUST_DOMAINS.size > 0 || devOverrideDomains() !== null;
  if (!anyRustDomain) return;
  bridgeInitialized = true;

  await listen<BackendChangedPayload>('backend:changed', (event) => {
    for (const category of event.payload.categories) {
      emitMockEvent(mockEventFor(category));
    }
  });
}
