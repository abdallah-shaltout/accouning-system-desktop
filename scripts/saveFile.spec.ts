/**
 * Pins `saveFile()`'s Tauri-vs-browser branch (modules/core/services/saveFile.ts) against
 * `@tauri-apps/api/core`'s `isTauri()`, which reads `globalThis.isTauri` (set by the Tauri
 * webview at runtime — see node_modules/@tauri-apps/api/core.js). Same plain bun-executable
 * assertion-script convention as scripts/totals.spec.ts — no test runner is configured.
 *
 *   bun run scripts/saveFile.spec.ts
 */
import { isTauri } from '@tauri-apps/api/core';

let passed = 0;
let failed = 0;

function ok(cond: boolean, label: string) {
  if (cond) {
    passed++;
  } else {
    failed++;
    console.error(`FAIL  ${label}`);
  }
}

// --- isTauri() reflects the runtime flag saveFile.ts branches on ------------------------------

ok(isTauri() === false, 'isTauri() is false with no Tauri runtime present (browser/e2e fallback path)');

(globalThis as any).isTauri = true;
ok(isTauri() === true, 'isTauri() is true once the Tauri webview sets globalThis.isTauri (native Save-dialog path)');
delete (globalThis as any).isTauri;

ok(isTauri() === false, 'isTauri() goes back to false once the flag is removed');

// --- saveFile.ts's own dynamic-import branch is gated by the same isTauri() call --------------
// (a full saveFile() call needs @tauri-apps/plugin-dialog's real `save()`, which shows a native
// OS dialog and can't run headlessly — so this pins the same predicate the module branches on
// rather than invoking the plugin, matching the "unit check" scope in docs/v2/17 Phase C.)
const source = await Bun.file('src/modules/core/services/saveFile.ts').text();
ok(source.includes('if (!isTauri())'), 'saveFile() branches on isTauri() before choosing the save path');

console.log(`\n${passed} passed, ${failed} failed`);
if (failed > 0) process.exit(1);
