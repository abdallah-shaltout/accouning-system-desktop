/**
 * L1 platform lane. The full "نسخة الآن" → "استعادة" round trip (17-backup.md §3.6/§3.8/§3.9) over a
 * real browser-format archive built by the actual `backupService.backupNow`/`restoreFromArchive`.
 *
 * Restored to the real end-to-end flow now that `scripts/parity/polyfills.ts` gives this harness a
 * real `indexedDB` (attachments) and a `document` stub for `saveFile.ts`'s `browserDownload()` —
 * `backupNow`'s mock path unconditionally reads `getAllAttachmentRecords()` then (browser mode, which
 * this harness always is: `isTauri()` is false under Bun) calls `saveFile`, which used to throw with
 * no DOM; `restoreFromArchive` itself also calls `flushSnapshot()`/`replaceAllAttachments` (more
 * IndexedDB) and, before replacing anything, an automatic pre-restore `backupNow('pre-restore')` —
 * see the polyfill's own header comment for the harness-gap history.
 *
 * Wave 2: the archives are built from the **base snapshot** (`loadBase('demo-sa')`), not the live mock
 * `db` — on the Rust pass the frontend `db` is empty (the data lives in MariaDB), so an archive built
 * from `db` there was a blank company ("لا يوجد فرع في البيانات المستوردة"). Both passes now restore the
 * exact same bytes. After the restore the session is gone on Rust (17-backup D-11: restore ends every
 * session; the modal reloads the app), so the case signs in again, as the user would. A browser-format
 * archive restores through the D10 importer (17-backup D-3), which gives every row a fresh UUID — so
 * ids read after the restore (steps and books) are compared as "some id", not as the base's ids.
 */
import { defineCase } from '../../case';
import type { MockDb } from '../../../../src/mocks/db';
import { clone } from '../../../../src/mocks/utils';
import { loadBase } from '../../bases';
import { SCHEMA_VERSION } from '../../../../src/mocks/persist';
import { buildBackupArchive } from '../../../../src/modules/settings/helpers/backupArchive';
import * as backupService from '../../../../src/modules/settings/services/backupService';
import * as partyService from '../../../../src/modules/parties/services/partyService';

export default defineCase({
  name: 'backup/backup-restore-browser-archive',
  source: '03-domains/17-backup.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  allow: [
    { path: 'steps.preview-compatible.value.compatibilityNote', reason: '17-backup D-3: a browser archive always restores through the importer, so Rust previews it with the "will be upgraded" note (§3.8)' },
    { path: 'steps.preview-encrypted.value.compatibilityNote', reason: '17-backup D-3: a browser archive always restores through the importer, so Rust previews it with the "will be upgraded" note (§3.8)' },
    { path: 'steps.login:admin#2.value.id', reason: '17-backup D-3: users re-keyed by the importer on restore' },
    { path: 'steps.customers-after-restore.value[].id', reason: '17-backup D-3: a browser archive restores through the D10 importer, which assigns fresh UUIDs' },
    { path: 'steps.customers-after-restore.value[].groupId', reason: '17-backup D-3: re-keyed by the importer on restore' },
    { path: 'steps.customers-after-restore.value[].phones[].id', reason: '17-backup D-3: re-keyed by the importer on restore' },
    { path: 'books.trialBalance.value[].accountId', reason: '17-backup D-3: accounts re-keyed by the importer on restore' },
    { path: 'books.customers.value[].id', reason: '17-backup D-3: re-keyed by the importer on restore' },
    { path: 'books.customers.value[].groupId', reason: '17-backup D-3: re-keyed by the importer on restore' },
    { path: 'books.customers.value[].phones[].id', reason: '17-backup D-3: re-keyed by the importer on restore' },
    { path: 'books.suppliers.value[].id', reason: '17-backup D-3: re-keyed by the importer on restore' },
    { path: 'books.suppliers.value[].groupId', reason: '17-backup D-3: re-keyed by the importer on restore' },
    { path: 'books.suppliers.value[].phones[].id', reason: '17-backup D-3: re-keyed by the importer on restore' },
    { path: 'books.inventory.value[].productId', reason: '17-backup D-3: products re-keyed by the importer on restore' },
  ],
  async run(s) {
    const baseData = (): MockDb => clone(loadBase('demo-sa', 'backup/backup-restore-browser-archive').snapshot!.data as MockDb);
    const plain = await buildBackupArchive('manual', baseData(), [], SCHEMA_VERSION);
    await s.step('preview-compatible', () => backupService.previewRestore(plain.bytes));

    // The manifest's `checksum` covers the encrypted payload, which is salted/IV'd from
    // `crypto.getRandomValues` (`backupCrypto.ts:44-45`, 17-backup.md §3.1's own "salt/IV from
    // getrandom" note) — genuinely random on every call, not seeded by the pinned-clock PRNG
    // (`pass.ts` only seeds `Math.random`, not WebCrypto). Compared field-by-field, skipping `checksum`.
    const encrypted = await buildBackupArchive('manual', baseData(), [], SCHEMA_VERSION, 'p@ssw0rd');
    await s.step('preview-encrypted', async () => {
      const preview = await backupService.previewRestore(encrypted.bytes);
      const { checksum: _checksum, ...manifestRest } = preview.manifest;
      return { ...preview, manifest: manifestRest };
    });

    // A newer-than-current schema version: same bytes, patched manifest — the "update the app first" note.
    const newer = await buildBackupArchive('manual', baseData(), [], SCHEMA_VERSION + 5);
    await s.step('preview-newer-version', () => backupService.previewRestore(newer.bytes));

    // A corrupted archive (no manifest.json at all).
    await s.expectError('preview-invalid-bytes', () => backupService.previewRestore(new TextEncoder().encode('not a zip')));

    // The real round trip: back up now (browser mode — a "download" through the harness's `document`
    // stub, plus a rolling IndexedDB history entry), then restore that exact archive. `backupNow`
    // doesn't hand back the raw bytes in browser mode, so the restore step reuses `plain` (byte-for-
    // byte the same shape `backupNow` would have produced from this same `db` at this same instant).
    const before = await s.step('customers-before', () => partyService.getCustomers());
    // 17-backup D-3: on Rust "نسخة الآن" builds the Rust archive format (`schemaVersion ≥ 100`, a
    // table dump), on the mock the browser zip — so the format version, checksum and byte size
    // differ by design, and `path` only exists where a native save ran. Everything else in the
    // manifest (kind, company, business-clock `createdAt`, encrypted, per-table counts — equal by
    // construction, 17-backup Q-3) is compared.
    const backup = await s.step('backup-now', async () => {
      const r = await backupService.backupNow('manual');
      const { schemaVersion: _v, checksum: _c, ...manifest } = r.manifest;
      return { manifest, cancelled: r.cancelled ?? false };
    });
    await s.step('restore', () => backupService.restoreFromArchive(plain.bytes));
    // 17-backup D-11: the restore ended the session on Rust (the app reloads to the login screen).
    await s.login('admin');
    const after = await s.step('customers-after-restore', () => partyService.getCustomers());
    await s.step('backup-settings-after', () => backupService.backupSettings());
    return void (before && backup && after);
  },
});
