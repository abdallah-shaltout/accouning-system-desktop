/**
 * L1 platform lane. `core/services/attachmentService.ts` (docs/v2/14-platform.md §5, C-16): save →
 * fetch (by owner) → fetch-by-ids → remove, plus the unknown-id read/remove no-ops. Attachments have
 * no `03-domains/` file of their own yet (they live in `core`, gated by `usesRust('core')`, not a
 * dedicated `BackendDomain`) — this case's `source` cites the seam module doc comment instead of a
 * §8(b) list, matching how `attachmentService.ts` itself is documented.
 *
 * Ids come from `attachmentService.newAttachmentId()` (Wave 2: `uid('att')` on the mock, a UUID on
 * Rust — the id the real `processFile` uses), not the mock-only `uid('att')`.
 *
 * Blobs aren't JSON-serializable (`JSON.stringify(new Blob(...))` is `"{}"`, so the harness's
 * `normalize()` would silently treat any two blobs as equal) — every step below returns the base64 of
 * a fetched blob/thumbnail explicitly instead of the raw `AttachmentRecord`, so a byte-for-byte
 * mismatch actually shows up as a diff.
 */
import { defineCase } from '../../case';
import * as attachmentService from '../../../../src/modules/core/services/attachmentService';
import type { AttachmentRecord } from '../../../../src/modules/core/services/attachmentService';

async function blobToBase64(blob: Blob): Promise<string> {
  const buf = await blob.arrayBuffer();
  return Buffer.from(buf).toString('base64');
}

async function describeRecord(record: AttachmentRecord | undefined): Promise<unknown> {
  if (!record) return null;
  const { blob, thumbnail, ...meta } = record;
  return { ...meta, blobBase64: await blobToBase64(blob), thumbnailBase64: thumbnail ? await blobToBase64(thumbnail) : null };
}

function makeRecord(id: string, ownerRef: string, bytes: number[], name: string, createdBy: string): AttachmentRecord {
  return {
    id,
    ownerRef,
    createdBy,
    name,
    mime: 'text/plain',
    kind: 'other',
    size: bytes.length,
    createdAt: '2026-06-30T09:00:00.000Z',
    blob: new Blob([new Uint8Array(bytes)], { type: 'text/plain' }),
  };
}

export default defineCase({
  name: 'attachments/attachments-lifecycle',
  source: 'src/modules/core/services/attachmentService.ts (C-16 seam module doc comment)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    const ownerRef = 'customer:cus-1';
    // The uploader, as `AttachmentField` passes it (the signed-in user). Rust stamps `created_by`
    // from the session actor server-side regardless; passing the same user keeps the mock's record
    // equal to what a real caller stores.
    const admin = s.baseId('usr-1');

    const firstId = attachmentService.newAttachmentId();
    await s.step('save-first', () => attachmentService.saveAttachment(makeRecord(firstId, ownerRef, [1, 2, 3, 4, 5], 'note.txt', admin)));

    const secondId = attachmentService.newAttachmentId();
    await s.step('save-second', () => attachmentService.saveAttachment(makeRecord(secondId, ownerRef, [9, 8, 7], 'second.txt', admin)));

    // A third attachment on a different owner — `fetchAttachments(ownerRef)` must not pick it up.
    const otherOwnerId = attachmentService.newAttachmentId();
    await s.step('save-other-owner', () => attachmentService.saveAttachment(makeRecord(otherOwnerId, 'customer:cus-2', [0], 'unrelated.txt', admin)));

    await s.step('fetch-by-owner', () => attachmentService.fetchAttachments(ownerRef));

    await s.step('fetch-first', async () => describeRecord(await attachmentService.fetchAttachment(firstId)));

    await s.step('fetch-by-ids', async () => {
      const rows = await attachmentService.fetchAttachmentsByIds([firstId, secondId]);
      return Promise.all(rows.map(describeRecord));
    });

    // Overwrite (upsert): same id, new bytes — `fetchAttachment` must reflect the new blob, not the old one.
    await s.step('overwrite-first', () => attachmentService.saveAttachment(makeRecord(firstId, ownerRef, [42, 42, 42], 'note-renamed.txt', admin)));
    await s.step('fetch-first-after-overwrite', async () => describeRecord(await attachmentService.fetchAttachment(firstId)));

    // Unknown id: a read resolves to "absent", never throws (IndexedDB `get`/Rust `find_by_id` on an
    // unparsable/missing key both resolve `None`/`undefined`).
    await s.step('fetch-unknown', async () => describeRecord(await attachmentService.fetchAttachment('att-does-not-exist')));
    await s.step('fetch-by-ids-with-unknown', async () => {
      const rows = await attachmentService.fetchAttachmentsByIds([firstId, 'att-does-not-exist']);
      return Promise.all(rows.map(describeRecord));
    });

    await s.step('remove-first', () => attachmentService.removeAttachment(firstId));
    await s.step('fetch-first-after-remove', async () => describeRecord(await attachmentService.fetchAttachment(firstId)));
    await s.step('fetch-by-owner-after-remove', () => attachmentService.fetchAttachments(ownerRef));

    // Removing an id that was never there (or already removed) is a no-op, never throws.
    await s.step('remove-unknown', () => attachmentService.removeAttachment('att-does-not-exist'));
    await s.step('remove-already-removed', () => attachmentService.removeAttachment(firstId));
  },
});
