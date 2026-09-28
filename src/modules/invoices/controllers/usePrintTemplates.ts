import { computed, ref, watch, type Ref } from 'vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import {
  DEFAULT_A4_TEMPLATE,
  DEFAULT_IMAGE_TEMPLATE,
  isA4Template,
  isImageTemplate,
  templateMeta,
  type A4TemplateId,
  type ImageTemplateId,
  type PrintLayout,
  type TemplateId,
} from '../helpers/invoiceTemplates';

/**
 * Plan 22: which template the print page shows per layout, seeded from `?template=` (one-off) or the
 * device default in `StoreSettings.printer`, plus "تعيين كافتراضي".
 */
export function usePrintTemplates(layout: Ref<PrintLayout>, queryTemplate: () => unknown) {
  const settings = useSettingsStore();
  const auth = useAuthStore();
  const toast = useToast();

  const defaultA4 = computed<A4TemplateId>(() => settings.settings?.printer.a4Template ?? DEFAULT_A4_TEMPLATE);
  const defaultImage = computed<ImageTemplateId>(() => settings.settings?.printer.imageTemplate ?? DEFAULT_IMAGE_TEMPLATE);

  const initial = queryTemplate();
  const a4 = ref<A4TemplateId>(isA4Template(initial) ? initial : defaultA4.value);
  const image = ref<ImageTemplateId>(isImageTemplate(initial) ? initial : defaultImage.value);
  // The print route's component is reused when only the query changes (e.g. `?template=` from a link).
  watch(queryTemplate, (t) => {
    if (isA4Template(t)) a4.value = t;
    else if (isImageTemplate(t)) image.value = t;
  });
  // Settings may finish loading after this page mounts (a hard reload of the print route): follow
  // the saved default until a template is chosen explicitly.
  let explicitA4 = isA4Template(initial);
  let explicitImage = isImageTemplate(initial);
  watch(defaultA4, (id) => {
    if (!explicitA4) a4.value = id;
  });
  watch(defaultImage, (id) => {
    if (!explicitImage) image.value = id;
  });

  /** The template of the current layout (thermal has one fixed layout, reported as the A4 pick). */
  const active = computed<TemplateId>({
    get: () => (layout.value === 'image' ? image.value : a4.value),
    set: (id) => {
      if (isImageTemplate(id)) {
        image.value = id;
        explicitImage = true;
      } else if (isA4Template(id)) {
        a4.value = id;
        explicitA4 = true;
      }
    },
  });
  const activeDefault = computed<TemplateId>(() => (layout.value === 'image' ? defaultImage.value : defaultA4.value));
  const activeMeta = computed(() => templateMeta(active.value));

  const canSetDefault = computed(() => auth.can('settings', 'write'));
  const savingDefault = ref(false);

  async function setDefault(id: TemplateId) {
    const printer = settings.settings?.printer;
    if (!printer) return;
    savingDefault.value = true;
    try {
      // Plain fields only: `updateSettings` merges `printer` server-side, and spreading the reactive
      // store object would hand its nested `thermal` Proxy to structuredClone (DataCloneError).
      const pick = isImageTemplate(id) ? { imageTemplate: id } : { a4Template: id as A4TemplateId };
      await settings.update({ printer: { mode: printer.mode, thermalWidthMm: printer.thermalWidthMm, ...pick } });
      toast.success('تم تعيين القالب الافتراضي', templateMeta(id)?.label);
    } catch (err) {
      toast.error(err);
    } finally {
      savingDefault.value = false;
    }
  }

  return { active, activeDefault, activeMeta, canSetDefault, savingDefault, setDefault };
}
