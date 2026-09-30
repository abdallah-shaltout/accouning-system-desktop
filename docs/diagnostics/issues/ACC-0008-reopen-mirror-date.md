---
id: ACC-0008
kind: accounting
status: verified
area: accounting
first_seen: 2026-09-29
last_seen: 2026-09-29
occurrences: 1
debug_namespace: posting
regression_test: scripts/verify/cases/ACC-0008-reopen-mirror-date.json
---

## قيد عكس الإقفال عند إعادة فتح السنة كان مؤرخاً «الآن» (D-A2)

`reopenFiscalYear` كان يؤرخ قيد عكس الإقفال بتاريخ اليوم. إذا أُعيد فتح سنة 2025 في 2026 يقع القيد
في 2026: تبقى إيرادات ومصروفات 2025 صفراً بعد إعادة الفتح، وتظهر حركتها المعكوسة في السنة التالية
(قائمة دخل 2026 خاطئة). ولا يمكن إقفال 2025 مرة أخرى: حسابات النتيجة فيها صافيها صفر، فيُرفض قيد
الإقفال بـ «يجب أن يحتوي القيد على سطرين على الأقل».

القرار (D-A2 / Q4): قيد العكس يُؤرَّخ بتاريخ قيد الإقفال نفسه (تاريخ نهاية السنة)، بنفس
`allowClosedPeriod: true`، فتعود أرصدة السنة المعاد فتحها ولا تتأثر السنة التالية.

### خطوات إعادة الإنتاج

1. سنة 2025 مفتوحة فيها إيراد 5000 ومصروف 1200، وسنة 2026 موجودة.
2. إقفال 2025 ثم إعادة فتحها (مدير).
3. إقفال 2025 مرة أخرى: قبل الإصلاح يفشل (سطرين على الأقل)؛ بعده ينجح ويرحّل صافي الربح 3800 من جديد.

### ما جُرِّب ولم ينجح

- لا شيء؛ الإصلاح سطر واحد في كل جانب (`date: closing.date` في المحاكي، `closing.date()` في Rust).
- ملاحظة: `scripts/verify/replay.ts` لم يكن يفعّل Pinia، فكانت أي خدمة تقرأ `useAuthStore()` (مثل
  `reopenYear` و`canPostToClosedPeriod`) تفشل دائماً في الإعادة. صار يفعّل Pinia ويسجّل الدخول بأول
  مدير نشط في اللقطة.

### الملفات ذات الصلة

- `src/mocks/backend/core.ts` — `reopenFiscalYear`.
- `src-tauri/src/domains/accounting/service/period.rs` — `reopen_year`.
- `src-tauri/tests/domain_accounting_period.rs` — `reopen_year_mirror_is_dated_at_closing_entry_date_and_year_recloses`.
- `scripts/verify/replay.ts` — `restoreSession`.
- `plans/pending/21-rust-backend/03-domains/12b-period-close.md` §7 (Q4).
