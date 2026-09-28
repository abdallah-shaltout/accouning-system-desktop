/**
 * Plan 22: turn a rendered mobile invoice template into a PNG — for WhatsApp and the like.
 *
 * `modern-screenshot` draws the element through the browser's own renderer (an SVG `foreignObject`),
 * so joined Arabic letters, fonts and gradients come out exactly as on screen — canvas re-implementers
 * such as html2canvas break Arabic shaping. Saving goes through the shared `saveFile` helper (native
 * Save dialog on desktop, rule 21); copying puts the PNG on the clipboard for pasting into a chat.
 */
import { saveFile } from '@/modules/core/services/saveFile';
import { wrap } from '@/modules/diagnostics/services/defineService';

/** 3× a 400 px phone layout → a 1200 px wide image: sharp on any phone, still small to send. */
const PIXEL_RATIO = 3;

async function renderPng(element: HTMLElement): Promise<Blob> {
  await document.fonts.ready;
  const { domToBlob } = await import('modern-screenshot');
  return domToBlob(element, { scale: PIXEL_RATIO, type: 'image/png' });
}

/** Saves the invoice image; resolves to the saved path / `true` (browser), or `null` when the user cancels the dialog. */
export const saveInvoiceImage = wrap('invoices.saveInvoiceImage', async function saveInvoiceImage(element: HTMLElement, invoiceNumber: string): Promise<string | true | null> {
  const blob = await renderPng(element);
  return saveFile(blob, { suggestedName: `فاتورة ${invoiceNumber}.png`, kind: 'image' });
});

/** Copies the invoice image to the clipboard. Throws when the platform refuses clipboard images. */
export const copyInvoiceImage = wrap('invoices.copyInvoiceImage', async function copyInvoiceImage(element: HTMLElement): Promise<void> {
  if (typeof ClipboardItem === 'undefined' || !navigator.clipboard?.write) throw new Error('نسخ الصور غير مدعوم هنا، احفظ الصورة بدلاً من ذلك');
  // Pass the promise straight to ClipboardItem so WebKit keeps the user-gesture context while rendering.
  await navigator.clipboard.write([new ClipboardItem({ 'image/png': renderPng(element) })]);
});
