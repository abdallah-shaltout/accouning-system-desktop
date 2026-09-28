/**
 * 21.03 §02-setup (Part 02 handoff §9, D-1/D-2): the first-run device role step + terminal
 * pairing. Every function here is Rust-only (`setup_get_device_setup_state`/`setup_provision_main`/
 * `setup_pair_terminal`) — there is no mock body, because a mock backend has no real embedded
 * database or LAN server to provision or pair against. Dormant until `usesRust('setup')`: a
 * browser/e2e run (`usesRust` gates on `isTauri()`) never reaches these, so `DeviceSetupPage.vue`
 * and the router guard are exercised only in a real desktop build.
 */
import { backendCall } from '@/modules/core/services/backend';
import { wrap } from '@/modules/diagnostics/services/defineService';
import type { DeviceSetupState, PairTerminalInput } from '../types';

let deviceStateCache: DeviceSetupState | null = null;
let deviceStatePromise: Promise<DeviceSetupState> | null = null;

export const getDeviceSetupState = wrap('setup.getDeviceSetupState', async function getDeviceSetupState(): Promise<DeviceSetupState> {
  return backendCall('setup_get_device_setup_state');
});

/** Re-fetches and re-caches the device state — called right after a successful import/reseed so
 * `isFreshInstallCached()` reflects the new `hasUsers` without a full page reload (00-import's W2
 * follow-up: `legacyImportService.importLegacySnapshot` / `devToolsService.reloadDemoData`'s Rust
 * branch each call this once their write commits). */
export async function refreshDeviceSetupState(): Promise<DeviceSetupState> {
  const state = await getDeviceSetupState();
  deviceStateCache = state;
  return state;
}

/** Cached promise so the router guard and every early page can await the same in-flight call
 * instead of racing several `setup_get_device_setup_state` IPC round trips on first paint. */
export function ensureDeviceSetupState(): Promise<DeviceSetupState> {
  if (deviceStateCache) return Promise.resolve(deviceStateCache);
  if (!deviceStatePromise) {
    deviceStatePromise = refreshDeviceSetupState().finally(() => {
      deviceStatePromise = null;
    });
  }
  return deviceStatePromise;
}

/** Sync read of the last-known device state — `authService.isFreshInstall`'s Rust branch (03-users
 * H-2) needs a synchronous answer for the router guard; before the cache is ever populated this
 * defaults to "fresh" (a fresh install shows the device-setup screen first, which populates the
 * cache before any fresh-install check runs). */
export function isFreshInstallCached(): boolean {
  return !deviceStateCache?.hasUsers;
}

export const provisionMainDevice = wrap('setup.provisionMainDevice', async function provisionMainDevice(): Promise<DeviceSetupState> {
  const state = await backendCall('setup_provision_main');
  deviceStateCache = state;
  return state;
});

export const pairTerminalDevice = wrap('setup.pairTerminalDevice', async function pairTerminalDevice(input: PairTerminalInput): Promise<DeviceSetupState> {
  const state = await backendCall('setup_pair_terminal', input);
  deviceStateCache = state;
  return state;
});
