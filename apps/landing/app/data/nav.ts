export interface NavLink {
  label: string
  to: string
}

export const NAV_LINKS: NavLink[] = [
  { label: 'المميزات', to: '/#features' },
  { label: 'الشاشات', to: '/#screens' },
  { label: 'لماذا ايكوال', to: '/#why' },
  { label: 'الأسئلة', to: '/#faq' },
  { label: 'المدونة', to: '/blog' },
]

export const NAV_CTA = 'حمّل التطبيق'

export interface FooterColumn {
  title: string
  links: NavLink[]
}

export const FOOTER_COLUMNS: FooterColumn[] = [
  {
    title: 'المنتج',
    links: [
      { label: 'المميزات', to: '/#features' },
      { label: 'الشاشات', to: '/#screens' },
      { label: 'التحميل', to: '/#download' },
      { label: 'الأسئلة الشائعة', to: '/#faq' },
    ],
  },
  {
    title: 'مصادر',
    links: [
      { label: 'المدونة', to: '/blog' },
      { label: 'دليل الجرد', to: '/blog/jard-bidun-ighlaq' },
      { label: 'الفاتورة الضريبية', to: '/blog/alfatura-aldaribiya' },
      { label: 'لماذا بدون إنترنت', to: '/blog/leh-offline-aham-miza' },
    ],
  },
  {
    title: 'الشركة',
    links: [
      { label: 'عن ايكوال', to: '/#why' },
      { label: 'أنواع المحلات', to: '/#types' },
      { label: 'سياسة الخصوصية', to: '/privacy' },
      { label: 'شروط الاستخدام', to: '/terms' },
    ],
  },
  {
    title: 'تواصل معنا',
    links: [
      { label: 'البريد الإلكتروني', to: 'mailto:hello@equal-app.com' },
      { label: 'واتساب', to: '/#download' },
      { label: 'الدعم الفني', to: 'mailto:support@equal-app.com' },
    ],
  },
]

export const LEGAL_LINKS: NavLink[] = [
  { label: 'سياسة الخصوصية', to: '/privacy' },
  { label: 'شروط الاستخدام', to: '/terms' },
]
