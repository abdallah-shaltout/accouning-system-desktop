import { onMounted, ref, shallowRef, type Ref } from 'vue';
import { errorMessage } from './useToast';

/**
 * Standard loading / error / data triple for a service call. Every list and detail screen uses it,
 * so loading skeletons, error states (with retry) and empty states behave the same everywhere.
 */
export function useAsync<T>(loader: () => Promise<T>, options: { immediate?: boolean; initial?: T } = {}) {
  const data = shallowRef<T | undefined>(options.initial) as Ref<T | undefined>;
  const loading = ref(false);
  const error = ref<string | null>(null);
  let callId = 0;

  async function run(): Promise<T | undefined> {
    const id = ++callId;
    loading.value = true;
    error.value = null;
    try {
      const result = await loader();
      if (id === callId) data.value = result;
      return result;
    } catch (err) {
      if (id === callId) error.value = errorMessage(err);
      return undefined;
    } finally {
      if (id === callId) loading.value = false;
    }
  }

  if (options.immediate !== false) onMounted(run);

  return { data, loading, error, run, reload: run };
}

/** Wrap a mutating action with a `pending` flag (disables buttons / shows spinners). */
export function useAction<A extends unknown[], R>(fn: (...args: A) => Promise<R>) {
  const pending = ref(false);
  async function execute(...args: A): Promise<R> {
    pending.value = true;
    try {
      return await fn(...args);
    } finally {
      pending.value = false;
    }
  }
  return { pending, execute };
}
