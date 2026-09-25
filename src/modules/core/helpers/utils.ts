import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

/** shadcn-vue's standard class-merging helper — every generated component imports this. */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
