import type { ComputedRef, Ref } from "vue"
import { createContext } from "reka-ui"

export const SIDEBAR_COOKIE_NAME = "sidebar_state"
export const SIDEBAR_COOKIE_MAX_AGE = 60 * 60 * 24 * 7
// docs/v2/16-equal-rebrand-and-ui-kit.md Phase D decision: 3.5rem icon width, rather than shadcn's
// own 3rem default. The expanded width is no longer fixed — the rail lets the user drag it within
// SIDEBAR_WIDTH_MIN/MAX_PX, persisted below (default 240px, ~15rem, matches the old fixed value).
export const SIDEBAR_WIDTH_MOBILE = "18rem"
export const SIDEBAR_WIDTH_ICON = "3.5rem"
export const SIDEBAR_KEYBOARD_SHORTCUT = "b"

export const SIDEBAR_WIDTH_MIN_PX = 192 // 12rem
export const SIDEBAR_WIDTH_MAX_PX = 320 // 20rem
const SIDEBAR_WIDTH_STORAGE_KEY = "app_sidebar_width_px"
const SIDEBAR_WIDTH_DEFAULT_PX = 240 // 15rem @ 16px root

function clampSidebarWidth(px: number): number {
  return Math.min(SIDEBAR_WIDTH_MAX_PX, Math.max(SIDEBAR_WIDTH_MIN_PX, px));
}

export function loadSidebarWidthPx(): number {
  try {
    const raw = localStorage.getItem(SIDEBAR_WIDTH_STORAGE_KEY);
    const n = raw === null ? Number.NaN : Number(raw);
    return Number.isFinite(n) ? clampSidebarWidth(n) : SIDEBAR_WIDTH_DEFAULT_PX;
  } catch {
    return SIDEBAR_WIDTH_DEFAULT_PX;
  }
}

export function saveSidebarWidthPx(px: number): number {
  const clamped = clampSidebarWidth(px);
  try {
    localStorage.setItem(SIDEBAR_WIDTH_STORAGE_KEY, String(clamped));
  } catch {
    /* ignore */
  }
  return clamped;
}

export const [useSidebar, provideSidebarContext] = createContext<{
  state: ComputedRef<"expanded" | "collapsed">
  open: Ref<boolean>
  setOpen: (value: boolean) => void
  isMobile: Ref<boolean>
  openMobile: Ref<boolean>
  setOpenMobile: (value: boolean) => void
  toggleSidebar: () => void
  widthPx: Ref<number>
  setWidthPx: (value: number) => void
}>("Sidebar")
