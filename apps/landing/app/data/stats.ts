export interface Stat {
  /** Final numeric value rendered in SSR markup (SEO/GEO-crawlable) and counted up to on scroll. */
  value: number
  prefix?: string
  suffix?: string
  label: string
}

/** Real product facts (plan decision D5): 28 report routes, 14 accounting invariants, local-only data. */
export const HERO_STATS: Stat[] = [
  { value: 28, prefix: '+', label: 'تقريرًا جاهزًا بنقرة' },
  { value: 14, label: 'فحصًا محاسبيًا تلقائيًا على كل قيد' },
  { value: 100, prefix: '%', label: 'من بياناتك تبقى على جهازك' },
]

export const TEAM_STATS: Stat[] = [
  { value: 4, label: 'أدوار جاهزة بصلاحياتها' },
  { value: 100, prefix: '+', label: 'شاشة عربية بالكامل' },
  { value: 17, prefix: '+', label: 'وحدة متخصصة' },
]
