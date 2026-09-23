/**
 * v2 phase 9 (docs/v2/10-branches-currencies-cost-centers.md §1): the currently-selected branch —
 * "one branch, or كل الفروع (all branches; read views only)". Persisted per-device in
 * localStorage (same pattern as `useTheme.ts`), not in `db.settings` — the selection is a per-user
 * UI preference, not a company setting. Only meaningful once `features.branches` is on and the
 * user has more than one allowed branch; every other screen keeps treating a single implicit
 * branch as before.
 */
import { defineStore } from 'pinia';
import { computed, ref, watch } from 'vue';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { useSettingsStore } from './useSettingsStore';
import * as branchesService from '../services/branchesService';
import type { Branch } from '../types';

const KEY = 'app_selected_branch';
/** Sentinel for "كل الفروع" (all branches; read-only aggregate views). */
export const ALL_BRANCHES = '__all__';

function loadSaved(): string | null {
  try {
    return localStorage.getItem(KEY);
  } catch {
    return null;
  }
}

export const useBranchStore = defineStore('branch', () => {
  const branches = ref<Branch[]>([]);
  const selected = ref<string>(loadSaved() ?? ALL_BRANCHES);
  const loaded = ref(false);

  async function load() {
    if (loaded.value) return;
    branches.value = await branchesService.getBranches();
    loaded.value = true;
    ensureValidSelection();
  }

  const auth = useAuthStore();
  const settingsStore = useSettingsStore();

  /** Branches this user may pick from — every active branch, filtered by `allowedBranches` when set. */
  const availableBranches = computed(() => {
    const allowed = auth.user?.allowedBranches;
    const active = branches.value.filter((b) => b.active);
    return allowed?.length ? active.filter((b) => allowed.includes(b.id)) : active;
  });

  /** The switcher only shows once branches are on AND this user actually has more than one to pick from. */
  const showSwitcher = computed(() => !!settingsStore.settings?.features?.branches && availableBranches.value.length > 1);

  const selectedBranch = computed(() => branches.value.find((b) => b.id === selected.value));
  const isAllBranches = computed(() => selected.value === ALL_BRANCHES);

  /** The branch id to stamp new documents with: the explicit selection, or the user's home/only branch. */
  const effectiveBranchId = computed(() => {
    if (selected.value !== ALL_BRANCHES) return selected.value;
    return auth.user?.homeBranch ?? availableBranches.value[0]?.id ?? branches.value[0]?.id;
  });

  function ensureValidSelection() {
    const allowedIds = new Set(availableBranches.value.map((b) => b.id));
    if (selected.value !== ALL_BRANCHES && !allowedIds.has(selected.value)) {
      select(auth.user?.homeBranch && allowedIds.has(auth.user.homeBranch) ? auth.user.homeBranch : availableBranches.value[0]?.id ?? ALL_BRANCHES);
    }
  }

  function select(id: string) {
    selected.value = id;
    try {
      localStorage.setItem(KEY, id);
    } catch {
      /* ignore */
    }
  }

  watch(() => auth.user?.id, () => ensureValidSelection());

  return { branches, selected, loaded, load, availableBranches, showSwitcher, selectedBranch, isAllBranches, effectiveBranchId, select };
});
