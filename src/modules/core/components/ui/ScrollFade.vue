<script setup lang="ts">
/**
 * Wraps a horizontally-scrollable strip (tabs, chip rows, filter bars) with a hidden
 * native scrollbar, drag-to-scroll, wheel-to-horizontal-scroll, and edge fade gradients
 * that only appear on the side(s) that still have more content to reveal.
 */
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { dirIcon } from '../../helpers/dirIcon';
import DirIcon from './DirIcon.vue';

const props = withDefaults(defineProps<{ fadeSize?: number }>(), { fadeSize: 40 });

const scroller = ref<HTMLElement | null>(null);
const canScrollStart = ref(false);
const canScrollEnd = ref(false);
const isDragging = ref(false);

let pointerDownX = 0;
let pointerDownScrollLeft = 0;
let activePointerId: number | null = null;
let didDrag = false;
let resizeObserver: ResizeObserver | undefined;
let dragRaf = 0;
let pendingScrollLeft: number | null = null;

function updateFades() {
  const el = scroller.value;
  if (!el) return;
  const isRtl = getComputedStyle(el).direction === 'rtl';
  const max = el.scrollWidth - el.clientWidth;
  if (max <= 1) {
    canScrollStart.value = false;
    canScrollEnd.value = false;
    return;
  }
  // In RTL, browsers disagree on scrollLeft's sign convention: Chromium/WebView2 (what this
  // app runs on) keeps it negative, from 0 at the start down to -max at the end; Firefox/older
  // Safari keep it positive, from +max at the start down to 0 at the end. Either way, start is
  // whichever extreme has the larger magnitude once max is removed — so probe the sign instead
  // of assuming one engine's convention.
  const fromStart = isRtl ? (el.scrollLeft <= 0 ? -el.scrollLeft : max - el.scrollLeft) : el.scrollLeft;
  const fromEnd = max - fromStart;
  canScrollStart.value = fromStart > 1;
  canScrollEnd.value = fromEnd > 1;
}

function onScroll() {
  updateFades();
}

// Only mouse/pen pointers drag-scroll — touch keeps its native scroll/tap behavior,
// and pointer capture is deferred until real movement so a plain click on a link
// underneath is never hijacked.
function onPointerDown(e: PointerEvent) {
  if (e.pointerType === 'touch') return;
  const el = scroller.value;
  if (!el) return;
  activePointerId = e.pointerId;
  didDrag = false;
  pointerDownX = e.clientX;
  pointerDownScrollLeft = el.scrollLeft;
}

// Writing scrollLeft directly on every pointermove forces a synchronous style/layout
// recalc each time (onScroll -> updateFades -> getComputedStyle), which on a fast pointer
// stream reads as a jerky, stepped drag instead of a smooth one. Collapsing every move
// between frames into a single rAF-scheduled write keeps it to one recalc per frame.
function onPointerMove(e: PointerEvent) {
  const el = scroller.value;
  if (!el || activePointerId !== e.pointerId) return;
  const delta = e.clientX - pointerDownX;
  if (!didDrag) {
    if (Math.abs(delta) <= 3) return;
    didDrag = true;
    isDragging.value = true;
    el.setPointerCapture(e.pointerId);
  }
  pendingScrollLeft = pointerDownScrollLeft - delta;
  if (!dragRaf) {
    dragRaf = requestAnimationFrame(() => {
      dragRaf = 0;
      if (pendingScrollLeft !== null && scroller.value) scroller.value.scrollLeft = pendingScrollLeft;
      pendingScrollLeft = null;
    });
  }
}

function onPointerUp(e: PointerEvent) {
  if (activePointerId !== e.pointerId) return;
  const el = scroller.value;
  activePointerId = null;
  isDragging.value = false;
  if (dragRaf) {
    cancelAnimationFrame(dragRaf);
    dragRaf = 0;
    if (pendingScrollLeft !== null && el) el.scrollLeft = pendingScrollLeft;
    pendingScrollLeft = null;
  }
  if (el?.hasPointerCapture(e.pointerId)) el.releasePointerCapture(e.pointerId);
}

