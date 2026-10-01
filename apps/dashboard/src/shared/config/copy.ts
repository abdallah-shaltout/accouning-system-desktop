/**
 * Shared Arabic copy (CLAUDE.md "UI" — "the shared Arabic copy ... lives in shared/config/copy.ts").
 * Calm, short, speaks to a shop owner or a staff member, not a developer. Never repeat these strings
 * in a page — import from here.
 */
export const copy = {
  empty: {
    default: "لا توجد بيانات لعرضها بعد",
    search: "لا توجد نتائج مطابقة لبحثك",
  },
  loading: "جارِ التحميل…",
  error: {
    generic: "حدث خطأ ما. حاول مرة أخرى",
    network: "تعذّر الاتصال بالخادم. تحقق من اتصالك بالإنترنت",
  },
  unsavedChanges: {
    title: "لديك تغييرات غير محفوظة",
    message: "هل تريد مغادرة الصفحة دون حفظ التغييرات؟",
    confirm: "مغادرة بدون حفظ",
    cancel: "البقاء في الصفحة",
  },
  confirmDelete: {
    title: "تأكيد الحذف",
    message: "لا يمكن التراجع عن هذا الإجراء. هل أنت متأكد؟",
    confirm: "حذف",
    cancel: "إلغاء",
  },
  sessionExpired: {
    title: "انتهت الجلسة",
    message: "الرجاء تسجيل الدخول مرة أخرى للمتابعة",
  },
  actions: {
    save: "حفظ",
    cancel: "إلغاء",
    retry: "إعادة المحاولة",
    back: "رجوع",
    logout: "تسجيل الخروج",
  },
} as const;
