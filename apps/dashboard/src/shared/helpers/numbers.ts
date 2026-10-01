/** Money arrives from the API in piasters (1/100 of a unit) as a number, or a string when it could
 *  overflow a JS number (JSON has no real bigint). Converts to the major unit for display. */
export function piastersToMajor(piasters: number | string): number {
  const value = typeof piasters === "string" ? Number(piasters) : piasters;
  return value / 100;
}
