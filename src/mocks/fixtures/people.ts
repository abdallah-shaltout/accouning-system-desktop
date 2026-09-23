import type { Customer, PartyGroup, Supplier } from '@/modules/parties/types';
import type { User } from '@/modules/users/types';

export const usersFixture: User[] = [
  { id: 'usr-1', username: 'admin', name: 'عبدالله الأحمد', phone: '+966501234567', role: 'admin', maxDiscount: 100, active: true },
  { id: 'usr-2', username: 'manager', name: 'سارة القحطاني', phone: '+966552345678', role: 'manager', maxDiscount: 30, active: true },
  { id: 'usr-3', username: 'accountant', name: 'محمد العتيبي', phone: '+966533456789', role: 'accountant', maxDiscount: 0, active: true },
  { id: 'usr-4', username: 'cashier', name: 'نورة الشهري', phone: '+966544567890', role: 'cashier', maxDiscount: 10, active: true },
  { id: 'usr-5', username: 'cashier2', name: 'خالد الزهراني', phone: '+966565678901', role: 'cashier', maxDiscount: 5, priceListId: 'pl-wholesale', active: true },
  { id: 'usr-6', username: 'fahad', name: 'فهد الدوسري', role: 'cashier', maxDiscount: 5, active: false },
  { id: 'usr-7', username: 'storekeeper', name: 'بندر السبيعي', phone: '+966576789012', role: 'storekeeper', maxDiscount: 0, active: true },
];

/** Demo credentials shown on the login screen. */
export const credentialsFixture: Record<string, string> = {
  admin: 'admin123',
  manager: 'manager123',
  accountant: 'acc123',
  cashier: 'cashier123',
  cashier2: 'cashier123',
  fahad: 'fahad123',
  storekeeper: 'store123',
};

/** Settings → Parties groups (docs/v2/08-customers-and-suppliers.md §5). */
export const partyGroupsFixture: PartyGroup[] = [
  { id: 'pg-retail', kind: 'customer', name: 'أفراد', paymentTermsDays: 0 },
  { id: 'pg-wholesale', kind: 'customer', name: 'جملة', priceListId: 'pl-wholesale', paymentTermsDays: 30, discountPercent: 5 },
  { id: 'pg-corporate', kind: 'customer', name: 'شركات', paymentTermsDays: 30 },
  { id: 'pg-vip', kind: 'customer', name: 'VIP', discountPercent: 10 },
  { id: 'pg-suppliers', kind: 'supplier', name: 'موردون محليون', paymentTermsDays: 30 },
];

export const customersFixture: Customer[] = [
  {
    id: 'cus-1', code: 'C-0001', name: 'أحمد بن سعيد الغامدي', type: 'individual', groupId: 'pg-retail',
    phone: '+966501112233', phones: [{ id: 'ph-1', label: 'mobile', number: '+966501112233' }],
    address: 'الرياض - حي الملقا', nationalAddress: { country: 'SA', city: 'الرياض', district: 'الملقا' },
    currency: 'SAR', paymentTermsDays: 0, creditLimit: 0, balance: 0, active: true,
  },
  {
    id: 'cus-2', code: 'C-0002', name: 'شركة النخبة للأزياء', nameEn: 'Al Nukhba Fashion Co.', type: 'company', groupId: 'pg-corporate',
    phone: '+966112223344', phones: [{ id: 'ph-2', label: 'work', number: '+966112223344' }],
    address: 'الرياض - طريق الملك فهد', nationalAddress: { country: 'SA', city: 'الرياض', district: 'العليا' },
    vatNumber: '300456789100003', crNumber: '1010123456', currency: 'SAR', paymentTermsDays: 30, creditLimit: 20000, balance: 0, active: true,
  },
  {
    id: 'cus-3', code: 'C-0003', name: 'ريم عبدالعزيز الحربي', type: 'individual', groupId: 'pg-retail',
    phone: '+966553334455', phones: [{ id: 'ph-3', label: 'mobile', number: '+966553334455' }],
    currency: 'SAR', paymentTermsDays: 0, balance: 0, active: true,
  },
  {
    id: 'cus-4', code: 'C-0004', name: 'مؤسسة الواحة التجارية', type: 'company', groupId: 'pg-corporate',
    phone: '+966124445566', phones: [{ id: 'ph-4', label: 'work', number: '+966124445566' }],
    address: 'جدة - حي الروضة', nationalAddress: { country: 'SA', city: 'جدة', district: 'الروضة' },
    vatNumber: '300987654300003', crNumber: '4030198765', currency: 'SAR', paymentTermsDays: 30, creditLimit: 15000, balance: 0, active: true,
  },
  {
    id: 'cus-5', code: 'C-0005', name: 'فيصل ناصر المطيري', type: 'individual', groupId: 'pg-wholesale',
    phone: '+966545556677', phones: [{ id: 'ph-5', label: 'mobile', number: '+966545556677' }, { id: 'ph-5w', label: 'whatsapp', number: '+966545556677' }],
    currency: 'SAR', paymentTermsDays: 30, creditLimit: 5000, balance: 0, active: true,
  },
  {
    id: 'cus-6', code: 'C-0006', name: 'هيا سليمان الدوسري', type: 'individual', groupId: 'pg-retail',
    phone: '+966566667788', phones: [{ id: 'ph-6', label: 'mobile', number: '+966566667788' }],
    address: 'الرياض - حي النرجس', nationalAddress: { country: 'SA', city: 'الرياض', district: 'النرجس' },
    currency: 'SAR', paymentTermsDays: 0, balance: 0, active: true,
  },
  {
    id: 'cus-7', code: 'C-0007', name: 'مدارس الرواد الأهلية', type: 'company', groupId: 'pg-corporate',
    phone: '+966117778899', phones: [{ id: 'ph-7', label: 'work', number: '+966117778899' }],
    address: 'الرياض - حي الياسمين', nationalAddress: { country: 'SA', city: 'الرياض', district: 'الياسمين' },
    vatNumber: '310222333400003', crNumber: '1010445566', currency: 'SAR', paymentTermsDays: 45, creditLimit: 30000, balance: 0, active: true,
  },
  {
    id: 'cus-8', code: 'C-0008', name: 'عمر خالد الشمري', type: 'individual', groupId: 'pg-retail',
    phone: '+966508889900', phones: [{ id: 'ph-8', label: 'mobile', number: '+966508889900' }],
    currency: 'SAR', paymentTermsDays: 0, balance: 0, active: true,
  },
  {
    id: 'cus-9', code: 'C-0009', name: 'لطيفة محمد العنزي', type: 'individual', groupId: 'pg-retail',
    phone: '+966559990011', phones: [{ id: 'ph-9', label: 'mobile', number: '+966559990011' }],
    currency: 'SAR', paymentTermsDays: 0, balance: 0, active: false,
  },
];

