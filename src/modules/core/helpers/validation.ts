import { z } from 'zod';
import { countryProfile, type CountryCode } from './countryProfiles';

/**
 * Validate form state with a Zod schema and return `{ field: firstMessage }` (empty = valid).
 * Nested paths are joined with dots, e.g. `lines.2.qty`.
 */
export function validate<S extends z.ZodType>(schema: S, data: unknown): Record<string, string> {
  const result = schema.safeParse(data);
  if (result.success) return {};
  const errors: Record<string, string> = {};
  for (const issue of result.error.issues) {
    const key = issue.path.map(String).join('.');
    if (!errors[key]) errors[key] = issue.message;
  }
  return errors;
}

const optionalText = z.string().trim().optional();

/**
 * `Address` (doc 18.E) validation, driven by `countryProfiles.ts`'s `profile.address` shape — a
 * region/city are always required (the picker or its free-text fallback must produce one), the rest
 * follows the country's own field set. No field is hard-required beyond region/city: a party's
 * address is optional overall (see `hasAddressContent`), so this schema only fires once the user has
 * started filling it in (callers pass `.optional()` around the whole object when the section is empty).
 */
export function addressSchema(country: CountryCode | string | undefined) {
  const code = countryProfile(country).code;
  const base = {
    country: z.enum(['EG', 'SA']),
    regionId: optionalText,
    regionName: optionalText,
    regionFreeText: optionalText,
    cityId: optionalText,
    cityName: optionalText,
    cityFreeText: optionalText,
    districtId: optionalText,
    districtName: optionalText,
    districtFreeText: optionalText,
    street: optionalText,
    landmark: optionalText,
  };

  if (code === 'SA') {
    return z.object({
      ...base,
      saBuildingNo: z
        .string()
        .trim()
        .regex(/^(\d{4})?$/, '4 أرقام')
        .optional(),
      saAdditionalNo: z
        .string()
        .trim()
        .regex(/^(\d{4})?$/, '4 أرقام')
        .optional(),
      saPostalCode: z
        .string()
        .trim()
        .regex(/^(\d{5})?$/, '5 أرقام')
        .optional(),
      saUnitNo: optionalText,
      saShortAddress: z
        .string()
        .trim()
        .regex(/^([A-Za-z]{4}\d{4})?$/, 'الصيغة: AAAA9999')
        .optional(),
    });
  }

  return z.object({
    ...base,
    buildingNo: optionalText,
    floor: optionalText,
    apartment: optionalText,
    postalCode: optionalText,
  });
}
