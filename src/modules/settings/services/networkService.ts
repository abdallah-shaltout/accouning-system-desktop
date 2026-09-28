/**
 * LAN sharing / pairing service (21 · 03.01 §6 "New (dormant until `usesRust('settings')`)").
 * No mock backend equivalent — there is nothing to toggle in browser/mock mode, so every function
 * throws outside Tauri + `usesRust('settings')`, matching the plan's exact wording.
 */
import { ApiError } from '@/mocks';
import type { LanSharingStatus, PairingInfo } from '../types/network';

import { backendCall, usesRust } from '@/modules/core/services/backend';
import { wrap } from '@/modules/diagnostics/services/defineService';

function requireRust(): void {
  if (!usesRust('settings')) throw new ApiError('متاح في نسخة سطح المكتب فقط', 'FORBIDDEN');
}

export const getLanSharingStatus = wrap('settings.getLanSharingStatus', async function getLanSharingStatus(): Promise<LanSharingStatus> {
  requireRust();
  return backendCall('settings_get_lan_sharing_status');
});

export const enableLanSharing = wrap('settings.enableLanSharing', async function enableLanSharing(): Promise<PairingInfo> {
  requireRust();
  return backendCall('settings_enable_lan_sharing');
});

export const disableLanSharing = wrap('settings.disableLanSharing', async function disableLanSharing(confirmDisconnect: boolean): Promise<LanSharingStatus> {
  requireRust();
  return backendCall('settings_disable_lan_sharing', { confirmDisconnect });
});

export const rotatePairingCode = wrap('settings.rotatePairingCode', async function rotatePairingCode(): Promise<PairingInfo> {
  requireRust();
  return backendCall('settings_rotate_pairing_code');
});

export const reconnectBackend = wrap('settings.reconnectBackend', async function reconnectBackend(): Promise<void> {
  requireRust();
  await backendCall('settings_reconnect_backend');
});
