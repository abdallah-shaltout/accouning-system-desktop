/**
 * Backend health poller (21 · 03.01 §6): when running in Tauri with `usesRust('settings')`,
 * polls `getBackendStatus()` every 5s and exposes a `failure` computed the `ServerFailureScreen`
 * mounts on. Outside that gate (mock mode, e2e, `settings` not yet flipped to Rust) `failure` is
 * always `null` — nothing to poll, nothing to fail.
 */
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { isTauri } from '@tauri-apps/api/core';
import { getBackendStatus } from '@/modules/core/services/backend';
import { usesRust } from '@/modules/core/services/backend';
import type { BackendStatus } from '@/modules/core/types/backend';

export interface BackendFailure {
  message: string;
  code: string;
}

const POLL_INTERVAL_MS = 5000;

export const useBackendHealth = defineStore('backendHealth', () => {
  const status = ref<BackendStatus | null>(null);
  let timer: ReturnType<typeof setInterval> | null = null;

  const failure = computed<BackendFailure | null>(() => {
    const s = status.value;
    if (!s) return null;
    if (s.server?.state === 'failed' && s.server.failure) {
      return { message: s.server.failure.message, code: s.server.failure.code };
    }
    if (!s.connected && s.error) {
      return { message: s.error, code: s.schema.toUpperCase() };
    }
    return null;
  });

  const role = computed(() => status.value?.role ?? 'terminal');

  async function poll() {
    try {
      status.value = await getBackendStatus();
    } catch {
      // A failed status call itself is not surfaced as a `failure` — the ordinary error toast
      // path already handles it; this store only reflects what the backend reports about itself.
    }
  }

  function start() {
    if (!isTauri() || !usesRust('settings')) return;
    if (timer) return;
    void poll();
    timer = setInterval(() => void poll(), POLL_INTERVAL_MS);
  }

  function stop() {
    if (timer) {
      clearInterval(timer);
      timer = null;
    }
  }

  return { status, failure, role, start, stop, poll };
});
