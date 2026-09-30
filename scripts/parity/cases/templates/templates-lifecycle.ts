/**
 * L1 platform lane. The 15-templates.md §8(b) sequence: list (seeds the two defaults) → create →
 * save → set default → duplicate → reset → import a valid and an invalid file → delete → list.
 *
 * Restored to the real chained lifecycle (each step uses the id an earlier step actually returned)
 * now that `scripts/parity/polyfills.ts` gives this harness a real in-memory `localStorage`:
 * `templateService.ts` stores templates in `localStorage` (its own module doc comment), which used to
 * be `undefined` under the plain `bun run` process this runner uses — every call re-seeded fresh
 * defaults and no write ever persisted (see the polyfill's own header comment for the harness-gap
 * history). The previous call-by-call version of this file (never chaining an id across steps) is
 * gone; this is the version its own doc comment said to write once the gap closed.
 */
import { defineCase } from '../../case';
import * as templateService from '../../../../src/modules/templates/services/templateService';
import type { TemplateExport } from '../../../../src/modules/templates/types';

export default defineCase({
  name: 'templates/templates-lifecycle',
  source: '03-domains/15-templates.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  unordered: ['steps.*.value', 'steps.*.value.options.columns'],
  async run(s) {
    // `list-seed` is the very first templateService call this case makes: it seeds the two defaults
    // (standard + simplified) into the (empty) polyfilled localStorage and returns them.
    const seeded = await s.step('list-seed', () => templateService.listTemplates('invoice'));
    const standard = seeded.find((t) => t.baseTemplateId === 'invoice_standard')!;
    const simplified = seeded.find((t) => t.baseTemplateId === 'invoice_simplified')!;

    await s.step('get-seeded-default', () => templateService.getTemplate(standard.id));
    await s.step('get-default', () => templateService.getDefaultTemplate('invoice'));
    await s.step('get-missing', () => templateService.getTemplate('tpl-does-not-exist'));

    const created = await s.step('create', () => templateService.createTemplate('invoice', 'invoice_standard', 'قالب جديد'));

    const saved = await s.step('save', () => templateService.saveTemplate({ ...created, name: 'القياسية معدّلة', options: { ...created.options, accentColor: '#123456' } }));

    await s.step('set-default', () => templateService.setAsDefault(simplified.id));
    await s.step('set-default-missing', () => templateService.setAsDefault('tpl-does-not-exist'));

    const duplicated = await s.step('duplicate', () => templateService.duplicateTemplate(saved.id));
    await s.step('duplicate-missing', () => templateService.duplicateTemplate('tpl-does-not-exist'));

    await s.step('reset', () => templateService.resetTemplateToDefaults(duplicated!.id));
    await s.step('reset-missing', () => templateService.resetTemplateToDefaults('tpl-does-not-exist'));

    const exported = templateService.exportTemplate(standard);
    const validImport: TemplateExport = { schema: 'pdf-template-v1', template: exported.template };
    const imported = await s.step('import-valid', () => templateService.importTemplate(validImport));
    await s.expectError('import-invalid', () => templateService.importTemplate({ schema: 'not-a-template', template: {} } as unknown as TemplateExport));

    await s.step('delete', () => templateService.deleteTemplate(imported.id));
    await s.step('delete-missing', () => templateService.deleteTemplate('tpl-does-not-exist'));

    await s.step('list-final', () => templateService.listTemplates('invoice'));
  },
});
