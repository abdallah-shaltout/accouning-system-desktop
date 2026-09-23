/**
 * A small typed event bus the mock backend emits on after mutating service calls, so Pinia
 * master-data caches (and later, anything else) can invalidate/refetch instead of loading once.
 *
 * Lives in `src/mocks` (not `modules/core`) because it's the mock backend that knows *when* a
 * mutation happened — the emit side sits next to `mutate()` and the backend functions that call
 * it. A real backend would replace this with server push (websocket/SSE) behind the same
 * `on()`/`off()` shape, so callers in `modules/*` don't need to change.
 */

export type MockEvent = 'ledger:changed' | 'catalog:changed' | 'parties:changed';

type Listener = () => void;

const listeners: Record<MockEvent, Set<Listener>> = {
  'ledger:changed': new Set(),
  'catalog:changed': new Set(),
  'parties:changed': new Set(),
};

export function on(event: MockEvent, listener: Listener): () => void {
  listeners[event].add(listener);
  return () => listeners[event].delete(listener);
}

export function off(event: MockEvent, listener: Listener): void {
  listeners[event].delete(listener);
}

export function emit(event: MockEvent): void {
  for (const listener of listeners[event]) listener();
}

/*
 * Example of a future Pinia cache subscribing (accounts/parties don't have one yet — see
 * useCatalogStore for the one real example wired in this track):
 *
 *   import { on } from '@/mocks/events';
 *   on('parties:changed', () => store.load(true));
 */
