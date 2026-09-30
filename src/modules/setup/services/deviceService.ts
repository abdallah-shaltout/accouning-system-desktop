/**
 * 21.03 §02-setup (Part 02 handoff §9, D-1/D-2): the first-run device role step + terminal
 * pairing. Every function here is Rust-only (`setup_get_device_setup_state`/`setup_provision_main`/
 * `setup_pair_terminal`) — there is no mock body, because a mock backend has no real embedded
 * database or LAN server to provision or pair against. Dormant until `usesRust('setup')`: a
 * browser/e2e run (`usesRust` gates on `isTauri()`) never reaches these, so `DeviceSetupPage.vue`
 * and the router guard are exercised only in a real desktop build.
 */
import { backendCall, getBackendStatus } from '@/modules/core/services/backend';
import { wrap } from '@/modules/diagnostics/services/defineService';
import type { DeviceSetupState, PairTerminalInput } from '../types';

let deviceStateCache: DeviceSetupState | null = null;
let deviceStatePromise: Promise<DeviceSetupState> | null = null;
let navigationHeldForDatabase = false;

export const getDeviceSetupState = wrap('setup.getDeviceSetupState', async function getDeviceSetupState(): Promise<DeviceSetupState> {
  return backendCall('setup_get_device_setup_state');
});

/** Re-fetches and re-caches the device state (`DeviceSetupPage` after provisioning, and
 * `deviceStateForNavigation` when a cached "no users" may be stale). */
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

/**
 * Plan 21 Part 04, E-2 (3): the device state the router guard decides on. A cached `hasUsers: false`
 * goes stale as soon as something creates the first users — the wizard's shell seed
 * (`setup_get_onboarding_progress`), a legacy import, the dev demo import — and then bounced the
 * wizard's or the import card's `/login` straight back to `/welcome`. So when the guard is about to
 * act on "no users" (`revalidateFresh`), a state that came from the cache is fetched again; one
 * fetched for this very navigation is not. One place instead of a refresh after every such write.
 */
export async function deviceStateForNavigation(revalidateFresh: boolean): Promise<DeviceSetupState> {
  const wasCached = deviceStateCache !== null;
  const state = await ensureDeviceSetupState();
  if (!wasCached || !revalidateFresh || !state.configured || state.hasUsers) return state;
  return refreshDeviceSetupState();
}

/** Sync read of the last-known device state — `authService.isFreshInstall`'s Rust branch (03-users
 * H-2) needs a synchronous answer for the router guard; before the cache is ever populated this
 * defaults to "fresh" (a fresh install shows the device-setup screen first, which populates the
 * cache before any fresh-install check runs). */
export function isFreshInstallCached(): boolean {
  return !deviceStateCache?.hasUsers;
}

/**
 * Plan 21 Part 04, E-2 (2): on a configured device, `hasUsers: false` is only an answer while the
 * database is connected — `setup_get_device_setup_state` also reports `false` when the DB is down
 * (a Main PC whose server failed, a terminal that can't reach the Main PC). Treating that as a fresh
 * install sent a real company to `/welcome` ("start company", demo) behind `ServerFailureScreen`, and
 * left it there after reconnecting. Returns `true` (and forgets the cached state) when the router
 * guard must hold the navigation instead; `App.vue` reloads once the database is back
 * (`isNavigationHeldForDatabase`). A failed status call never holds: nothing would release it.
 */
export async function mustHoldForDatabase(state: DeviceSetupState): Promise<boolean> {
  if (!state.configured || state.hasUsers) return false;
  const status = await getBackendStatus().catch(() => null);
  if (!status || status.connected) return false;
  deviceStateCache = null;
  navigationHeldForDatabase = true;
  return true;
}

/** True once the router guard has held a navigation because the database was down (see
 * `mustHoldForDatabase`) — `App.vue` then reloads the window when `useBackendHealth` reports the
 * database connected again, so the whole boot path re-runs against real data. */
export function isNavigationHeldForDatabase(): boolean {
  return navigationHeldForDatabase;
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
