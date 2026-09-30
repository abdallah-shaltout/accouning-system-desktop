/**
 * L1 platform lane. The 10MB attachment size refusal — `src-tauri/src/domains/attachments/service.rs`
 * `save_attachment`'s `MAX_ATTACHMENT_SIZE` check (server-side, "never trust the frontend alone" per
 * CLAUDE.md), mirroring `src/modules/core/helpers/attachments.ts`'s `MAX_ATTACHMENT_SIZE` (10MB) —
 * but that check lives in the **client-side** `processFile()` helper (needs `File`/canvas/`Image`,
 * never reachable from a headless service call), not in `attachmentService.saveAttachment` itself.
 *
 * **Update (Wave 2): closed** — the mock's `saveAttachment` now refuses `size > 10MB` with the same
 * message, so the step below is an `expectError` on both passes. Original Wave 1 note kept below.
 *
 * **Known cross-backend gap, not fixed here (CLAUDE.md "report, don't fix" / P4-7 needs a decision):**
 * the mock's `putAttachment` (`src/mocks/attachments.ts`) has **no** size check at all — it stores
 * whatever `AttachmentRecord` it's given, so calling `attachmentService.saveAttachment` directly (as
 * every service-level caller, including this harness, does) with an over-size blob succeeds silently
 * on the mock. The equivalent Rust command refuses it with a `VALIDATION` `الملف أكبر من الحد المسموح
 * (10 ميجابايت)`. This case pins the **mock's actual behaviour** (succeeds) rather than routing around
 * it or guessing the eventual decision — once a Rust pass runs this case it will show a real diff
 * (`ok: true` vs `ok: false` on the same step) that is not a harness gap; that diff is the signal for
 * whoever owns this file next to decide (a) add the same guard to `saveAttachment`'s mock body (a
 * `BUG-`/`ACC-`-style fix, `verify:mocks`-safe since no accounting number is involved) or (b) allow it
 * with a decision id. Not something this wave's `--mock-only` gate can catch on its own.
 */
import { defineCase } from '../../case';
import * as attachmentService from '../../../../src/modules/core/services/attachmentService';
import type { AttachmentRecord } from '../../../../src/modules/core/services/attachmentService';

const TEN_MB = 10 * 1024 * 1024;

function bigRecord(id: string, size: number): AttachmentRecord {
  return {
    id,
    ownerRef: 'customer:cus-1',
    name: 'big.bin',
    mime: 'application/octet-stream',
    kind: 'other',
    size,
    createdAt: '2026-06-30T09:00:00.000Z',
    blob: new Blob([new Uint8Array(size)], { type: 'application/octet-stream' }),
  };
}

export default defineCase({
  name: 'attachments/attachments-size-refusal',
  source: 'src-tauri/src/domains/attachments/service.rs MAX_ATTACHMENT_SIZE (10MB, server-side only)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    const okId = attachmentService.newAttachmentId();
    // Exactly at the limit: allowed on both backends (Rust's check is strictly `>`).
    await s.step('save-exactly-10mb', () => attachmentService.saveAttachment(bigRecord(okId, TEN_MB)));

    const overId = attachmentService.newAttachmentId();
    // One byte over: refused on both backends — the mock's `saveAttachment` now carries the same
    // `MAX_ATTACHMENT_SIZE` guard as Rust's `save_attachment` (the gap the header describes is closed),
    // so this is an `expectError` pinning the byte-exact VALIDATION message on both sides.
    await s.expectError('save-over-10mb', () => attachmentService.saveAttachment(bigRecord(overId, TEN_MB + 1)));
  },
});
