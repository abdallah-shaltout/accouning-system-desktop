<script setup lang="ts">
import type { HTMLAttributes } from "vue"
import { cn } from '@/modules/core/helpers/utils'
import { SIDEBAR_WIDTH_MAX_PX, SIDEBAR_WIDTH_MIN_PX, useSidebar } from "./utils"

const props = defineProps<{
  class?: HTMLAttributes["class"]
}>()

const { widthPx, setWidthPx } = useSidebar()

// Drag-to-resize only — no click-to-toggle (that's what caused the accidental collapse the rail
// used to trigger on a plain click; collapsing now lives solely on SidebarTrigger in the header).
function onPointerDown(event: PointerEvent) {
  const rail = event.currentTarget as HTMLElement
  const side = rail.closest<HTMLElement>('[data-side]')?.dataset.side === 'left' ? 'left' : 'right'
  const startX = event.clientX
  const startWidth = widthPx.value
  rail.setPointerCapture(event.pointerId)

  function onPointerMove(moveEvent: PointerEvent) {
    const delta = moveEvent.clientX - startX
    // Right-side sidebar: dragging left (negative delta) grows it. Left-side: the opposite.
    const signedDelta = side === 'right' ? -delta : delta
    setWidthPx(startWidth + signedDelta)
  }

  function onPointerUp(upEvent: PointerEvent) {
    rail.releasePointerCapture(upEvent.pointerId)
    rail.removeEventListener('pointermove', onPointerMove)
    rail.removeEventListener('pointerup', onPointerUp)
  }

  rail.addEventListener('pointermove', onPointerMove)
  rail.addEventListener('pointerup', onPointerUp)
}
</script>

<template>
  <div
    data-sidebar="rail"
    data-slot="sidebar-rail"
    role="separator"
    aria-label="تغيير عرض القائمة الجانبية"
    aria-orientation="vertical"
    :aria-valuemin="SIDEBAR_WIDTH_MIN_PX"
    :aria-valuemax="SIDEBAR_WIDTH_MAX_PX"
    :aria-valuenow="widthPx"
    tabindex="-1"
    title="اسحب لتغيير العرض"
    :class="cn(
      'hover:after:bg-sidebar-border absolute inset-y-0 z-20 hidden w-4 -translate-x-1/2 transition-all ease-linear group-data-[side=left]:-right-4 group-data-[side=right]:left-0 after:absolute after:inset-y-0 after:left-1/2 after:w-0.5 sm:flex',
      'in-data-[side=left]:cursor-w-resize in-data-[side=right]:cursor-e-resize',
      'group-data-[collapsible=icon]:hidden group-data-[collapsible=offcanvas]:hidden',
      props.class,
    )"
    @pointerdown="onPointerDown"
  >
    <slot />
  </div>
</template>
