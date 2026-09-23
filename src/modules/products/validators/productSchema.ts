import { z } from 'zod';

const money = (label: string) => z.number({ error: `أدخل ${label}` }).min(0, `${label} لا يمكن أن يكون سالباً`);

export const productSchema = z
  .object({
    name: z.string().trim().min(2, 'اسم المنتج مطلوب'),
    sku: z.string().trim().min(1, 'رمز المنتج مطلوب'),
    barcode: z.string().trim().regex(/^\d{0,14}$/, 'الباركود أرقام فقط (حتى 14 رقماً)').optional(),
    type: z.enum(['product', 'service']),
    price: money('سعر البيع'),
    costPrice: money('سعر التكلفة'),
    minStock: z.number().min(0, 'لا يقل عن 0').optional(),
    openingQty: z.number().min(0, 'لا يقل عن 0').optional(),
  })
  .refine((p) => p.type === 'service' || p.price >= p.costPrice || p.price === 0, {
    message: 'سعر البيع أقل من التكلفة',
    path: ['price'],
  });

export const stockLineSchema = z.object({
  productId: z.string().min(1, 'اختر المنتج'),
  qty: z.number({ error: 'أدخل الكمية' }).positive('الكمية يجب أن تكون أكبر من صفر'),
});
