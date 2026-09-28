/**
 * Template CRUD for the designer (Phase 11a). Stored in `localStorage`, not
 * the mock `db`/IndexedDB snapshot — this module intentionally avoids
 * touching `src/mocks/db.ts` / `persist.ts` (out of scope: Phase 13a is
 * concurrently changing backup-related persistence, and the whole-db
 * snapshot shape isn't a good fit for a per-device "which template did I
 * leave open" kind of setting anyway). Matches the same localStorage
 * pattern as `useAppearance.ts` and `format.ts`'s numeral setting.
 */
import { ApiError } from '@/mocks';
import { defaultTemplateOptions, type DocumentKind, type PdfTemplate, type TemplateExport } from '../types';

import { backendCall, usesRust } from '@/modules/core/services/backend';
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

export const listTemplates = wrap('templates.listTemplates', async function listTemplates(kind?: DocumentKind): Promise<PdfTemplate[]> {
  if (usesRust('templates')) return backendCall('templates_list_templates', { kind });
  const all = load();
  return kind ? all.filter((t) => t.kind === kind) : all;
});

export const getTemplate = wrap('templates.getTemplate', async function getTemplate(id: string): Promise<PdfTemplate | undefined> {
  if (usesRust('templates')) return (await backendCall('templates_get_template', { id })) ?? undefined;
  return load().find((t) => t.id === id);
});

export const getDefaultTemplate = wrap('templates.getDefaultTemplate', async function getDefaultTemplate(kind: DocumentKind): Promise<PdfTemplate | undefined> {
  if (usesRust('templates')) return (await backendCall('templates_get_default_template', { kind })) ?? undefined;
  const all = load().filter((t) => t.kind === kind);
  return all.find((t) => t.isDefault) ?? all[0];
});

export const saveTemplate = wrap('templates.saveTemplate', async function saveTemplate(template: PdfTemplate): Promise<PdfTemplate> {
  if (usesRust('templates')) return backendCall('templates_save_template', { template });
  const all = load();
  const idx = all.findIndex((t) => t.id === template.id);
  const updated: PdfTemplate = { ...template, updatedAt: new Date().toISOString() };
  if (idx >= 0) all[idx] = updated;
  else all.push(updated);
  save(all);
  return updated;
});

export const setAsDefault = wrap('templates.setAsDefault', async function setAsDefault(id: string): Promise<void> {
  if (usesRust('templates')) {
    await backendCall('templates_set_as_default', { id });
    return;
  }
  const all = load();
  const target = all.find((t) => t.id === id);
  if (!target) return;
  for (const t of all) {
    if (t.kind === target.kind) t.isDefault = t.id === id;
  }
  save(all);
});

export const duplicateTemplate = wrap('templates.duplicateTemplate', async function duplicateTemplate(id: string): Promise<PdfTemplate | undefined> {
  if (usesRust('templates')) return (await backendCall('templates_duplicate_template', { id })) ?? undefined;
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

export const deleteTemplate = wrap('templates.deleteTemplate', async function deleteTemplate(id: string): Promise<void> {
  if (usesRust('templates')) {
    await backendCall('templates_delete_template', { id });
    return;
  }
  const all = load().filter((t) => t.id !== id);
  save(all);
});

export const resetTemplateToDefaults = wrap('templates.resetTemplateToDefaults', async function resetTemplateToDefaults(id: string): Promise<PdfTemplate | undefined> {
  if (usesRust('templates')) return (await backendCall('templates_reset_template_to_defaults', { id })) ?? undefined;
  const all = load();
  const target = all.find((t) => t.id === id);
  if (!target) return undefined;
  target.options = defaultTemplateOptions();
  target.customSource = null;
  target.updatedAt = new Date().toISOString();
  save(all);
  return target;
});

/** Pure transform of an object the page already holds — no backend round trip (T-6). */
export const exportTemplate = wrap('templates.exportTemplate', function exportTemplate(template: PdfTemplate): TemplateExport {
  const { id: _id, isDefault: _isDefault, createdAt: _createdAt, updatedAt: _updatedAt, ...rest } = template;
  return { schema: 'pdf-template-v1', template: rest };
});

export const importTemplate = wrap('templates.importTemplate', async function importTemplate(json: TemplateExport): Promise<PdfTemplate> {
  if (usesRust('templates')) return backendCall('templates_import_template', { json });
  if (json.schema !== 'pdf-template-v1' || !json.template) {
    throw new ApiError('ملف القالب غير صالح', 'VALIDATION');
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
export const createTemplate = wrap('templates.createTemplate', async function createTemplate(kind: DocumentKind, baseTemplateId: PdfTemplate['baseTemplateId'], name: string): Promise<PdfTemplate> {
  if (usesRust('templates')) return backendCall('templates_create_template', { kind, baseTemplateId, name });
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