// A click that ended a real drag shouldn't also activate/navigate.
function onClickCapture(e: MouseEvent) {
  if (didDrag) {
    e.preventDefault();
    e.stopPropagation();
    didDrag = false;
  }
}

// Arrow buttons move by most of a page so users who don't know they can drag/wheel-scroll
// still have a discoverable, keyboard/mouse-friendly way to reach the rest of the strip.
function scrollToward(direction: 'start' | 'end') {
  const el = scroller.value;
  if (!el) return;
  const isRtl = getComputedStyle(el).direction === 'rtl';
  const page = el.clientWidth * 0.8;
  // Chromium RTL's scrollLeft decreases toward the end (see updateFades); LTR and
  // non-Chromium RTL both increase toward the end relative to the start's sign.
  const towardEndSign = isRtl && el.scrollLeft <= 0 ? -1 : 1;
  const sign = direction === 'end' ? towardEndSign : -towardEndSign;
  el.scrollBy({ left: sign * page, behavior: 'smooth' });
}

function onWheel(e: WheelEvent) {
  const el = scroller.value;
  if (!el) return;
  if (Math.abs(e.deltaY) > Math.abs(e.deltaX)) {
    el.scrollLeft += e.deltaY;
    e.preventDefault();
  }
}

onMounted(() => {
  updateFades();
  const el = scroller.value;
  if (!el) return;
  resizeObserver = new ResizeObserver(updateFades);
  resizeObserver.observe(el);
  for (const child of el.children) resizeObserver.observe(child);
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  if (dragRaf) cancelAnimationFrame(dragRaf);
});
</script>

<template>
  <div class="relative">
    <div
      ref="scroller"
      class="scroll-fade-track flex overflow-x-auto overflow-y-hidden"
      :class="isDragging ? 'cursor-grabbing select-none' : 'cursor-grab'"
      @scroll="onScroll"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
      @pointercancel="onPointerUp"
      @wheel="onWheel"
      @click.capture="onClickCapture"
    >
      <slot />
    </div>
    <div
      class="scroll-fade-edge scroll-fade-edge--start pointer-events-none absolute inset-y-0 start-0 transition-opacity"
      :style="{ width: `${props.fadeSize}px`, opacity: canScrollStart ? 1 : 0 }"
      aria-hidden="true"
    />
    <div
      class="scroll-fade-edge scroll-fade-edge--end pointer-events-none absolute inset-y-0 end-0 transition-opacity"
      :style="{ width: `${props.fadeSize}px`, opacity: canScrollEnd ? 1 : 0 }"
      aria-hidden="true"
    />
    <button
      v-if="canScrollStart"
      type="button"
      class="scroll-fade-arrow absolute inset-y-0 start-0 flex items-center px-0.5 text-text-secondary transition-colors hover:text-text-primary"
      aria-label="تمرير للخلف"
      tabindex="-1"
      @click="scrollToward('start')"
    >
      <DirIcon :icon="dirIcon.back" class="size-4" />
    </button>
    <button
      v-if="canScrollEnd"
      type="button"
      class="scroll-fade-arrow absolute inset-y-0 end-0 flex items-center px-0.5 text-text-secondary transition-colors hover:text-text-primary"
      aria-label="تمرير للأمام"
      tabindex="-1"
      @click="scrollToward('end')"
    >
      <DirIcon :icon="dirIcon.forward" class="size-4" />
    </button>
  </div>
</template>

<style scoped>
.scroll-fade-track {
  scrollbar-width: none;
}
.scroll-fade-track::-webkit-scrollbar {
  display: none;
}

/* Gradients point from the transparent scrollable side toward the solid edge — the
   physical direction flips with reading direction, so it's driven by :dir(), not a
   hard-coded side. */
:dir(rtl) .scroll-fade-edge--start {
  background: linear-gradient(to left, var(--color-background), transparent);
}
:dir(rtl) .scroll-fade-edge--end {
  background: linear-gradient(to right, var(--color-background), transparent);
}
:dir(ltr) .scroll-fade-edge--start {
  background: linear-gradient(to right, var(--color-background), transparent);
}
:dir(ltr) .scroll-fade-edge--end {
  background: linear-gradient(to left, var(--color-background), transparent);
}
</style>
