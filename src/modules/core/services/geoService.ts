/**
 * Geo lookup service (doc 18.E — the seam for `AddressFields`). Backed today by the static, lazily
 * loaded per-country JSON files in `core/data/geo/` (`import()` per country so neither file sits in
 * the main bundle) — a real backend can replace these calls with a network request later without
 * touching `AddressFields` or any page.
 */
import { matchesPlaceSearch, normalizePlaceName } from '../helpers/search';
import type { CountryCode } from '../helpers/countryProfiles';

export interface GeoDistrict {
  id: string;
  ar: string;
  en: string;
}

export interface GeoCity {
  id: string;
  ar: string;
  en: string;
  districts: GeoDistrict[];
}

export interface GeoRegion {
  id: string;
  ar: string;
  en: string;
  cities: GeoCity[];
}

export interface GeoDataset {
  country: CountryCode;
  source: string;
  sourceCommit: string;
  license: string;
  generatedAt: string;
  labels: { region: string; city: string; district: string | null };
  regions: GeoRegion[];
}

export interface GeoPlaceMatch {
  kind: 'region' | 'city' | 'district';
  id: string;
  ar: string;
  en: string;
  /** Breadcrumb, e.g. "الرياض ← العليا" for a district match. */
  path: string;
  regionId: string;
  cityId?: string;
  districtId?: string;
}

const datasets = new Map<CountryCode, Promise<GeoDataset>>();

function loadDataset(country: CountryCode): Promise<GeoDataset> {
  let promise = datasets.get(country);
  if (!promise) {
    promise =
      country === 'SA'
        ? import('../data/geo/sa.json').then((m) => m.default as GeoDataset)
        : import('../data/geo/eg.json').then((m) => m.default as GeoDataset);
    datasets.set(country, promise);
  }
  return promise;
}

/** All regions (محافظة / منطقة) for a country, in dataset order (already sorted with `Intl.Collator('ar')`). */
export async function getRegions(country: CountryCode): Promise<GeoRegion[]> {
  const data = await loadDataset(country);
  return data.regions;
}

/** Cities within a region. Empty array (not an error) when the region id doesn't exist. */
export async function getCities(country: CountryCode, regionId: string | undefined): Promise<GeoCity[]> {
  if (!regionId) return [];
  const data = await loadDataset(country);
  return data.regions.find((r) => r.id === regionId)?.cities ?? [];
}

/** Districts within a city. Empty for EG today (no district level in the placeholder dataset) or an unknown city id. */
export async function getDistricts(country: CountryCode, cityId: string | undefined): Promise<GeoDistrict[]> {
  if (!cityId) return [];
  const data = await loadDataset(country);
  for (const region of data.regions) {
    const city = region.cities.find((c) => c.id === cityId);
    if (city) return city.districts;
  }
  return [];
}

/** The dataset's provenance labels — used by `AddressFields` to pick the right field label per level. */
export async function getLabels(country: CountryCode): Promise<GeoDataset['labels']> {
  return (await loadDataset(country)).labels;
}

/** Arabic-tolerant free search across all three levels (`AddressFields`' combobox filtering, and any future "search places" box). */
export async function searchPlaces(country: CountryCode, query: string): Promise<GeoPlaceMatch[]> {
  const q = normalizePlaceName(query);
  if (!q) return [];
  const data = await loadDataset(country);
  const out: GeoPlaceMatch[] = [];
  for (const region of data.regions) {
    if (matchesPlaceSearch([region.ar, region.en], query)) {
      out.push({ kind: 'region', id: region.id, ar: region.ar, en: region.en, path: region.ar, regionId: region.id });
    }
    for (const city of region.cities) {
      if (matchesPlaceSearch([city.ar, city.en], query)) {
        out.push({
          kind: 'city',
          id: city.id,
          ar: city.ar,
          en: city.en,
          path: `${region.ar} ← ${city.ar}`,
          regionId: region.id,
          cityId: city.id,
        });
      }
      for (const district of city.districts) {
        if (matchesPlaceSearch([district.ar, district.en], query)) {
          out.push({
            kind: 'district',
            id: district.id,
            ar: district.ar,
            en: district.en,
            path: `${region.ar} ← ${city.ar} ← ${district.ar}`,
            regionId: region.id,
            cityId: city.id,
            districtId: district.id,
          });
        }
      }
    }
  }
  return out.slice(0, 50);
}
