export function sanitizeData<T = any>(doc: T, fields: (keyof T)[]): Partial<T> {
    const result: Partial<T> = {};
    for (const field of fields) {
        const value = (doc as any)?.[field];
        if (value !== undefined) {
            result[field] = value;
        }
    }
    return result;
}
