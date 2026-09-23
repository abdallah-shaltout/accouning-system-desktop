/**
 * IndexedDB blob storage for `AttachmentField` (docs/v2/14-platform.md §5). Separate object store
 * from `persist.ts`'s DB snapshot store — snapshots are cloned/JSON-friendly table data, while
 * attachment blobs are large binary payloads that should never ride along with every 500ms
 * snapshot write.
 *
 * Same database (`mock-db`) as `persist.ts`, but a bumped IDB version so both stores are created
 * on the same `onupgradeneeded`. `persist.ts` opens the DB with version 1 (its original store
 * only); this module opens it with version 2, which also creates the `attachments` store. Whichever
 * module opens the connection first during a session triggers the upgrade — IndexedDB runs
 * `onupgradeneeded` once per version bump regardless of which caller's `open()` call arrives first.
 */

const DB_NAME = 'mock-db';
const DB_VERSION = 2;
const STORE_NAME = 'attachments';

export type AttachmentKind = 'image' | 'pdf' | 'office' | 'other';

export interface AttachmentMeta {
  id: string;
  /** The record this file is attached to, e.g. `customer:cus-3`, `journal:je-12`. Optional for demo/unowned files. */
  ownerRef?: string;
  name: string;
  mime: string;
  kind: AttachmentKind;
  /** Bytes of the stored (possibly resized/re-encoded) file. */
  size: number;
  width?: number;
  height?: number;
  createdAt: string;
  createdBy?: string;
}

export interface AttachmentRecord extends AttachmentMeta {
  blob: Blob;
  /** Small WebP preview for images only. */
  thumbnail?: Blob;
}

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = () => {
      if (!req.result.objectStoreNames.contains('snapshot')) req.result.createObjectStore('snapshot');
      if (!req.result.objectStoreNames.contains(STORE_NAME)) req.result.createObjectStore(STORE_NAME, { keyPath: 'id' });
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

export async function putAttachment(record: AttachmentRecord): Promise<void> {
  const conn = await openDb();
  return new Promise((resolve, reject) => {
    const tx = conn.transaction(STORE_NAME, 'readwrite');
    tx.objectStore(STORE_NAME).put(record);
    tx.oncomplete = () => {
      conn.close();
      resolve();
    };
    tx.onerror = () => reject(tx.error);
  });
}

export async function getAttachment(id: string): Promise<AttachmentRecord | undefined> {
  const conn = await openDb();
  return new Promise((resolve, reject) => {
    const tx = conn.transaction(STORE_NAME, 'readonly');
    const req = tx.objectStore(STORE_NAME).get(id);
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
    tx.oncomplete = () => conn.close();
  });
}

export async function deleteAttachment(id: string): Promise<void> {
  const conn = await openDb();
  return new Promise((resolve, reject) => {
    const tx = conn.transaction(STORE_NAME, 'readwrite');
    tx.objectStore(STORE_NAME).delete(id);
    tx.oncomplete = () => {
      conn.close();
      resolve();
    };
    tx.onerror = () => reject(tx.error);
  });
}

/** All attachments for a given owner ref, newest first. Used by pages that list what's attached to a record. */
export async function listAttachments(ownerRef: string): Promise<AttachmentMeta[]> {
  const conn = await openDb();
  return new Promise((resolve, reject) => {
    const tx = conn.transaction(STORE_NAME, 'readonly');
    const req = tx.objectStore(STORE_NAME).getAll();
    req.onsuccess = () => {
      const all = (req.result as AttachmentRecord[])
        .filter((r) => r.ownerRef === ownerRef)
        .sort((a, b) => b.createdAt.localeCompare(a.createdAt))
        .map(({ blob: _blob, thumbnail: _thumbnail, ...meta }) => meta);
      resolve(all);
    };
    req.onerror = () => reject(req.error);
    tx.oncomplete = () => conn.close();
  });
}

/**
 * Every attachment record (blob + thumbnail included) — used by the Phase 13a backup archiver to
 * pack `attachments/` into the zip, and by restore to repopulate this store.
 */
export async function getAllAttachmentRecords(): Promise<AttachmentRecord[]> {
  const conn = await openDb();
  return new Promise((resolve, reject) => {
    const tx = conn.transaction(STORE_NAME, 'readonly');
    const req = tx.objectStore(STORE_NAME).getAll();
    req.onsuccess = () => resolve(req.result as AttachmentRecord[]);
    req.onerror = () => reject(req.error);
    tx.oncomplete = () => conn.close();
  });
}

/** Replaces the entire attachments store with `records` — used by restore. */
export async function replaceAllAttachments(records: AttachmentRecord[]): Promise<void> {
  const conn = await openDb();
  return new Promise((resolve, reject) => {
    const tx = conn.transaction(STORE_NAME, 'readwrite');
    const store = tx.objectStore(STORE_NAME);
    store.clear();
    for (const record of records) store.put(record);
    tx.oncomplete = () => {
      conn.close();
      resolve();
    };
    tx.onerror = () => reject(tx.error);
  });
}
