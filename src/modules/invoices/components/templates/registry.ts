import { defineAsyncComponent, type Component } from 'vue';
import { FONTS, type FontFamily } from '@/modules/core/controllers/useAppearance';
import type { A4TemplateId, DocFont, ImageTemplateId, TemplateId } from '../../helpers/invoiceTemplates';

/**
 * Plan 22: template id → component. Metadata (labels, faces) lives in `helpers/invoiceTemplates.ts`;
 * this file only maps ids to lazily loaded components, so opening the print page never pulls in all
 * twenty layouts at once.
 */
const A4: Record<A4TemplateId, Component> = {
  standard: defineAsyncComponent(() => import('../InvoiceA4.vue')),
  corporate: defineAsyncComponent(() => import('./a4/A4Corporate.vue')),
  sidebar: defineAsyncComponent(() => import('./a4/A4Sidebar.vue')),
  banner: defineAsyncComponent(() => import('./a4/A4Banner.vue')),
  swiss: defineAsyncComponent(() => import('./a4/A4Swiss.vue')),
  bento: defineAsyncComponent(() => import('./a4/A4Bento.vue')),
  luxury: defineAsyncComponent(() => import('./a4/A4Luxury.vue')),
  compact: defineAsyncComponent(() => import('./a4/A4Compact.vue')),
  letter: defineAsyncComponent(() => import('./a4/A4Letter.vue')),
  geometric: defineAsyncComponent(() => import('./a4/A4Geometric.vue')),
};

const IMAGE: Record<ImageTemplateId, Component> = {
  instapay: defineAsyncComponent(() => import('./image/ImgInstapay.vue')),
  ticket: defineAsyncComponent(() => import('./image/ImgTicket.vue')),
  wallet: defineAsyncComponent(() => import('./image/ImgWallet.vue')),
  chat: defineAsyncComponent(() => import('./image/ImgChat.vue')),
  paper: defineAsyncComponent(() => import('./image/ImgPaper.vue')),
  noir: defineAsyncComponent(() => import('./image/ImgNoir.vue')),
  spotlight: defineAsyncComponent(() => import('./image/ImgSpotlight.vue')),
  grouped: defineAsyncComponent(() => import('./image/ImgGrouped.vue')),
  poster: defineAsyncComponent(() => import('./image/ImgPoster.vue')),
  glass: defineAsyncComponent(() => import('./image/ImgGlass.vue')),
};

export function templateComponent(id: TemplateId): Component {
  return (A4 as Record<string, Component>)[id] ?? (IMAGE as Record<string, Component>)[id] ?? A4.standard;
}

const FONT_PACKAGE: Record<DocFont, FontFamily> = {
  cairo: 'cairo',
  plex: 'ibm-plex-sans-arabic',
  tajawal: 'tajawal',
  naskh: 'noto-naskh-arabic',
};
const loaded = new Set<DocFont>();

/** Loads a template face's stylesheet once (same fontsource packages the appearance setting uses). */
export async function loadDocFont(font: DocFont): Promise<void> {
  if (loaded.has(font)) return;
  await FONTS[FONT_PACKAGE[font]].load();
  loaded.add(font);
}
