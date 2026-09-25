<script setup lang="ts">
/**
 * Equal brand mark (docs/v2/16-equal-rebrand-and-ui-kit.md Phase B). `variant="lockup"` shows the
 * full mark + wordmark (login/welcome/about); `variant="mark"` shows just the vector mark (sidebar,
 * compact spots — Phase D), colored via a CSS mask so it can pick up `currentColor` (an `<img
 * src="*.svg">` can't inherit page color — its SVG document is rendered in its own isolated
 * context). The lockup swaps source image by theme — the wordmark is baked into the raster art
 * itself, not recolored in CSS, since it's a designed lockup rather than a plain glyph. The mark is
 * the traced `mark.svg` (scripts/brand/trace-mark.py) rather than the source webp, so it stays
 * crisp at any size.
 */
import { computed } from 'vue';
import { resolvedTheme } from '@/modules/core/controllers/useTheme';
import logoLightBg from '@/assets/brand/logo-light-bg.webp';
import logoDarkBg from '@/assets/brand/logo-dark-bg.webp';
import markUrl from '@/assets/brand/mark.svg';
import { APP_NAME_AR } from '@/modules/core/helpers/brand';

const props = withDefaults(defineProps<{ variant?: 'lockup' | 'mark' }>(), { variant: 'lockup' });

const isMark = computed(() => props.variant === 'mark');
const src = computed(() => (isMark.value ? undefined : resolvedTheme.value === 'dark' ? logoDarkBg : logoLightBg));
const maskStyle = computed(() =>
  isMark.value
    ? { backgroundColor: 'currentColor', maskImage: `url(${markUrl})`, maskSize: '100% 100%', WebkitMaskImage: `url(${markUrl})`, WebkitMaskSize: '100% 100%' }
    : undefined,
);
</script>

<template>
  <span v-if="isMark" role="img" :aria-label="APP_NAME_AR" :style="maskStyle" />
  <img v-else :src="src" :alt="APP_NAME_AR" />
</template>
