export const staticValues = {
    minNameLength: 2,
    maxNameLength: 100,
    minDescriptionLength: 0,
    maxDescriptionLength: 2000,
    minSlugLength: 2,
    maxSlugLength: 100,
    minTitleLength: 2,
    maxTitleLength: 200,
    minQuantity: 0,
    maxQuantity: 1_000_000,
};

export const cacheConfig = {
    extraLow: 30,
    low: 60,
    medium: 5 * 60,
    high: 30 * 60,
    day: 24 * 60 * 60,
    month: 30 * 24 * 60 * 60,
};

export const sucessMessage = { status: "success", success: true } as const;
export const failMessage = { status: "fail", success: false } as const;
