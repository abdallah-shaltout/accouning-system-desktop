/**
 * Seam wrapper around `src/mocks/attachments.ts`'s IndexedDB blob store (docs/v2/14-platform.md §5).
 * `AttachmentField`, `ProductImageGallery` and `helpers/attachments.ts` all need the same CRUD +
 * id-generation primitives — centralized here instead of importing `@/mocks` from components.
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

export type { AttachmentKind, AttachmentMeta, AttachmentRecord };
export { uid };

export async function fetchAttachments(ownerRef: string): Promise<AttachmentMeta[]> {
  return listAttachments(ownerRef);
}

export async function fetchAttachment(id: string): Promise<AttachmentRecord | undefined> {
  return getAttachment(id);
}

export async function saveAttachment(record: AttachmentRecord): Promise<void> {
  return putAttachment(record);
}

export async function removeAttachment(id: string): Promise<void> {
  return deleteAttachment(id);
}
