/**
 * Structured, country-aware address (doc 18 Phase E). Replaces `parties/types`' `NationalAddress`
 * (still exported there for now, deprecated — new code should use `Address`).
 *
 * Stores BOTH ids and names: the id lets the picker re-open pre-filled and lets a future backend
 * join against fresh geo data, while the name is a snapshot taken at save time so a printed
 * document never changes retroactively if the underlying geo dataset is refreshed later.
 */
import type { CountryCode } from '../helpers/countryProfiles';

/** Fields shared by every country, plus the per-country fields below. */
export interface Address {
  country: CountryCode;

  regionId?: string;
  regionName?: string;
  cityId?: string;
  cityName?: string;
  districtId?: string;
  districtName?: string;

  /** Free-text fallback when a level isn't in the picked list ("غير موجود في القائمة؟ اكتب يدوياً"). */
  regionFreeText?: string;
  cityFreeText?: string;
  districtFreeText?: string;

  // EG fields (mirrors the ETA address shape so e-invoicing later needs no migration).
  street?: string;
  buildingNo?: string;
  floor?: string;
  apartment?: string;
  /** علامة مميزة */
  landmark?: string;
  /** EG postal code is optional. */
  postalCode?: string;

  // SA fields.
  /** 4 digits. */
  saBuildingNo?: string;
  /** 4 digits ("الرقم الإضافي"). */
  saAdditionalNo?: string;
  /** 5 digits. */
  saPostalCode?: string;
  saUnitNo?: string;
  /** `AAAA9999` pattern ("العنوان المختصر"). */
  saShortAddress?: string;
}

/** True when the address has enough content to be worth saving/printing. */
export function hasAddressContent(addr: Address | undefined | null): boolean {
  if (!addr) return false;
  const { country, ...rest } = addr;
  return Object.values(rest).some((v) => typeof v === 'string' && v.trim().length > 0);
}
