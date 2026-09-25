import { ChevronLeft, ChevronRight } from '@lucide/vue';

/**
 * Logical navigation icons that must mirror under RTL (they point toward "back"/"forward" in
 * reading-direction terms, not a fixed screen side). Pages should use these instead of importing
 * ChevronLeft/ChevronRight/ArrowLeft/ArrowRight directly for anything meaning back/next/open/prev.
 *
 * LTR: back/prev point left, forward/next/open point right.
 * RTL: back/prev point right, forward/next/open point left — DirIcon flips automatically.
 */
export const dirIcon = {
  back: ChevronLeft,
  forward: ChevronRight,
  prev: ChevronLeft,
  next: ChevronRight,
  open: ChevronRight,
} as const;

export type DirIconName = keyof typeof dirIcon;
