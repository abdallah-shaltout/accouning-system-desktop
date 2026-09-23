/**
 * Client-side file processing for `AttachmentField` (docs/v2/14-platform.md §5): type/size
 * validation, image resize + WebP re-encode via canvas, and thumbnail generation.
 */
import { uid } from '@/mocks';
import type { AttachmentKind, AttachmentRecord } from '@/mocks/attachments';

/** Max upload size. A setting in the spec's language — exported as a const until Settings grows a UI for it. */
export const MAX_ATTACHMENT_SIZE = 10 * 1024 * 1024; // 10MB

/** Longest edge images are resized to before re-encoding as WebP. */
export const IMAGE_MAX_DIMENSION = 2000;
export const IMAGE_QUALITY = 0.85;
export const THUMBNAIL_MAX_DIMENSION = 240;

const IMAGE_TYPES = ['image/png', 'image/jpeg', 'image/webp', 'image/gif', 'image/svg+xml', 'image/bmp'];
const PDF_TYPES = ['application/pdf'];
const OFFICE_TYPES = [
  'application/msword',
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  'application/vnd.ms-excel',
  'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
  'application/vnd.ms-powerpoint',
  'application/vnd.openxmlformats-officedocument.presentationml.presentation',
];

export const ACCEPTED_TYPES = [...IMAGE_TYPES, ...PDF_TYPES, ...OFFICE_TYPES];

/** Accept attribute value for a hidden file input. */
export const ACCEPT_ATTR = [...ACCEPTED_TYPES, '.doc', '.docx', '.xls', '.xlsx', '.ppt', '.pptx'].join(',');

export function kindOf(mime: string): AttachmentKind {
  if (IMAGE_TYPES.includes(mime)) return 'image';
  if (PDF_TYPES.includes(mime)) return 'pdf';
  if (OFFICE_TYPES.includes(mime)) return 'office';
  return 'other';
}

export function isAcceptedType(mime: string, name: string): boolean {
  if (ACCEPTED_TYPES.includes(mime)) return true;
  // Some browsers/OSes report an empty or generic mime for Office files picked via drag-drop.
  return /\.(docx?|xlsx?|pptx?)$/i.test(name);
}

export class AttachmentError extends Error {}

function loadImage(blob: Blob): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(blob);
    const img = new Image();
    img.onload = () => {
      URL.revokeObjectURL(url);
      resolve(img);
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new AttachmentError('تعذرت قراءة الصورة'));
    };
    img.src = url;
  });
}

function canvasToBlob(canvas: HTMLCanvasElement, type: string, quality?: number): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => (blob ? resolve(blob) : reject(new AttachmentError('تعذر معالجة الصورة'))), type, quality);
  });
}

function scaledSize(width: number, height: number, maxDimension: number): { width: number; height: number } {
  const longest = Math.max(width, height);
  if (longest <= maxDimension) return { width, height };
  const scale = maxDimension / longest;
  return { width: Math.round(width * scale), height: Math.round(height * scale) };
}

async function resizeToWebp(img: HTMLImageElement, maxDimension: number, quality: number): Promise<{ blob: Blob; width: number; height: number }> {
  const { width, height } = scaledSize(img.naturalWidth, img.naturalHeight, maxDimension);
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new AttachmentError('تعذر معالجة الصورة');
  ctx.drawImage(img, 0, 0, width, height);
  const blob = await canvasToBlob(canvas, 'image/webp', quality);
  return { blob, width, height };
}

/**
 * Validates and processes a picked/dropped/pasted file into a storable `AttachmentRecord`.
 * Images are resized (longest edge `IMAGE_MAX_DIMENSION`) and re-encoded as WebP with a thumbnail;
 * everything else (PDF, Office, other) is stored as-is.
 */
export async function processFile(file: File, ownerRef: string | undefined, createdBy?: string): Promise<AttachmentRecord> {
  if (file.size > MAX_ATTACHMENT_SIZE) {
    throw new AttachmentError(`الملف أكبر من الحد المسموح (${Math.round(MAX_ATTACHMENT_SIZE / 1024 / 1024)} ميجابايت)`);
  }
  if (!isAcceptedType(file.type, file.name)) {
    throw new AttachmentError('نوع الملف غير مدعوم (صور، PDF، أو ملفات Office فقط)');
  }

  const kind = kindOf(file.type);
  const id = uid('att');
  const createdAt = new Date().toISOString();

  if (kind === 'image' && file.type !== 'image/svg+xml') {
    const img = await loadImage(file);
    const main = await resizeToWebp(img, IMAGE_MAX_DIMENSION, IMAGE_QUALITY);
    const thumb = await resizeToWebp(img, THUMBNAIL_MAX_DIMENSION, IMAGE_QUALITY);
    return {
      id,
      ownerRef,
      name: file.name.replace(/\.\w+$/, '.webp'),
      mime: 'image/webp',
      kind,
      size: main.blob.size,
      width: main.width,
      height: main.height,
      createdAt,
      createdBy,
      blob: main.blob,
      thumbnail: thumb.blob,
    };
  }

  return { id, ownerRef, name: file.name, mime: file.type || 'application/octet-stream', kind, size: file.size, createdAt, createdBy, blob: file };
}

export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} بايت`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} كيلوبايت`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} ميجابايت`;
}
