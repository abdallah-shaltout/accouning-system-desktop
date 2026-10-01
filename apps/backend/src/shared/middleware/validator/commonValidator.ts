import { z } from "zod";

/** Money as bigint piasters, always positive-or-zero. */
export const moneyPiasters = z.coerce.bigint().nonnegative();

/** Egyptian phone number: 01[0125]XXXXXXXX. */
export const phoneEG = z
    .string()
    .regex(/^01[0125][0-9]{8}$/, "رقم الهاتف غير صالح");

export const uuidParam = z.object({
    id: z.string().uuid("المعرف غير صالح"),
});

export const paginationQuery = z.object({
    page: z.coerce.number().int().positive().optional(),
    limit: z.coerce.number().int().positive().max(100).optional(),
    sort: z.string().optional(),
    search: z.string().optional(),
    searchBy: z.string().optional(),
});

export function textField(opts: { min?: number; max?: number; fieldName?: string } = {}) {
    const { min = 1, max = 200 } = opts;
    return z
        .string()
        .min(min, `${opts.fieldName ?? "الحقل"} قصير جدًا`)
        .max(max, `${opts.fieldName ?? "الحقل"} طويل جدًا`);
}
