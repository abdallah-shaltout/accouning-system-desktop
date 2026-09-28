/**
 * LAN sharing / pairing (21 · 03.01 §2, Part 02 handoff §9 — the Main-PC-hosted MariaDB toggle,
 * pairing-code display and server-failure screen). New in this phase; dormant on the mock (no
 * Tauri backend to toggle) until `usesRust('settings')` — see `networkService.ts`.
 */

export interface PairingInfo {
  hostName: string;
  addresses: string[];
  port: number;
  code: string;
}

export interface LanSharingStatus {
  role: 'main' | 'terminal';
  provisioned: boolean;
  lanSharing: boolean;
  connectedTerminals: number;
  pairing?: PairingInfo;
  /** Terminal role only: the Main PC's host/IP this terminal is paired to. */
  mainHost?: string;
}
