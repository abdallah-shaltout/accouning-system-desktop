/**
 * Builds and parses backup archives (docs/v2/14-platform.md §4): a zip (via `fflate`) containing
 * `manifest.json`, `data.json` (the whole `MockDb` snapshot + settings + preferences) and
 * `attachments/` (every attachment blob from the Phase 0 IndexedDB blob store).
 *
 * With a password, everything except `manifest.json` is packed into one buffer and AES-GCM
 * encrypted (see `backupCrypto.ts`) before being zipped as a single `payload.enc` entry alongside
 * `crypto.json` (salt/iv/iterations — not secret, just parameters). Without a password, `data.json`
 * and `attachments/*` are stored as normal zip entries.
 */
import { unzipSync, zipSync } from 'fflate';
import { db, type MockDb } from '@/mocks/db';
import { clone } from '@/mocks/utils';
import { getAllAttachmentRecords, type AttachmentRecord } from '@/mocks/attachments';
import { SCHEMA_VERSION } from '@/mocks/persist';
import { decryptBytes, encryptBytes, sha256Hex } from './backupCrypto';
import type { BackupCrypto, BackupKind, BackupManifest } from '../types/backup';

const APP_NAME = 'accounting-app';
const APP_VERSION = '0.1.0'; // kept in sync with package.json manually (no bundler JSON import for this one string)

export interface ArchiveAttachment {
  id: string;
  meta: Omit<AttachmentRecord, 'blob' | 'thumbnail'>;
  blob: Uint8Array;
  blobType: string;
  thumbnail?: Uint8Array;
  thumbnailType?: string;
}

export interface BackupData {
  db: MockDb;
  attachments: ArchiveAttachment[];
}

async function blobToBytes(blob: Blob): Promise<Uint8Array> {
  return new Uint8Array(await blob.arrayBuffer());
}

function bytesToBlob(bytes: Uint8Array, type: string): Blob {
  return new Blob([bytes.slice().buffer], { type });
}

/** Snapshot everything the backup needs to restore: the mock DB (settings/templates/preferences live inside it) + attachment blobs. */
export async function collectBackupData(): Promise<BackupData> {
  const records = await getAllAttachmentRecords();
  const attachments: ArchiveAttachment[] = [];
  for (const r of records) {
    const { blob, thumbnail, ...meta } = r;
    attachments.push({
      id: r.id,
      meta,
      blob: await blobToBytes(blob),
      blobType: blob.type,
      thumbnail: thumbnail ? await blobToBytes(thumbnail) : undefined,
      thumbnailType: thumbnail?.type,
    });
  }
  return { db: clone(db), attachments };
}

export function tableCounts(data: BackupData): Record<string, number> {
  const counts: Record<string, number> = {};
  for (const [key, value] of Object.entries(data.db)) {
    if (Array.isArray(value)) counts[key] = value.length;
  }
  counts.attachments = data.attachments.length;
  return counts;
}

