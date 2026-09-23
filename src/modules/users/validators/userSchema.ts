import { z } from 'zod';

export const loginSchema = z.object({
  username: z.string().trim().min(1, 'أدخل اسم المستخدم'),
  password: z.string().min(1, 'أدخل كلمة المرور'),
});

export const userSchema = (isNew: boolean) =>
  z.object({
    name: z.string().trim().min(2, 'الاسم مطلوب'),
    username: z
      .string()
      .trim()
      .min(3, 'اسم المستخدم 3 أحرف على الأقل')
      .regex(/^[a-zA-Z0-9._-]+$/, 'أحرف إنجليزية وأرقام فقط'),
    // Stored as E.164 by AppPhoneInput (docs/v2/08-customers-and-suppliers.md §2); the component
    // itself validates per-country, this just guards the shape.
    phone: z.string().trim().regex(/^(\+\d{6,15})?$/, 'رقم جوال غير صحيح').optional(),
    role: z.enum(['admin', 'manager', 'accountant', 'cashier', 'storekeeper']),
    maxDiscount: z.number({ error: 'أدخل رقماً' }).min(0, 'لا يقل عن 0').max(100, 'لا يزيد عن 100'),
    password: isNew
      ? z.string().min(6, 'كلمة المرور 6 أحرف على الأقل')
      : z.union([z.literal(''), z.string().min(6, 'كلمة المرور 6 أحرف على الأقل')]).optional(),
  });
