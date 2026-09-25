/**
 * Template CRUD for the designer (Phase 11a). Stored in `localStorage`, not
 * the mock `db`/IndexedDB snapshot — this module intentionally avoids
 * touching `src/mocks/db.ts` / `persist.ts` (out of scope: Phase 13a is
 * concurrently changing backup-related persistence, and the whole-db
 * snapshot shape isn't a good fit for a per-device "which template did I
 * leave open" kind of setting anyway). Matches the same localStorage
 * pattern as `useAppearance.ts` and `format.ts`'s numeral setting.
 */
import { defaultTemplateOptions, type DocumentKind, type PdfTemplate, type TemplateExport } from '../types';

import { wrap } from '@/modules/diagnostics/services/defineService';

const STORAGE_KEY = 'pdf_templates_v1';

function uid(): string {
  return `tpl_${Date.now().toString(36)}${Math.random().toString(36).slice(2, 8)}`;
}

function load(): PdfTemplate[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return seedDefaults();
    const parsed = JSON.parse(raw) as PdfTemplate[];
    return Array.isArray(parsed) && parsed.length > 0 ? parsed : seedDefaults();
  } catch {
    return seedDefaults();
  }
}

function save(templates: PdfTemplate[]): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(templates));
  } catch {
    /* private mode — changes just won't persist */
  }
}

function seedDefaults(): PdfTemplate[] {
  const now = new Date().toISOString();
  const standard: PdfTemplate = {
    id: uid(),
    name: 'الفاتورة الضريبية القياسية',
    kind: 'invoice',
    baseTemplateId: 'invoice_standard',
    options: defaultTemplateOptions(),
    customSource: null,
    isDefault: true,
    createdAt: now,
    updatedAt: now,
  };
  const simplified: PdfTemplate = {
    id: uid(),
    name: 'الفاتورة الضريبية المبسطة',
    kind: 'invoice',
    baseTemplateId: 'invoice_simplified',
    options: { ...defaultTemplateOptions(), header: { ...defaultTemplateOptions().header, title: 'فاتورة ضريبية مبسطة', titleEn: 'SIMPLIFIED TAX INVOICE' } },
    customSource: null,
    isDefault: false,
    createdAt: now,
    updatedAt: now,
  };
  const seeded = [standard, simplified];
  save(seeded);
  return seeded;
}

export const listTemplates = wrap('templates.listTemplates', function listTemplates(kind?: DocumentKind): PdfTemplate[] {
  const all = load();
  return kind ? all.filter((t) => t.kind === kind) : all;
});

export const getTemplate = wrap('templates.getTemplate', function getTemplate(id: string): PdfTemplate | undefined {
  return load().find((t) => t.id === id);
});

export const getDefaultTemplate = wrap('templates.getDefaultTemplate', function getDefaultTemplate(kind: DocumentKind): PdfTemplate | undefined {
  const all = load().filter((t) => t.kind === kind);
  return all.find((t) => t.isDefault) ?? all[0];
});

export const saveTemplate = wrap('templates.saveTemplate', function saveTemplate(template: PdfTemplate): PdfTemplate {
  const all = load();
  const idx = all.findIndex((t) => t.id === template.id);
  const updated: PdfTemplate = { ...template, updatedAt: new Date().toISOString() };
  if (idx >= 0) all[idx] = updated;
  else all.push(updated);
  save(all);
  return updated;
});

export const setAsDefault = wrap('templates.setAsDefault', function setAsDefault(id: string): void {
  const all = load();
  const target = all.find((t) => t.id === id);
  if (!target) return;
  for (const t of all) {
    if (t.kind === target.kind) t.isDefault = t.id === id;
  }
  save(all);
});

export const duplicateTemplate = wrap('templates.duplicateTemplate', function duplicateTemplate(id: string): PdfTemplate | undefined {
  const all = load();
  const source = all.find((t) => t.id === id);
  if (!source) return undefined;
  const now = new Date().toISOString();
  const copy: PdfTemplate = {
    ...source,
    id: uid(),
    name: `${source.name} (نسخة)`,
    isDefault: false,
    createdAt: now,
    updatedAt: now,
    options: JSON.parse(JSON.stringify(source.options)),
  };
  all.push(copy);
  save(all);
  return copy;
});

export const deleteTemplate = wrap('templates.deleteTemplate', function deleteTemplate(id: string): void {
  const all = load().filter((t) => t.id !== id);
  save(all);
});

export const resetTemplateToDefaults = wrap('templates.resetTemplateToDefaults', function resetTemplateToDefaults(id: string): PdfTemplate | undefined {
  const all = load();
  const target = all.find((t) => t.id === id);
  if (!target) return undefined;
  target.options = defaultTemplateOptions();
  target.customSource = null;
  target.updatedAt = new Date().toISOString();
  save(all);
  return target;
});

export const exportTemplate = wrap('templates.exportTemplate', function exportTemplate(template: PdfTemplate): TemplateExport {
  const { id: _id, isDefault: _isDefault, createdAt: _createdAt, updatedAt: _updatedAt, ...rest } = template;
  return { schema: 'pdf-template-v1', template: rest };
});

export const importTemplate = wrap('templates.importTemplate', function importTemplate(json: TemplateExport): PdfTemplate {
  if (json.schema !== 'pdf-template-v1' || !json.template) {
    throw new Error('ملف القالب غير صالح');
  }
  const now = new Date().toISOString();
  const imported: PdfTemplate = {
    ...json.template,
    id: uid(),
    isDefault: false,
    createdAt: now,
    updatedAt: now,
  };
  const all = load();
  all.push(imported);
  save(all);
  return imported;
});

/** A brand-new blank template for "duplicate"/"new" flows the designer's top bar offers. */
export const createTemplate = wrap('templates.createTemplate', function createTemplate(kind: DocumentKind, baseTemplateId: PdfTemplate['baseTemplateId'], name: string): PdfTemplate {
  const now = new Date().toISOString();
  const template: PdfTemplate = {
    id: uid(),
    name,
    kind,
    baseTemplateId,
    options: defaultTemplateOptions(),
    customSource: null,
    isDefault: false,
    createdAt: now,
    updatedAt: now,
  };
  const all = load();
  all.push(template);
  save(all);
  return template;
});