export const suppliersFixture: Supplier[] = [
  {
    id: 'sup-1', code: 'S-0001', name: 'مصنع النسيج الوطني', type: 'company', groupId: 'pg-suppliers',
    phone: '+966114441122', phones: [{ id: 'sph-1', label: 'work', number: '+966114441122' }],
    contactPerson: 'م. ياسر الفهد', address: 'الرياض - المدينة الصناعية الثانية', nationalAddress: { country: 'SA', city: 'الرياض' },
    vatNumber: '300111222300003', crNumber: '1010556677', currency: 'SAR', paymentTermsDays: 30,
    bank: { bankName: 'البنك الأهلي السعودي', iban: 'SA0380000000608010167519', accountName: 'مصنع النسيج الوطني' },
    balance: 0, active: true,
  },
  {
    id: 'sup-2', code: 'S-0002', name: 'شركة الأزياء الحديثة للتجارة', type: 'company', groupId: 'pg-suppliers',
    phone: '+966126662233', phones: [{ id: 'sph-2', label: 'work', number: '+966126662233' }],
    contactPerson: 'منى الحارثي', address: 'جدة - حي الصفا', nationalAddress: { country: 'SA', city: 'جدة' },
    vatNumber: '300333444500003', crNumber: '4030223344', currency: 'SAR', paymentTermsDays: 30,
    bank: { bankName: 'مصرف الراجحي', iban: 'SA4420000001234567891011', accountName: 'شركة الأزياء الحديثة' },
    balance: 0, active: true,
  },
  {
    id: 'sup-3', code: 'S-0003', name: 'مؤسسة الخليج للأحذية', type: 'company', groupId: 'pg-suppliers',
    phone: '+966138883344', phones: [{ id: 'sph-3', label: 'work', number: '+966138883344' }],
    contactPerson: 'سعد البقمي', address: 'الدمام - حي الفيصلية', nationalAddress: { country: 'SA', city: 'الدمام' },
    vatNumber: '300555666700003', currency: 'SAR', paymentTermsDays: 15, balance: 0, active: true,
  },
  {
    id: 'sup-4', code: 'S-0004', name: 'دار الإكسسوارات الراقية', type: 'company', groupId: 'pg-suppliers',
    phone: '+966115554455', phones: [{ id: 'sph-4', label: 'work', number: '+966115554455' }],
    contactPerson: 'نواف العمري', vatNumber: '300777888900003', currency: 'SAR', paymentTermsDays: 30, balance: 0, active: true,
  },
  {
    id: 'sup-5', code: 'S-0005', name: 'مستودعات الأطفال السعيدة', type: 'company', groupId: 'pg-suppliers',
    phone: '+966127775566', phones: [{ id: 'sph-5', label: 'work', number: '+966127775566' }],
    contactPerson: 'أمل الزهراني', address: 'جدة - حي البوادي', nationalAddress: { country: 'SA', city: 'جدة' },
    currency: 'SAR', paymentTermsDays: 15, balance: 0, active: true,
  },
];

/** Which supplier restocks which category (used by the seed's purchase generator). */
export const supplierByCategory: Record<string, string> = {
  'cat-men': 'sup-1',
  'cat-women': 'sup-2',
  'cat-kids': 'sup-5',
  'cat-shoes': 'sup-3',
  'cat-acc': 'sup-4',
};
