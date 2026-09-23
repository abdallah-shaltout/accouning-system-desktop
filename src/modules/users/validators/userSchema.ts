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
    phone: z.string().trim().regex(/^(05\d{8})?$/, 'رقم جوال سعودي يبدأ بـ 05 (10 أرقام)').optional(),
    role: z.enum(['admin', 'manager', 'accountant', 'cashier']),
    maxDiscount: z.number({ error: 'أدخل رقماً' }).min(0, 'لا يقل عن 0').max(100, 'لا يزيد عن 100'),
    password: isNew
      ? z.string().min(6, 'كلمة المرور 6 أحرف على الأقل')
      : z.union([z.literal(''), z.string().min(6, 'كلمة المرور 6 أحرف على الأقل')]).optional(),
  });
