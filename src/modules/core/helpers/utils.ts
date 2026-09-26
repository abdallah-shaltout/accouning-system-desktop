import { type ClassValue, clsx } from 'clsx';
import { extendTailwindMerge } from 'tailwind-merge';

/**
 * Our custom `text-*` font-size utilities (design-system.css `@utility text-body` etc.) aren't in
 * tailwind-merge's built-in font-size scale, so by default it buckets them with `text-color`
 * instead — e.g. `text-primary-foreground text-body` silently drops the color class, since merge
 * treats both as the same "text color" group. Registering them under `font-size` keeps the two
 * groups separate so a size class never cancels out a color class (or vice versa).
 */
const twMerge = extendTailwindMerge({
  extend: {
    classGroups: {
      'font-size': ['text-caption', 'text-tiny', 'text-label', 'text-body', 'text-ui', 'text-lead', 'text-heading-sm', 'text-heading', 'text-stat'],
    },
  },
});

/** shadcn-vue's standard class-merging helper — every generated component imports this. */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
