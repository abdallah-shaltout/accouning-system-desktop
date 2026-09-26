/**
 * Doc 18.E migration helper: a party may have `structuredAddress` (new `Address` picker),
 * `nationalAddress` (deprecated `NationalAddress`, old seeded/legacy data) or only the plain
 * `address` string (older records/settings). Pick the best one and render a single printable line,
 * so print templates and party statements never need to branch on which shape a given record has.
 */
import { formatAddress } from '@/modules/core/helpers/format';
import { DEFAULT_COUNTRY, type CountryCode } from '@/modules/core/helpers/countryProfiles';
import type { Address } from '@/modules/core/types/address';
import type { NationalAddress, PartyCommon } from '../types';

function formatLegacyNationalAddress(a: NationalAddress): string {
  const parts = [a.street, a.district, a.city, a.buildingNo ? `مبنى ${a.buildingNo}` : undefined, a.postalCode].filter(
    (p): p is string => !!p && p.trim().length > 0,
  );
  return parts.join('، ');
}

/** Best-effort printable address line for a customer/supplier-shaped record. */
export function resolvePartyAddressLine(party: Pick<PartyCommon, 'address' | 'nationalAddress' | 'structuredAddress'> | undefined | null): string {
  if (!party) return '';
  if (party.structuredAddress) return formatAddress(party.structuredAddress);
  if (party.nationalAddress && Object.values(party.nationalAddress).some(Boolean)) return formatLegacyNationalAddress(party.nationalAddress);
  return party.address ?? '';
}

/**
 * doc 18.E migration (used by `PartyFormPage` on load): an old record only has the deprecated
 * `NationalAddress` (ids-less city/district strings) or the even older plain `address` string. Map
 * what we have into the new `Address`'s `*Name`-only fields (no id — there was never a picked
 * region/city to snapshot an id from), so nothing is lost and the record still renders/prints
 * correctly until someone re-picks it from the cascading combobox.
 */
export function migrateLegacyAddress(
  legacy: NationalAddress | undefined,
  plainAddress: string | undefined,
  fallbackCountry: CountryCode | undefined,
): Address {
  const country = (legacy?.country as CountryCode) || fallbackCountry || DEFAULT_COUNTRY;
  if (!legacy || !Object.values(legacy).some(Boolean)) {
    return plainAddress ? { country, street: plainAddress } : { country };
  }
  return {
    country,
    cityName: legacy.city,
    districtName: legacy.district,
    street: legacy.street,
    buildingNo: legacy.buildingNo,
    postalCode: legacy.postalCode,
    saBuildingNo: legacy.buildingNo,
    saAdditionalNo: legacy.additionalNo,
    saPostalCode: legacy.postalCode,
    saUnitNo: legacy.unitNo,
    saShortAddress: legacy.shortAddress,
  };
}
