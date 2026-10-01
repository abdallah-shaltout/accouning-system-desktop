/**
 * The error shape every `apps/backend` failure response carries
 * (`apps/backend/src/shared/middleware/error/globalErrorHandling.ts`):
 * `{ status, success, statusCode, message, code?, action?, errors? }`.
 */
export interface ApiErrorShape {
  status: "warning" | "error" | "fail";
  success: false;
  statusCode: number;
  message: string;
  code?: string;
  action?: string | null;
  errors?: unknown;
}

/** Per-field validation errors, when `code === "validation_failed"`. */
export interface ApiFieldErrors {
  [field: string]: string[] | undefined;
}

/** A single Zod issue, as `validate()` in `apps/backend/.../validation.core.ts` attaches them raw to `errors`. */
interface ZodIssueLike {
  path: (string | number)[];
  message: string;
}

function isZodIssueArray(value: unknown): value is ZodIssueLike[] {
  return (
    Array.isArray(value) &&
    value.every((v) => v && typeof v === "object" && Array.isArray((v as ZodIssueLike).path) && typeof (v as ZodIssueLike).message === "string")
  );
}

/** Groups the backend's raw Zod issues array (`errors: [{ path: ["phone"], message: "..." }]`) by field. */
function fieldErrorsFromIssues(issues: ZodIssueLike[]): ApiFieldErrors {
  const grouped: ApiFieldErrors = {};
  for (const issue of issues) {
    const field = issue.path.join(".") || "_root";
    (grouped[field] ??= []).push(issue.message);
  }
  return grouped;
}

/** Stable error codes documented in `apps/backend/docs/05-api-spec.md` ("Error codes"). */
export type ApiErrorCode =
  | "validation_failed"
  | "unauthorized"
  | "forbidden"
  | "token_reused"
  | "not_found"
  | "conflict"
  | "invalid_transition"
  | "payment_already_reviewed"
  | "amount_mismatch"
  | "idempotency_conflict"
  | "rate_limited"
  | "account_required"
  | "credits_exhausted"
  | "not_creditable"
  | "device_limit"
  | "invalid_activation_code"
  | "activation_expired"
  | "license_revoked"
  | "org_suspended";

/**
 * Thrown by `shared/api/http.ts` for every failed request. Carries the backend's message as a
 * fallback, plus the stable `code` (when present) so callers can look up a calmer Arabic message via
 * `errorMessageFor`, and `fieldErrors` for form-level display.
 */
export class ApiError extends Error {
  readonly statusCode: number;
  readonly code?: string;
  readonly action?: string | null;
  readonly fieldErrors?: ApiFieldErrors;

  constructor(shape: ApiErrorShape) {
    super(shape.message);
    this.name = "ApiError";
    this.statusCode = shape.statusCode;
    this.code = shape.code;
    this.action = shape.action ?? null;
    this.fieldErrors = isZodIssueArray(shape.errors)
      ? fieldErrorsFromIssues(shape.errors)
      : isFieldErrors(shape.errors)
        ? shape.errors
        : undefined;
  }
}

function isFieldErrors(value: unknown): value is ApiFieldErrors {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * Short, calm Arabic messages for the stable error codes — speaks to a shop owner, not a developer
 * (05-ui-rules.md "Copy"). Falls back to the backend's own message (already Arabic) when the code is
 * missing or unrecognized, then to a generic message as a last resort.
 */
const ERROR_CODE_MESSAGES: Record<ApiErrorCode, string> = {
  validation_failed: "من فضلك راجع البيانات المدخلة",
  unauthorized: "البريد الإلكتروني أو كلمة المرور غير صحيحة",
  forbidden: "ليس لديك صلاحية للقيام بهذا الإجراء",
  token_reused: "انتهت صلاحية الجلسة. الرجاء تسجيل الدخول مرة أخرى",
  not_found: "العنصر المطلوب غير موجود",
  conflict: "هذا الإجراء يتعارض مع بيانات موجودة بالفعل",
  invalid_transition: "لا يمكن تنفيذ هذا الإجراء في الحالة الحالية",
  payment_already_reviewed: "تمت مراجعة هذه العملية من قبل",
  amount_mismatch: "المبلغ المدخل لا يطابق المطلوب",
  idempotency_conflict: "هذا الطلب تم إرساله بالفعل",
  rate_limited: "محاولات كثيرة جدًا. حاول مرة أخرى بعد قليل",
  account_required: "هذا الإجراء يحتاج حسابًا مرتبطًا بالمنشأة",
  credits_exhausted: "انتهى رصيد هذه الميزة لهذا الشهر",
  not_creditable: "هذه الميزة غير متاحة في باقتك الحالية",
  device_limit: "تم الوصول للحد الأقصى من الأجهزة. أوقف جهازًا آخر أولًا",
  invalid_activation_code: "كود التفعيل غير صحيح",
  activation_expired: "انتهت صلاحية كود التفعيل. اطلب كودًا جديدًا",
  license_revoked: "تم إلغاء ترخيص هذا الجهاز",
  org_suspended: "تم إيقاف هذه المنشأة مؤقتًا. تواصل مع الدعم",
};

const GENERIC_ERROR_MESSAGE = "حدث خطأ ما. حاول مرة أخرى";

/** Resolve the Arabic message to show for an `ApiError` — code map first, then the backend's own message. */
export function errorMessageFor(error: ApiError | ApiErrorCode | string | null | undefined): string {
  if (!error) return GENERIC_ERROR_MESSAGE;
  const code = typeof error === "string" ? error : error.code;
  if (code && code in ERROR_CODE_MESSAGES) {
    return ERROR_CODE_MESSAGES[code as ApiErrorCode];
  }
  if (typeof error !== "string" && error.message) return error.message;
  return GENERIC_ERROR_MESSAGE;
}
