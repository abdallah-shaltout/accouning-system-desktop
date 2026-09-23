import type { Category, PriceList, Product, Unit } from '@/modules/products/types';

export const categoriesFixture: Category[] = [
  { id: 'cat-men', name: 'ملابس رجالية' },
  { id: 'cat-women', name: 'ملابس نسائية' },
  { id: 'cat-kids', name: 'ملابس أطفال' },
  { id: 'cat-shoes', name: 'أحذية' },
  { id: 'cat-acc', name: 'إكسسوارات' },
  { id: 'cat-services', name: 'خدمات' },
];

export const unitsFixture: Unit[] = [
  { id: 'unit-piece', name: 'قطعة' },
  { id: 'unit-pair', name: 'زوج' },
  { id: 'unit-set', name: 'طقم' },
  { id: 'unit-service', name: 'خدمة' },
];

export const priceListsFixture: PriceList[] = [
  { id: 'pl-wholesale', name: 'سعر الجملة', active: true },
  { id: 'pl-vip', name: 'كبار العملاء', active: true },
  { id: 'pl-staff', name: 'سعر الموظفين', active: false },
];

/** EAN-13 with the Saudi GS1 prefix (628) and a valid check digit. */
function ean13(n: number): string {
  const body = `6281234${String(n).padStart(5, '0')}`;
  const digits = body.split('').map(Number);
  const total = digits.reduce((acc, d, i) => acc + d * (i % 2 === 0 ? 1 : 3), 0);
  return body + ((10 - (total % 10)) % 10);
}

type Row = [name: string, categoryId: string, unitId: string, cost: number, price: number, minStock: number, openingQty: number];

const rows: Row[] = [
  ['قميص رجالي قطن كلاسيك', 'cat-men', 'unit-piece', 65, 129, 8, 40],
  ['بنطال جينز رجالي سليم', 'cat-men', 'unit-piece', 85, 179, 8, 35],
  ['تيشيرت بولو رجالي', 'cat-men', 'unit-piece', 38, 79, 12, 60],
  ['ثوب سعودي أبيض', 'cat-men', 'unit-piece', 120, 249, 10, 45],
  ['شماغ أحمر فاخر', 'cat-men', 'unit-piece', 70, 145, 10, 40],
  ['جاكيت شتوي رجالي', 'cat-men', 'unit-piece', 190, 389, 4, 14],
  ['بدلة رسمية رجالية', 'cat-men', 'unit-set', 520, 1099, 2, 6],
  ['عباية سوداء مطرزة', 'cat-women', 'unit-piece', 210, 449, 6, 25],
  ['فستان سهرة طويل', 'cat-women', 'unit-piece', 340, 699, 3, 10],
  ['بلوزة شيفون نسائية', 'cat-women', 'unit-piece', 55, 119, 8, 36],
  ['تنورة ميدي', 'cat-women', 'unit-piece', 60, 129, 6, 24],
  ['جاكيت جينز نسائي', 'cat-women', 'unit-piece', 110, 229, 5, 18],
  ['بيجامة قطنية نسائية', 'cat-women', 'unit-set', 48, 99, 8, 30],
  ['طرحة شيفون', 'cat-women', 'unit-piece', 18, 39, 15, 80],
  ['طقم أطفال صيفي', 'cat-kids', 'unit-set', 45, 95, 8, 30],
  ['فستان بناتي', 'cat-kids', 'unit-piece', 58, 119, 6, 22],
  ['بنطال أطفال رياضي', 'cat-kids', 'unit-piece', 28, 59, 10, 40],
  ['تيشيرت أطفال مطبوع', 'cat-kids', 'unit-piece', 18, 39, 12, 55],
  ['حذاء رياضي رجالي', 'cat-shoes', 'unit-pair', 160, 329, 5, 20],
  ['حذاء كعب نسائي', 'cat-shoes', 'unit-pair', 130, 269, 5, 16],
  ['صندل جلد رجالي', 'cat-shoes', 'unit-pair', 75, 159, 6, 24],
  ['حذاء أطفال مضيء', 'cat-shoes', 'unit-pair', 55, 115, 6, 20],
  ['حزام جلد طبيعي', 'cat-acc', 'unit-piece', 35, 79, 8, 30],
  ['محفظة جلد رجالية', 'cat-acc', 'unit-piece', 45, 99, 8, 28],
  ['حقيبة يد نسائية', 'cat-acc', 'unit-piece', 150, 319, 4, 12],
  ['نظارة شمسية', 'cat-acc', 'unit-piece', 60, 139, 5, 18],
  ['جوارب قطن (طقم 3)', 'cat-acc', 'unit-set', 9, 25, 20, 90],
  ['ساعة يد كلاسيك', 'cat-acc', 'unit-piece', 180, 379, 3, 8],
];

const SKU_PREFIX: Record<string, string> = {
  'cat-men': 'MEN',
  'cat-women': 'WOM',
  'cat-kids': 'KID',
  'cat-shoes': 'SHO',
  'cat-acc': 'ACC',
  'cat-services': 'SRV',
};

/** Products start with stockQty 0 — the seed posts `openingQty` as STOCK_IN adjustments. */
export const productsFixture: (Product & { openingQty: number })[] = [
  ...rows.map(([name, categoryId, unitId, costPrice, price, minStock, openingQty], i) => {
    const perCategory = rows.slice(0, i).filter((r) => r[1] === categoryId).length + 1;
    return {
      id: `prd-${i + 1}`,
      name,
      sku: `${SKU_PREFIX[categoryId]}-${String(perCategory).padStart(3, '0')}`,
      barcode: ean13(i + 1),
      categoryId,
      unitId,
      type: 'product' as const,
      costPrice,
      price,
      stockQty: 0,
      minStock,
      active: true,
      prices: [
        { priceListId: 'pl-wholesale', value: Math.round(price * 0.85) },
        { priceListId: 'pl-vip', value: Math.round(price * 0.92) },
      ],
      openingQty,
    };
  }),
  {
    id: 'prd-svc-1',
    name: 'تعديل مقاس (خياطة)',
    sku: 'SRV-001',
    categoryId: 'cat-services',
    unitId: 'unit-service',
    type: 'service',
    costPrice: 0,
    price: 25,
    stockQty: 0,
    active: true,
    openingQty: 0,
  },
  {
    id: 'prd-svc-2',
    name: 'تغليف هدايا',
    sku: 'SRV-002',
    categoryId: 'cat-services',
    unitId: 'unit-service',
    type: 'service',
    costPrice: 0,
    price: 15,
    stockQty: 0,
    active: true,
    openingQty: 0,
  },
];
