<script setup lang="ts">
import { computed, nextTick, onMounted, ref, useId, watch } from 'vue';
import { useRouter } from 'vue-router';
import { Search } from '@lucide/vue';
import { useCommandPalette } from '../controllers/useCommandPalette';
import { useHotkeys } from '../controllers/useHotkeys';
import { GROUP_LABEL, type PaletteResult } from '../types/commandPalette';

/**
 * Global Ctrl+K command palette overlay (docs/v2/14-platform.md §2). Mounted once in App.vue.
 * A combobox/listbox: `aria-activedescendant`, ↑↓ to move, Enter to choose, Esc to close.
 */
const router = useRouter();
const palette = useCommandPalette();
const input = ref<HTMLInputElement>();
const listId = useId();

useHotkeys({
  'ctrl+k': () => {
    palette.toggle();
    return false;
  },
});

watch(palette.open, async (isOpen) => {
  if (!isOpen) return;
  await nextTick();
  input.value?.focus();
});

const flatResults = computed(() => palette.results.value);
const activeId = computed(() => {
  const r = flatResults.value[palette.activeIndex.value];
  return r ? `${listId}-${r.id}` : undefined;
});

const activePrefix = computed(() => palette.detectPrefix(palette.query.value).prefix);
const prefixHint = computed(() => {
  const map: Record<string, string> = {
    '>': 'أوامر فقط',
    '@': 'عملاء وموردون',
    '#': 'أرقام مستندات',
    '$': 'حسابات',
    '?': 'مساعدة',
  };
  return activePrefix.value ? map[activePrefix.value] : null;
});

function select(result: PaletteResult) {
  const { to } = palette.choose(result);
  if (to) router.push(to);
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'ArrowDown') {
    e.preventDefault();
    palette.activeIndex.value = Math.min(palette.activeIndex.value + 1, flatResults.value.length - 1);
    scrollActive();
  } else if (e.key === 'ArrowUp') {
    e.preventDefault();
    palette.activeIndex.value = Math.max(palette.activeIndex.value - 1, 0);
    scrollActive();
  } else if (e.key === 'Enter') {
    e.preventDefault();
    const r = flatResults.value[palette.activeIndex.value];
    if (r) select(r);
  } else if (e.key === 'Escape') {
    e.preventDefault();
    palette.hide();
  } else if (e.key === 'Tab') {
    // Tab moves between groups: jump to the first item of the next group.
    e.preventDefault();
    const groups = [...palette.grouped.value.keys()];
    const current = flatResults.value[palette.activeIndex.value]?.group;
    const currentGroupIdx = groups.indexOf(current as string);
    const nextGroup = groups[(currentGroupIdx + 1) % groups.length];
    const idx = flatResults.value.findIndex((r) => r.group === nextGroup);
    if (idx >= 0) {
      palette.activeIndex.value = idx;
      scrollActive();
    }
  }
}

const list = ref<HTMLElement>();
function scrollActive() {
  nextTick(() => list.value?.querySelector<HTMLElement>('[data-active="true"]')?.scrollIntoView({ block: 'nearest' }));
}

onMounted(() => {
  palette.onQueryChange();
});
watch(palette.query, () => palette.onQueryChange());
</script>

<template>
  <Teleport to="body">
    <div v-if="palette.open.value" dir="rtl" class="fixed inset-0 z-80 flex items-start justify-center bg-black/45 p-4 pt-[10vh]" @mousedown.self="palette.hide()">
      <div role="dialog" aria-modal="true" aria-label="لوحة الأوامر" class="w-full max-w-xl overflow-hidden rounded-xl border border-border bg-background shadow-2xl">
        <div class="flex items-center gap-2 border-b border-border px-3">
          <Search class="size-4 shrink-0 text-text-secondary" />
          <input
            ref="input"
            v-model="palette.query.value"
            role="combobox"
            aria-expanded="true"
            :aria-controls="listId"
            :aria-activedescendant="activeId"
            aria-autocomplete="list"
            placeholder="ابحث أو نفّذ أمراً… (جرّب > أو @ أو # أو $ أو ؟)"
            class="h-11 w-full bg-transparent text-body outline-none placeholder:text-text-secondary"
            @keydown="onKeydown"
          />
          <span v-if="prefixHint" class="shrink-0 rounded bg-surface-hover px-1.5 py-0.5 text-caption text-text-secondary">{{ prefixHint }}</span>
        </div>

        <div ref="list" :id="listId" role="listbox" class="max-h-[60vh] overflow-y-auto p-1.5">
          <template v-if="palette.loading.value && !flatResults.length">
            <div class="px-3 py-8 text-center text-xs text-text-secondary">جارٍ البحث…</div>
          </template>
          <template v-else-if="!flatResults.length">
            <div class="px-3 py-8 text-center text-xs text-text-secondary">
              {{ palette.query.value ? 'لا توجد نتائج' : 'ابدأ الكتابة للبحث، أو استخدم الأسهم لفتح العناصر الأخيرة' }}
            </div>
          </template>
          <template v-else>
            <div v-for="[group, items] in palette.grouped.value" :key="group" class="mb-1 last:mb-0">
              <p class="px-2 py-1 text-caption font-medium text-text-secondary">{{ GROUP_LABEL[group as keyof typeof GROUP_LABEL] }}</p>
              <button
                v-for="item in items"
                :id="`${listId}-${item.id}`"
                :key="item.id"
                type="button"
                role="option"
                :aria-selected="flatResults[palette.activeIndex.value]?.id === item.id"
                :data-active="flatResults[palette.activeIndex.value]?.id === item.id"
                class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-start text-body"
                :class="flatResults[palette.activeIndex.value]?.id === item.id ? 'bg-primary/10 text-primary' : 'hover:bg-surface-hover'"
                @mouseenter="palette.activeIndex.value = flatResults.indexOf(item)"
                @click="select(item)"
              >
                <component :is="item.icon" v-if="item.icon" class="size-4 shrink-0 text-text-secondary" />
                <span class="min-w-0 flex-1">
                  <span class="block truncate">{{ item.title }}</span>
                  <span v-if="item.subtitle" class="block truncate text-xs text-text-secondary">{{ item.subtitle }}</span>
                </span>
              </button>
            </div>
          </template>
        </div>

        <div class="flex items-center justify-between border-t border-border px-3 py-1.5 text-caption text-text-secondary">
          <span class="flex items-center gap-2">
            <kbd class="num rounded border border-border px-1">↑↓</kbd> تنقّل
            <kbd class="num rounded border border-border px-1">Enter</kbd> فتح
            <kbd class="num rounded border border-border px-1">Tab</kbd> مجموعة
          </span>
          <kbd class="num rounded border border-border px-1">Esc</kbd>
        </div>
      </div>
    </div>
  </Teleport>
</template>
