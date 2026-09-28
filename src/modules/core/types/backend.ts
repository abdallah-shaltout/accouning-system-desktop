/**
 * Cross-cutting backend-bridge types (21.02-F, F-2). The frontend defines these first (master
 * plan rule 1) — Rust's `src-tauri/src/core/dto.rs` mirrors them exactly, with ts-rs bindings
 * checked against this file by `src/modules/core/types/contract.check.ts` (F-4).
 */

/**
 * The closed wire error catalogue (cross-cutting.md §4). Matches `src/mocks/utils.ts`'s
 * `ApiError` codes exactly, plus `'INTERNAL'` — a Rust-only addition (P2-23 / C-02) for
 * infrastructure failures the mock can never produce (DB unreachable, unexpected DB error).
 */
export type ApiErrorCode = 'NOT_FOUND' | 'VALIDATION' | 'CONFLICT' | 'FORBIDDEN' | 'UNAUTHORIZED' | 'INTERNAL';

/** The serialized shape of Rust's `AppError` — `{ code, message }`, nothing else. */
export interface ApiErrorPayload {
  code: ApiErrorCode;
  message: string;
}

/**
 * The three change-versions categories (P2-10/P2-11) — match the mock's three event names
 * (`ledger:changed`/`catalog:changed`/`parties:changed`) minus the `:changed` suffix.
 */
export type ChangeCategory = 'ledger' | 'catalog' | 'parties';

/** Payload of the `backend:changed` Tauri event — carries no data, only which categories changed. */
export interface BackendChangedPayload {
  categories: ChangeCategory[];
}

/**
 * The managed-server sub-status (added by phase A2, C-22) — present only on a Main PC with a
 * managed server, built from `AppState.server.snapshot()`.
 */
export interface BackendServerStatus {
  state: 'provisioning' | 'starting' | 'upgrading' | 'running' | 'stopped' | 'failed';
  version: string;
  port: number;
  lanSharing: boolean;
  failure?: { code: string; message: string };
}

/**
 * Result of the (not-yet-registered — a later wave) `core_backend_status` command: the real
 * backend's connection/role/schema state. `server` is omitted entirely on a terminal or an
 * unmanaged connection.
 */
export interface BackendStatus {
  connected: boolean;
  role: 'main' | 'terminal';
  terminalId: string;
  serverVersion?: string;
  schema: 'ok' | 'behind' | 'ahead' | 'unknown';
  error?: string;
  server?: BackendServerStatus;
}
