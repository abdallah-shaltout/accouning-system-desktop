/** Global constants shared across modules. Add here, never re-declare locally. */

export const DEFAULT_PAGE_SIZE = 20;
export const PAGE_SIZE_OPTIONS = [10, 20, 50, 100] as const;

/** Default currency code for `MoneyText` when the API doesn't send one. */
export const DEFAULT_CURRENCY = "EGP";
