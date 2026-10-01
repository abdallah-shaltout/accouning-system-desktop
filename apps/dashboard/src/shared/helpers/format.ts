import { DEFAULT_CURRENCY } from "@/shared/config/constants";
import { piastersToMajor } from "./numbers";

const CURRENCY_LABELS: Record<string, string> = {
  EGP: "ج.م",
};

/** `formatMoney(29900, "EGP")` -> "299 ج.م" (used by `MoneyText`; never call this from a page directly). */
export function formatMoney(piasters: number | string, currency: string = DEFAULT_CURRENCY): string {
  const major = piastersToMajor(piasters);
  const amount = new Intl.NumberFormat("ar-EG", {
    minimumFractionDigits: 0,
    maximumFractionDigits: 2,
  }).format(major);
  const label = CURRENCY_LABELS[currency] ?? currency;
  return `${amount} ${label}`;
}

/** `formatDate("2026-09-30T12:00:00Z")` -> Arabic-locale date, no time. */
export function formatDate(iso: string | Date): string {
  const date = typeof iso === "string" ? new Date(iso) : iso;
  return new Intl.DateTimeFormat("ar-EG", { year: "numeric", month: "long", day: "numeric" }).format(date);
}

/** `formatDateTime("2026-09-30T12:00:00Z")` -> Arabic-locale date + time. */
export function formatDateTime(iso: string | Date): string {
  const date = typeof iso === "string" ? new Date(iso) : iso;
  return new Intl.DateTimeFormat("ar-EG", {
    year: "numeric",
    month: "long",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(date);
}