function companySlug(storeName: string | undefined): string {
  const base = (storeName ?? 'company').trim() || 'company';
  // Keep Arabic letters (filenames handle them fine on modern OSes); strip characters invalid on Windows.
  return base.replace(/[\\/:*?"<>|]+/g, '-').replace(/\s+/g, '-').slice(0, 40);
}

export function backupFileName(storeName: string | undefined, date = new Date()): string {
  const pad = (n: number) => String(n).padStart(2, '0');
  const stamp = `${date.getFullYear()}${pad(date.getMonth() + 1)}${pad(date.getDate())}-${pad(date.getHours())}${pad(date.getMinutes())}`;
  return `backup-${companySlug(storeName)}-${stamp}.zip`;
}

interface BuildResult {
  bytes: Uint8Array;
  manifest: BackupManifest;
}

/** Builds the zip's bytes. `password`, when given, encrypts everything except the manifest. */
export async function buildBackupArchive(kind: BackupKind, password?: string): Promise<BuildResult> {
  const data = await collectBackupData();
  const counts = tableCounts(data);

  const payloadFiles: Record<string, Uint8Array> = {
    'data.json': new TextEncoder().encode(JSON.stringify(data.db)),
  };
  for (const a of data.attachments) {
    payloadFiles[`attachments/${a.id}.blob`] = a.blob;
    payloadFiles[`attachments/${a.id}.json`] = new TextEncoder().encode(
      JSON.stringify({ meta: a.meta, blobType: a.blobType, thumbnailType: a.thumbnailType }),
    );
    if (a.thumbnail) payloadFiles[`attachments/${a.id}.thumb`] = a.thumbnail;
  }

  const zipEntries: Record<string, Uint8Array> = {};
  const encrypted = !!password;

  if (encrypted) {
    // Pack every payload file into one tar-like buffer (length-prefixed entries) so a single
    // AES-GCM operation covers the whole payload, then store it as one zip entry.
    const packed = packFiles(payloadFiles);
    const enc = await encryptBytes(packed, password!);
    zipEntries['payload.enc'] = enc.ciphertext;
    const cryptoInfo: BackupCrypto = { saltB64: enc.saltB64, ivB64: enc.ivB64, iterations: enc.iterations };
    zipEntries['crypto.json'] = new TextEncoder().encode(JSON.stringify(cryptoInfo));
  } else {
    Object.assign(zipEntries, payloadFiles);
  }

  // Checksum covers the (possibly encrypted) payload bytes as they sit in the zip, so "verify"
  // catches corruption regardless of whether the archive is encrypted.
  const checksumSource = encrypted ? zipEntries['payload.enc'] : concatFiles(payloadFiles);
  const checksum = await sha256Hex(checksumSource);

  const manifest: BackupManifest = {
    app: APP_NAME,
    appVersion: APP_VERSION,
    schemaVersion: SCHEMA_VERSION,
    createdAt: new Date().toISOString(),
    company: data.db.settings.storeName || 'company',
    counts,
    checksum,
    encrypted,
    kind,
  };
  zipEntries['manifest.json'] = new TextEncoder().encode(JSON.stringify(manifest, null, 2));

  const bytes = zipSync(zipEntries, { level: 6 });
  return { bytes, manifest };
}

// --- simple length-prefixed multi-file packing (used only for the encrypted payload) -----------

function packFiles(files: Record<string, Uint8Array>): Uint8Array {
  const entries = Object.entries(files);
  const header = { names: entries.map(([name, bytes]) => [name, bytes.length] as const) };
  const headerBytes = new TextEncoder().encode(JSON.stringify(header));
  const headerLen = new Uint8Array(4);
  new DataView(headerLen.buffer).setUint32(0, headerBytes.length, true);
  const total = headerLen.length + headerBytes.length + entries.reduce((acc, [, b]) => acc + b.length, 0);
  const out = new Uint8Array(total);
  let offset = 0;
  out.set(headerLen, offset);
  offset += headerLen.length;
  out.set(headerBytes, offset);
  offset += headerBytes.length;
  for (const [, bytes] of entries) {
    out.set(bytes, offset);
    offset += bytes.length;
  }
  return out;
}

function unpackFiles(packed: Uint8Array): Record<string, Uint8Array> {
  const headerLen = new DataView(packed.buffer, packed.byteOffset, 4).getUint32(0, true);
  const headerBytes = packed.slice(4, 4 + headerLen);
  const header = JSON.parse(new TextDecoder().decode(headerBytes)) as { names: [string, number][] };
  let offset = 4 + headerLen;
  const out: Record<string, Uint8Array> = {};
  for (const [name, len] of header.names) {
    out[name] = packed.slice(offset, offset + len);
    offset += len;
  }
  return out;
}

/** Concatenation used only to compute the unencrypted checksum, in a stable (sorted) key order. */
function concatFiles(files: Record<string, Uint8Array>): Uint8Array {
  const keys = Object.keys(files).sort();
  const total = keys.reduce((acc, k) => acc + files[k].length, 0);
  const out = new Uint8Array(total);
  let offset = 0;
  for (const k of keys) {
    out.set(files[k], offset);
    offset += files[k].length;
  }
  return out;
}

// --- reading -------------------------------------------------------------------------------------

export interface ParsedArchive {
  manifest: BackupManifest;
  /** Present only when `manifest.encrypted` is false — call `decryptArchive` otherwise. */
  data?: BackupData;
  encryptedPayload?: Uint8Array;
  crypto?: BackupCrypto;
}

/** Reads `manifest.json` (always plaintext) without decrypting anything else. */
export function readManifest(bytes: Uint8Array): BackupManifest {
  const files = unzipSync(bytes, { filter: (f) => f.name === 'manifest.json' });
  const raw = files['manifest.json'];
  if (!raw) throw new Error('ملف النسخة الاحتياطية غير صالح: manifest.json مفقود');
  return JSON.parse(new TextDecoder().decode(raw)) as BackupManifest;
}

/** Full parse: for an unencrypted archive, decodes `data.json` + attachments right away. */
export function parseArchive(bytes: Uint8Array): ParsedArchive {
  const files = unzipSync(bytes);
  const manifestRaw = files['manifest.json'];
  if (!manifestRaw) throw new Error('ملف النسخة الاحتياطية غير صالح: manifest.json مفقود');
  const manifest = JSON.parse(new TextDecoder().decode(manifestRaw)) as BackupManifest;

  if (manifest.encrypted) {
    const encryptedPayload = files['payload.enc'];
    const cryptoRaw = files['crypto.json'];
    if (!encryptedPayload || !cryptoRaw) throw new Error('ملف النسخة الاحتياطية مشفّر لكن بياناته مفقودة');
    const crypto = JSON.parse(new TextDecoder().decode(cryptoRaw)) as BackupCrypto;
    return { manifest, encryptedPayload, crypto };
  }

  const data = decodePayloadFiles(files);
  return { manifest, data };
}

/** Decrypts an encrypted archive's payload with the given password, returning the same shape as an unencrypted parse. */
export async function decryptArchive(parsed: ParsedArchive, password: string): Promise<BackupData> {
  if (!parsed.encryptedPayload || !parsed.crypto) throw new Error('لا يوجد محتوى مشفّر لفك تشفيره');
  const packed = await decryptBytes(parsed.encryptedPayload, password, parsed.crypto.saltB64, parsed.crypto.ivB64, parsed.crypto.iterations);
  const files = unpackFiles(packed);
  return decodePayloadFiles(files);
}

function decodePayloadFiles(files: Record<string, Uint8Array>): BackupData {
  const dataRaw = files['data.json'];
  if (!dataRaw) throw new Error('ملف النسخة الاحتياطية غير صالح: data.json مفقود');
  const dbData = JSON.parse(new TextDecoder().decode(dataRaw)) as MockDb;

  const attachments: ArchiveAttachment[] = [];
  const ids = new Set<string>();
  for (const name of Object.keys(files)) {
    const m = /^attachments\/(.+)\.json$/.exec(name);
    if (m) ids.add(m[1]);
  }
  for (const id of ids) {
    const metaRaw = files[`attachments/${id}.json`];
    const blob = files[`attachments/${id}.blob`];
    if (!metaRaw || !blob) continue;
    const { meta, blobType, thumbnailType } = JSON.parse(new TextDecoder().decode(metaRaw)) as {
      meta: Omit<AttachmentRecord, 'blob' | 'thumbnail'>;
      blobType: string;
      thumbnailType?: string;
    };
    const thumbnail = files[`attachments/${id}.thumb`];
    attachments.push({ id, meta, blob, blobType, thumbnail, thumbnailType });
  }

  return { db: dbData, attachments };
}

export function archiveAttachmentToRecord(a: ArchiveAttachment): AttachmentRecord {
  return {
    ...a.meta,
    blob: bytesToBlob(a.blob, a.blobType),
    thumbnail: a.thumbnail ? bytesToBlob(a.thumbnail, a.thumbnailType ?? 'image/webp') : undefined,
  };
}
