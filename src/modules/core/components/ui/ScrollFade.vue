<script setup lang="ts">
/**
 * Wraps a horizontally-scrollable strip (tabs, chip rows, filter bars) with a hidden
 * native scrollbar, drag-to-scroll, wheel-to-horizontal-scroll, and edge fade gradients
 * that only appear on the side(s) that still have more content to reveal.
 */
import { onBeforeUnmount, onMounted, ref } from 'vue';

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

function updateFades() {
  const el = scroller.value;
  if (!el) return;
  const isRtl = getComputedStyle(el).direction === 'rtl';
  // scrollLeft can be negative (Firefox/Safari RTL) or reversed (Chrome legacy) —
  // measure "distance from start/end" independent of direction.
  const max = el.scrollWidth - el.clientWidth;
  if (max <= 1) {
    canScrollStart.value = false;
    canScrollEnd.value = false;
    return;
  }
  const fromLeft = Math.abs(el.scrollLeft);
  const fromStart = isRtl ? max - fromLeft : fromLeft;
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
  el.scrollLeft = pointerDownScrollLeft - delta;
}

function onPointerUp(e: PointerEvent) {
  if (activePointerId !== e.pointerId) return;
  const el = scroller.value;
  activePointerId = null;
  isDragging.value = false;
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
