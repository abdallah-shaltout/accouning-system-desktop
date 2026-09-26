/**
 * v2 doc 17 Phase F-0 — the shared form composable: Zod schema → reactive `values`/`errors`,
 * dirty-tracking, `submit()`/`reset()`, a route-leave guard when dirty, and Ctrl+S bound to submit.
 * Reuses the existing `validate()` helper (`core/helpers/validation.ts`) rather than re-implementing
 * Zod parsing, and `useConfirm()` for the leave-guard prompt (same dialog every other confirm uses).
 *
 * Not wired into any existing page yet (that's F-2/F-3) — this is the composable a `FormPage` /
 * hand-built form will call once pages start migrating.
 */
import { computed, reactive, ref, type Ref } from 'vue';
import { onBeforeRouteLeave } from 'vue-router';
import type { z } from 'zod';
import { validate } from '../helpers/validation';
import { useConfirm } from './useConfirm';
import { useHotkeys } from './useHotkeys';

export interface UseFormOptions<V extends object, S extends z.ZodType> {
  schema: S | ((values: V) => S);
  initial: V;
  onSubmit: (values: V) => Promise<void> | void;
  /** Skip the "unsaved changes" leave-guard (e.g. read-only or already-posted documents). */
  guardLeave?: boolean;
}

export interface UseForm<V> {
  values: V;
  errors: Record<string, string>;
  dirty: Ref<boolean>;
  submitting: Ref<boolean>;
  submit: () => Promise<boolean>;
  reset: (next?: V) => void;
  validateNow: () => boolean;
}

export function useForm<V extends object, S extends z.ZodType>(options: UseFormOptions<V, S>): UseForm<V> {
  type Values = V;
  const initialSnapshot: Values = structuredClone(options.initial);
  const values = reactive(structuredClone(options.initial)) as Values;
  const errors = reactive<Record<string, string>>({});
  const submitting = ref(false);
  const confirm = useConfirm();

  const dirty = computed(() => JSON.stringify(values) !== JSON.stringify(initialSnapshot));

  function schemaFor(v: Values): S {
    return typeof options.schema === 'function' ? (options.schema as (v: Values) => S)(v) : options.schema;
  }

  function validateNow(): boolean {
    const result = validate(schemaFor(values), values);
    for (const key of Object.keys(errors)) delete errors[key];
    Object.assign(errors, result);
    return Object.keys(result).length === 0;
  }

  async function submit(): Promise<boolean> {
    if (!validateNow()) return false;
    submitting.value = true;
    try {
      await options.onSubmit(values);
      Object.assign(initialSnapshot, structuredClone(values));
      return true;
    } finally {
      submitting.value = false;
    }
  }

  function reset(next?: Values) {
    const base = next ?? initialSnapshot;
    Object.assign(values, structuredClone(base));
    Object.assign(initialSnapshot, structuredClone(base));
    for (const key of Object.keys(errors)) delete errors[key];
  }

  // Ctrl+S submits instead of the OS "save page" dialog.
  useHotkeys({
    'ctrl+s': () => {
      void submit();
      return false;
    },
  });

  if (options.guardLeave !== false) {
    onBeforeRouteLeave(async () => {
      if (!dirty.value) return true;
      return confirm({
        title: 'تغييرات غير محفوظة',
        message: 'لديك تغييرات لم تُحفظ. هل تريد المغادرة دون حفظها؟',
        confirmText: 'مغادرة دون حفظ',
        cancelText: 'البقاء',
        danger: true,
      });
    });
  }

  return { values, errors, dirty, submitting, submit, reset, validateNow };
}
