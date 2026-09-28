/**
 * Invoice template catalogue (plans/pending/22-invoice-templates) — the single owner of template ids,
 * Arabic labels and faces. The async components live in `components/templates/registry.ts`; adding a
 * template is one component file + one row here + one row there.
 */

/** What the print page renders. `thermal` has one fixed layout; `a4` and `image` pick a template. */
export type PrintLayout = 'a4' | 'thermal' | 'image';

export type DocFont = 'cairo' | 'plex' | 'tajawal' | 'naskh';

export type A4TemplateId = 'standard' | 'corporate' | 'sidebar' | 'banner' | 'swiss' | 'bento' | 'luxury' | 'compact' | 'letter' | 'geometric';

export type ImageTemplateId = 'instapay' | 'ticket' | 'wallet' | 'chat' | 'paper' | 'noir' | 'spotlight' | 'grouped' | 'poster' | 'glass';

export type TemplateId = A4TemplateId | ImageTemplateId;

export interface InvoiceTemplateMeta<Id extends TemplateId = TemplateId> {
  id: Id;
  label: string;
  /** One line shown under the name in the gallery. */
  description: string;
  font: DocFont;
  /** A4 only: the template draws edge to edge (its own inner padding), so the page prints with no margin. */
  bleed?: boolean;
}

export const A4_TEMPLATES: InvoiceTemplateMeta<A4TemplateId>[] = [
  { id: 'standard', label: 'القياسي', description: 'فاتورة ضريبية كاملة بالأبيض والأسود', font: 'cairo' },
  { id: 'corporate', label: 'مؤسسي', description: 'جدول بحدود كاملة وخانات ختم وتوقيع', font: 'naskh', bleed: true },
  { id: 'sidebar', label: 'عمود جانبي', description: 'بيانات المتجر والإجمالي في عمود ملون', font: 'plex', bleed: true },
  { id: 'banner', label: 'شريط علوي', description: 'الإجمالي داخل شريط داكن أعلى الصفحة', font: 'cairo', bleed: true },
  { id: 'swiss', label: 'سويسري', description: 'رقم الفاتورة كبير ومساحات بيضاء وخطوط رفيعة', font: 'plex', bleed: true },
  { id: 'bento', label: 'مربعات', description: 'البيانات في مربعات منفصلة سهلة القراءة', font: 'tajawal', bleed: true },
  { id: 'luxury', label: 'فاخر', description: 'إطار مزدوج وشعار دائري بخط النسخ', font: 'naskh', bleed: true },
  { id: 'compact', label: 'مضغوط', description: 'للفواتير الطويلة: صفوف كثيفة متناوبة', font: 'plex', bleed: true },
  { id: 'letter', label: 'خطاب رسمي', description: 'ترويسة وتحية ومبلغ بالحروف وتوقيع', font: 'naskh', bleed: true },
  { id: 'geometric', label: 'هندسي', description: 'أشكال مائلة وأرقام أصناف بارزة', font: 'cairo', bleed: true },
];

export const IMAGE_TEMPLATES: InvoiceTemplateMeta<ImageTemplateId>[] = [
  { id: 'instapay', label: 'إيصال تحويل', description: 'بنمط إيصالات إنستاباي: مبلغ كبير وعلامة نجاح', font: 'cairo' },
  { id: 'ticket', label: 'تذكرة', description: 'مثل بطاقة الصعود بخط تثقيب وقسيمة', font: 'plex' },
  { id: 'wallet', label: 'بطاقة بنكية', description: 'الإجمالي على بطاقة داكنة والأصناف أسفلها', font: 'plex' },
  { id: 'chat', label: 'محادثة', description: 'الأصناف كرسائل على خلفية محادثة', font: 'tajawal' },
  { id: 'paper', label: 'ورقة إيصال', description: 'إيصال بحواف ممزقة على خلفية ملونة', font: 'plex' },
  { id: 'noir', label: 'داكن فاخر', description: 'خلفية داكنة وأرقام ذهبية', font: 'naskh' },
  { id: 'spotlight', label: 'إجمالي بارز', description: 'بسيط جداً والإجمالي في المنتصف', font: 'tajawal' },
  { id: 'grouped', label: 'قوائم مجمعة', description: 'مجموعات مثل إعدادات الهاتف', font: 'cairo' },
  { id: 'poster', label: 'ملصق', description: 'لون جريء وخط ضخم', font: 'cairo' },
  { id: 'glass', label: 'زجاجي', description: 'بطاقة شفافة فوق تدرج لوني', font: 'tajawal' },
];

export const DEFAULT_A4_TEMPLATE: A4TemplateId = 'standard';
export const DEFAULT_IMAGE_TEMPLATE: ImageTemplateId = 'instapay';

export function templatesFor(layout: PrintLayout): InvoiceTemplateMeta[] {
  if (layout === 'a4') return A4_TEMPLATES;
  if (layout === 'image') return IMAGE_TEMPLATES;
  return [];
}

export function isA4Template(id: unknown): id is A4TemplateId {
  return A4_TEMPLATES.some((t) => t.id === id);
}

export function isImageTemplate(id: unknown): id is ImageTemplateId {
  return IMAGE_TEMPLATES.some((t) => t.id === id);
}

export function templateMeta(id: TemplateId): InvoiceTemplateMeta | undefined {
  return [...A4_TEMPLATES, ...IMAGE_TEMPLATES].find((t) => t.id === id);
}

/** Tailwind class for a template face (tokens in design-system.css). */
export const DOC_FONT_CLASS: Record<DocFont, string> = {
  cairo: 'font-doc-cairo',
  plex: 'font-doc-plex',
  tajawal: 'font-doc-tajawal',
  naskh: 'font-doc-naskh',
};

/** Width (CSS px) every image template is laid out at — a phone screen; exported at 3× pixel ratio. */
export const IMAGE_TEMPLATE_WIDTH_PX = 400;
