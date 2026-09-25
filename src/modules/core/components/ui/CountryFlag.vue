<script setup lang="ts">
/**
 * Inline SVG flags (18.A1 #4) — Windows/WebView2 has no flag emoji glyphs, so `country.flag` emoji
 * render as literal "SA"/"EG" letters there. A tiny 3:2 SVG per country avoids a web font or an
 * icon-font dependency. `aria-hidden`: the accessible name lives on the parent control/option.
 */
import { computed } from 'vue';

const props = defineProps<{ code: string }>();

const code = computed(() => props.code.toUpperCase());
</script>

<template>
  <svg
    aria-hidden="true"
    viewBox="0 0 3 2"
    class="h-3.5 w-[21px] shrink-0 overflow-hidden rounded-[2px] ring-1 ring-black/10"
  >
    <!-- Saudi Arabia: green field (shahada + sword omitted for simplicity at this size) -->
    <template v-if="code === 'SA'">
      <rect width="3" height="2" fill="#006c35" />
    </template>
    <!-- Egypt: red / white / black horizontal bands with a stylised gold eagle dot -->
    <template v-else-if="code === 'EG'">
      <rect width="3" height="2" fill="#ce1126" />
      <rect y="0.667" width="3" height="0.667" fill="#fff" />
      <rect y="1.333" width="3" height="0.667" fill="#000" />
      <circle cx="1.5" cy="1" r="0.22" fill="#c09a3e" />
    </template>
    <!-- UAE: red hoist band + green/white/black horizontal bands -->
    <template v-else-if="code === 'AE'">
      <rect width="3" height="2" fill="#00732f" />
      <rect y="0.667" width="3" height="0.667" fill="#fff" />
      <rect y="1.333" width="3" height="0.667" fill="#000" />
      <rect width="0.8" height="2" fill="#ff0000" />
    </template>
    <!-- Kuwait: green / white / red bands + black trapezoid hoist -->
    <template v-else-if="code === 'KW'">
      <rect width="3" height="2" fill="#007a3d" />
      <rect y="0.667" width="3" height="0.667" fill="#fff" />
      <rect y="1.333" width="3" height="0.667" fill="#ce1126" />
      <path d="M0,0 L0.7,0 L0,1 Z" fill="#000" />
      <path d="M0,1 L0.7,2 L0,2 Z" fill="#000" />
    </template>
    <!-- Qatar: maroon field with white serrated hoist band -->
    <template v-else-if="code === 'QA'">
      <rect width="3" height="2" fill="#8d1b3d" />
      <rect width="0.9" height="2" fill="#fff" />
    </template>
    <!-- Bahrain: red field with white serrated hoist band -->
    <template v-else-if="code === 'BH'">
      <rect width="3" height="2" fill="#ce1126" />
      <rect width="0.9" height="2" fill="#fff" />
    </template>
    <!-- Oman: white / red / green horizontal bands + red hoist -->
    <template v-else-if="code === 'OM'">
      <rect width="3" height="2" fill="#fff" />
      <rect y="0.667" width="3" height="0.667" fill="#ce1126" />
      <rect y="1.333" width="3" height="0.667" fill="#008000" />
      <rect width="0.9" height="2" fill="#ce1126" />
    </template>
    <!-- Jordan: black / white / green bands + red triangle -->
    <template v-else-if="code === 'JO'">
      <rect width="3" height="2" fill="#000" />
      <rect y="0.667" width="3" height="0.667" fill="#fff" />
      <rect y="1.333" width="3" height="0.667" fill="#007a3d" />
      <path d="M0,0 L1.3,1 L0,2 Z" fill="#ce1126" />
    </template>
    <!-- Lebanon: red / white / red bands + green cedar dot -->
    <template v-else-if="code === 'LB'">
      <rect width="3" height="2" fill="#ee161f" />
      <rect y="0.5" width="3" height="1" fill="#fff" />
      <rect x="1.3" y="0.75" width="0.4" height="0.5" fill="#00522f" />
    </template>
    <!-- Iraq: red / white / black bands -->
    <template v-else-if="code === 'IQ'">
      <rect width="3" height="2" fill="#ce1126" />
      <rect y="0.667" width="3" height="0.667" fill="#fff" />
      <rect y="1.333" width="3" height="0.667" fill="#000" />
    </template>
    <!-- Yemen: red / white / black bands -->
    <template v-else-if="code === 'YE'">
      <rect width="3" height="2" fill="#ce1126" />
      <rect y="0.667" width="3" height="0.667" fill="#fff" />
      <rect y="1.333" width="3" height="0.667" fill="#000" />
    </template>
    <!-- Syria: red / white / black bands + two green stars -->
    <template v-else-if="code === 'SY'">
      <rect width="3" height="2" fill="#ce1126" />
      <rect y="0.667" width="3" height="0.667" fill="#fff" />
      <rect y="1.333" width="3" height="0.667" fill="#000" />
      <circle cx="1.2" cy="1" r="0.09" fill="#007a3d" />
      <circle cx="1.8" cy="1" r="0.09" fill="#007a3d" />
    </template>
    <!-- Palestine: black / white / green bands + red triangle -->
    <template v-else-if="code === 'PS'">
      <rect width="3" height="2" fill="#000" />
      <rect y="0.667" width="3" height="0.667" fill="#fff" />
      <rect y="1.333" width="3" height="0.667" fill="#007a3d" />
      <path d="M0,0 L1.3,1 L0,2 Z" fill="#ce1126" />
    </template>
    <!-- Morocco: red field + green pentagram -->
    <template v-else-if="code === 'MA'">
      <rect width="3" height="2" fill="#c1272d" />
      <circle cx="1.5" cy="1" r="0.28" fill="none" stroke="#006233" stroke-width="0.06" />
    </template>
    <!-- Algeria: green / white bands + red crescent and star -->
    <template v-else-if="code === 'DZ'">
      <rect width="1.5" height="2" fill="#006233" />
      <rect x="1.5" width="1.5" height="2" fill="#fff" />
      <circle cx="1.5" cy="1" r="0.3" fill="#d21034" />
      <circle cx="1.62" cy="1" r="0.25" fill="#fff" />
    </template>
    <!-- Tunisia: red field + white disc and red crescent -->
    <template v-else-if="code === 'TN'">
      <rect width="3" height="2" fill="#e70013" />
      <circle cx="1.5" cy="1" r="0.5" fill="#fff" />
      <circle cx="1.6" cy="1" r="0.38" fill="#e70013" />
      <circle cx="1.48" cy="1" r="0.3" fill="#fff" />
    </template>
    <!-- Libya: red / black / green bands + white crescent and star -->
    <template v-else-if="code === 'LY'">
      <rect width="3" height="2" fill="#000" />
      <rect y="0.4" width="3" height="1.2" fill="#000" />
      <rect y="0" width="3" height="0.4" fill="#e70013" />
      <rect y="1.6" width="3" height="0.4" fill="#239e46" />
      <circle cx="1.5" cy="1" r="0.32" fill="none" stroke="#fff" stroke-width="0.06" />
    </template>
    <!-- Sudan: red / white / black bands + green triangle -->
    <template v-else-if="code === 'SD'">
      <rect width="3" height="2" fill="#d21034" />
      <rect y="0.667" width="3" height="0.667" fill="#fff" />
      <rect y="1.333" width="3" height="0.667" fill="#000" />
      <path d="M0,0 L1.1,1 L0,2 Z" fill="#007229" />
    </template>
    <!-- US: simplified stripes + blue canton -->
    <template v-else-if="code === 'US'">
      <rect width="3" height="2" fill="#b22234" />
      <rect y="0.154" width="3" height="0.154" fill="#fff" />
      <rect y="0.462" width="3" height="0.154" fill="#fff" />
      <rect y="0.769" width="3" height="0.154" fill="#fff" />
      <rect y="1.077" width="3" height="0.154" fill="#fff" />
      <rect y="1.385" width="3" height="0.154" fill="#fff" />
      <rect y="1.692" width="3" height="0.154" fill="#fff" />
      <rect width="1.2" height="1.077" fill="#3c3b6e" />
    </template>
    <!-- UK: simplified union flag -->
    <template v-else-if="code === 'GB'">
      <rect width="3" height="2" fill="#012169" />
      <path d="M0,0 L3,2 M3,0 L0,2" stroke="#fff" stroke-width="0.3" />
      <path d="M0,0 L3,2 M3,0 L0,2" stroke="#c8102e" stroke-width="0.12" />
      <rect x="1.25" width="0.5" height="2" fill="#fff" />
      <rect y="0.75" width="3" height="0.5" fill="#fff" />
      <rect x="1.35" width="0.3" height="2" fill="#c8102e" />
      <rect y="0.85" width="3" height="0.3" fill="#c8102e" />
    </template>
    <!-- Fallback: neutral grey placeholder -->
    <template v-else>
      <rect width="3" height="2" fill="#a1a1aa" />
    </template>
  </svg>
</template>
