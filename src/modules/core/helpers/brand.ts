/**
 * Single source of truth for the app's product name (docs/v2/16-equal-rebrand-and-ui-kit.md
 * Phase B). Every UI string that names the app reads from here instead of hard-coding text, so a
 * future rename touches one file.
 *
 * Deliberately NOT renamed here (see the doc): the Tauri `identifier`
 * (`com.abdallah.accounting-app`, keys the WebView2 IndexedDB data folder) and `APP_NAME` in
 * `backupArchive.ts` (old backups must keep restoring) stay as they are.
 */
export const APP_NAME_AR = 'ايكوال المحاسبي';
export const APP_NAME_EN = 'Equal Accounting';
export const APP_SHORT = 'Equal';
