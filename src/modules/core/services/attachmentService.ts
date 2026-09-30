/**
 * Seam wrapper around `src/mocks/attachments.ts`'s IndexedDB blob store (docs/v2/14-platform.md §5).
 * `AttachmentField`, `ProductImageGallery` and `helpers/attachments.ts` all need the same CRUD +
 * id-generation primitives — centralized here instead of importing `@/mocks` from components.
 *
 * C-16 (plans/pending/21-rust-backend/02-CORE-AND-SHARED-ARCHITECTURE.md): attachment blobs move
 * from the browser's per-device IndexedDB into MariaDB (`attachments` domain,
 * `src-tauri/src/domains/attachments/`) so every terminal sharing the Main-PC DB sees a file
 * attached on another terminal, and so backups include it. `usesRust('core')` (attachments has no
 * dedicated `BackendDomain` entry — it lives in `core`, same module `attachmentService.ts` itself
 * belongs to) switches each function to `backendCall`; the mock IndexedDB body is unchanged below
 * it. Blobs cross IPC as standard base64 strings (the `pdf_base64` precedent) — decoded/encoded
 * here so callers keep working with `Blob`s exactly as the mock's `AttachmentRecord` shape does.
 */
import { uid } from '@/mocks';
import {
  deleteAttachment,
  getAttachment,
  listAttachments,
  putAttachment,
  type AttachmentKind,
  type AttachmentMeta,
  type AttachmentRecord,
} from '@/mocks/attachments';

import { backendCall, usesRust } from '@/modules/core/services/backend';
import { wrap } from '@/modules/diagnostics/services/defineService';

export type { AttachmentKind, AttachmentMeta, AttachmentRecord };
export { uid };

function base64ToBlob(base64: string, type: string): Blob {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return new Blob([bytes], { type });
}

async function blobToBase64(blob: Blob): Promise<string> {
  const buf = await blob.arrayBuffer();
  const bytes = new Uint8Array(buf);
  let binary = '';
  for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
  return btoa(binary);
}

/** Maps the Rust `AttachmentMeta` DTO (camelCase, `size`/`width`/`height` as `number`) onto the
 * mock's own `AttachmentMeta` shape — the two are already field-for-field identical. */
function metaFromDto(dto: import('@/modules/core/types/gen/AttachmentMeta').AttachmentMeta): AttachmentMeta {
  return {
    id: dto.id,
    ownerRef: dto.ownerRef ?? undefined,
    name: dto.name,
    mime: dto.mime,
    kind: dto.kind,
    size: dto.size,
    width: dto.width ?? undefined,
    height: dto.height ?? undefined,
    createdAt: dto.createdAt,
    createdBy: dto.createdBy ?? undefined,
  };
}

async function recordFromDto(dto: import('@/modules/core/types/gen/AttachmentRecord').AttachmentRecord): Promise<AttachmentRecord> {
  return {
    ...metaFromDto(dto),
    blob: base64ToBlob(dto.blobBase64, dto.mime),
    thumbnail: dto.thumbnailBase64 ? base64ToBlob(dto.thumbnailBase64, 'image/webp') : undefined,
  };
}

/**
 * A fresh id for a new attachment (the caller keeps it in the owning document's `attachmentIds`).
 * Rust stores attachments under UUID keys and survives restarts, so the id must be a UUID there — the
 * mock's `uid('att')` counter restarts at 1 on every launch when nothing resyncs it (Rust mode has no
 * mock `db` to resync from) and would overwrite an older attachment, and a non-UUID id is not a key
 * Rust can store (plan 21 Part 04 Wave 2, L1 `attachments/attachments-lifecycle`). Sync on purpose:
 * `processFile` builds the record before any await.
 */
export function newAttachmentId(): string {
  if (usesRust('core')) return crypto.randomUUID();
  return uid('att');
}

export const fetchAttachments = wrap('core.fetchAttachments', async function fetchAttachments(ownerRef: string): Promise<AttachmentMeta[]> {
  if (usesRust('core')) {
    const rows = await backendCall('attachments_fetch_attachments', { ownerRef });
    return rows.map(metaFromDto);
  }
  return listAttachments(ownerRef);
});

export const fetchAttachment = wrap('core.fetchAttachment', async function fetchAttachment(id: string): Promise<AttachmentRecord | undefined> {
  if (usesRust('core')) {
    const dto = await backendCall('attachments_fetch_attachment', { id });
    return dto ? recordFromDto(dto) : undefined;
  }
  return getAttachment(id);
});

/** Batch lookup by id list — new (no mock caller yet; every other domain's `attachmentIds: string[]`
 * field will want it once it stops being an opaque id list). Falls back to N individual
 * `getAttachment` calls on the mock, since IndexedDB has no native "get many" primitive. */
export const fetchAttachmentsByIds = wrap('core.fetchAttachmentsByIds', async function fetchAttachmentsByIds(ids: string[]): Promise<AttachmentRecord[]> {
  if (usesRust('core')) {
    const rows = await backendCall('attachments_fetch_attachments_by_ids', { ids });
    return Promise.all(rows.map(recordFromDto));
  }
  const found: AttachmentRecord[] = [];
  for (const id of ids) {
    const record = await getAttachment(id);
    if (record) found.push(record);
  }
  return found;
});

export const saveAttachment = wrap('core.saveAttachment', async function saveAttachment(record: AttachmentRecord): Promise<void> {
  if (usesRust('core')) {
    await backendCall('attachments_save_attachment', {
      id: record.id,
      ownerRef: record.ownerRef,
      name: record.name,
      mime: record.mime,
      kind: record.kind,
      width: record.width,
      height: record.height,
      blobBase64: await blobToBase64(record.blob),
      thumbnailBase64: record.thumbnail ? await blobToBase64(record.thumbnail) : undefined,
    });
    return;
  }
  return putAttachment(record);
});

export const removeAttachment = wrap('core.removeAttachment', async function removeAttachment(id: string): Promise<void> {
  if (usesRust('core')) {
    await backendCall('attachments_remove_attachment', { id });
    return;
  }
  return deleteAttachment(id);
});
