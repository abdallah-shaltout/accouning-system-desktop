/**
 * Country data for `AppPhoneInput` and the party form's national-address section
 * (docs/v2/08-customers-and-suppliers.md §2 "Country data"). Ported in spirit from
 * `references/accouning-system/POS-Fares-webiste/app/assets/data/country.ts`, trimmed to the
 * countries this app's customers actually deal with and extended with an Arabic name + flag emoji.
 *
 * `dialDigits`/`nsnLength` describe the *national significant number* length used only for the
 * placeholder/max-length hint in the UI — actual E.164 validation goes through
 * `libphonenumber-js/min` (`AppPhoneInput.vue`), not this table.
 */
export interface CountryInfo {
  /** ISO 3166-1 alpha-2. */
  code: string;
  nameAr: string;
  nameEn: string;
  /** E.164 calling code, without "+". */
  dialCode: string;
  currency: string;
  flag: string;
  /** National number length (digits after the dial code), used for the placeholder/maxlength. */
  nsnLength: number;
  placeholder: string;
}

/**
 * Common countries first (docs/v2/08 §2 "order of the list"): SA, EG, AE, KW, QA, BH, OM, JO —
 * then the rest, alphabetical by dial code region.
 */
export const COUNTRIES: CountryInfo[] = [
  { code: 'SA', nameAr: 'السعودية', nameEn: 'Saudi Arabia', dialCode: '966', currency: 'SAR', flag: '🇸🇦', nsnLength: 9, placeholder: '5X XXX XXXX' },
  { code: 'EG', nameAr: 'مصر', nameEn: 'Egypt', dialCode: '20', currency: 'EGP', flag: '🇪🇬', nsnLength: 10, placeholder: '1X XXXX XXXX' },
  { code: 'AE', nameAr: 'الإمارات', nameEn: 'United Arab Emirates', dialCode: '971', currency: 'AED', flag: '🇦🇪', nsnLength: 9, placeholder: '5X XXX XXXX' },
  { code: 'KW', nameAr: 'الكويت', nameEn: 'Kuwait', dialCode: '965', currency: 'KWD', flag: '🇰🇼', nsnLength: 8, placeholder: 'XXXX XXXX' },
  { code: 'QA', nameAr: 'قطر', nameEn: 'Qatar', dialCode: '974', currency: 'QAR', flag: '🇶🇦', nsnLength: 8, placeholder: 'XXXX XXXX' },
  { code: 'BH', nameAr: 'البحرين', nameEn: 'Bahrain', dialCode: '973', currency: 'BHD', flag: '🇧🇭', nsnLength: 8, placeholder: 'XXXX XXXX' },
  { code: 'OM', nameAr: 'عُمان', nameEn: 'Oman', dialCode: '968', currency: 'OMR', flag: '🇴🇲', nsnLength: 8, placeholder: 'XXXX XXXX' },
  { code: 'JO', nameAr: 'الأردن', nameEn: 'Jordan', dialCode: '962', currency: 'JOD', flag: '🇯🇴', nsnLength: 9, placeholder: '7X XXX XXXX' },
  { code: 'LB', nameAr: 'لبنان', nameEn: 'Lebanon', dialCode: '961', currency: 'LBP', flag: '🇱🇧', nsnLength: 8, placeholder: 'XX XXX XXX' },
  { code: 'IQ', nameAr: 'العراق', nameEn: 'Iraq', dialCode: '964', currency: 'IQD', flag: '🇮🇶', nsnLength: 10, placeholder: '7XX XXX XXXX' },
  { code: 'YE', nameAr: 'اليمن', nameEn: 'Yemen', dialCode: '967', currency: 'YER', flag: '🇾🇪', nsnLength: 9, placeholder: '7XX XXX XXX' },
  { code: 'SY', nameAr: 'سوريا', nameEn: 'Syria', dialCode: '963', currency: 'SYP', flag: '🇸🇾', nsnLength: 9, placeholder: '9XX XXX XXX' },
  { code: 'PS', nameAr: 'فلسطين', nameEn: 'Palestine', dialCode: '970', currency: 'ILS', flag: '🇵🇸', nsnLength: 9, placeholder: '5XX XXX XXX' },
  { code: 'MA', nameAr: 'المغرب', nameEn: 'Morocco', dialCode: '212', currency: 'MAD', flag: '🇲🇦', nsnLength: 9, placeholder: '6XX XXXXXX' },
  { code: 'DZ', nameAr: 'الجزائر', nameEn: 'Algeria', dialCode: '213', currency: 'DZD', flag: '🇩🇿', nsnLength: 9, placeholder: '5XX XX XX XX' },
  { code: 'TN', nameAr: 'تونس', nameEn: 'Tunisia', dialCode: '216', currency: 'TND', flag: '🇹🇳', nsnLength: 8, placeholder: 'XX XXX XXX' },
  { code: 'LY', nameAr: 'ليبيا', nameEn: 'Libya', dialCode: '218', currency: 'LYD', flag: '🇱🇾', nsnLength: 9, placeholder: '9X XXX XXXX' },
  { code: 'SD', nameAr: 'السودان', nameEn: 'Sudan', dialCode: '249', currency: 'SDG', flag: '🇸🇩', nsnLength: 9, placeholder: '9X XXX XXXX' },
  { code: 'US', nameAr: 'الولايات المتحدة', nameEn: 'United States', dialCode: '1', currency: 'USD', flag: '🇺🇸', nsnLength: 10, placeholder: 'XXX XXX XXXX' },
  { code: 'GB', nameAr: 'المملكة المتحدة', nameEn: 'United Kingdom', dialCode: '44', currency: 'GBP', flag: '🇬🇧', nsnLength: 10, placeholder: 'XXXX XXXXXX' },
];

export const DEFAULT_COUNTRY_CODE = 'SA';

export function countryByCode(code: string | undefined | null): CountryInfo | undefined {
  return COUNTRIES.find((c) => c.code === code);
}

export function countryByDialCode(dialCode: string | undefined | null): CountryInfo | undefined {
  return COUNTRIES.find((c) => c.dialCode === dialCode);
}
