import type { Customer, Supplier } from '@/modules/parties/types';
import type { User } from '@/modules/users/types';

export const usersFixture: User[] = [
  { id: 'usr-1', username: 'admin', name: 'عبدالله الأحمد', phone: '0501234567', role: 'admin', maxDiscount: 100, active: true },
  { id: 'usr-2', username: 'manager', name: 'سارة القحطاني', phone: '0552345678', role: 'manager', maxDiscount: 30, active: true },
  { id: 'usr-3', username: 'accountant', name: 'محمد العتيبي', phone: '0533456789', role: 'accountant', maxDiscount: 0, active: true },
  { id: 'usr-4', username: 'cashier', name: 'نورة الشهري', phone: '0544567890', role: 'cashier', maxDiscount: 10, active: true },
  { id: 'usr-5', username: 'cashier2', name: 'خالد الزهراني', phone: '0565678901', role: 'cashier', maxDiscount: 5, priceListId: 'pl-wholesale', active: true },
  { id: 'usr-6', username: 'fahad', name: 'فهد الدوسري', role: 'cashier', maxDiscount: 5, active: false },
];

/** Demo credentials shown on the login screen. */
export const credentialsFixture: Record<string, string> = {
  admin: 'admin123',
  manager: 'manager123',
  accountant: 'acc123',
  cashier: 'cashier123',
  cashier2: 'cashier123',
  fahad: 'fahad123',
};

export const customersFixture: Customer[] = [
  { id: 'cus-1', name: 'أحمد بن سعيد الغامدي', phone: '0501112233', address: 'الرياض - حي الملقا', type: 'individual', balance: 0, active: true },
  { id: 'cus-2', name: 'شركة النخبة للأزياء', phone: '0112223344', address: 'الرياض - طريق الملك فهد', type: 'company', vatNumber: '300456789100003', balance: 0, active: true },
  { id: 'cus-3', name: 'ريم عبدالعزيز الحربي', phone: '0553334455', type: 'individual', balance: 0, active: true },
  { id: 'cus-4', name: 'مؤسسة الواحة التجارية', phone: '0124445566', address: 'جدة - حي الروضة', type: 'company', vatNumber: '300987654300003', balance: 0, active: true },
  { id: 'cus-5', name: 'فيصل ناصر المطيري', phone: '0545556677', type: 'individual', balance: 0, active: true },
  { id: 'cus-6', name: 'هيا سليمان الدوسري', phone: '0566667788', address: 'الرياض - حي النرجس', type: 'individual', balance: 0, active: true },
  { id: 'cus-7', name: 'مدارس الرواد الأهلية', phone: '0117778899', address: 'الرياض - حي الياسمين', type: 'company', vatNumber: '310222333400003', balance: 0, active: true },
  { id: 'cus-8', name: 'عمر خالد الشمري', phone: '0508889900', type: 'individual', balance: 0, active: true },
  { id: 'cus-9', name: 'لطيفة محمد العنزي', phone: '0559990011', type: 'individual', balance: 0, active: false },
];

export const suppliersFixture: Supplier[] = [
  { id: 'sup-1', name: 'مصنع النسيج الوطني', phone: '0114441122', contactPerson: 'م. ياسر الفهد', address: 'الرياض - المدينة الصناعية الثانية', vatNumber: '300111222300003', balance: 0, active: true },
  { id: 'sup-2', name: 'شركة الأزياء الحديثة للتجارة', phone: '0126662233', contactPerson: 'منى الحارثي', address: 'جدة - حي الصفا', vatNumber: '300333444500003', balance: 0, active: true },
  { id: 'sup-3', name: 'مؤسسة الخليج للأحذية', phone: '0138883344', contactPerson: 'سعد البقمي', address: 'الدمام - حي الفيصلية', vatNumber: '300555666700003', balance: 0, active: true },
  { id: 'sup-4', name: 'دار الإكسسوارات الراقية', phone: '0115554455', contactPerson: 'نواف العمري', vatNumber: '300777888900003', balance: 0, active: true },
  { id: 'sup-5', name: 'مستودعات الأطفال السعيدة', phone: '0127775566', contactPerson: 'أمل الزهراني', address: 'جدة - حي البوادي', balance: 0, active: true },
];

/** Which supplier restocks which category (used by the seed's purchase generator). */
export const supplierByCategory: Record<string, string> = {
  'cat-men': 'sup-1',
  'cat-women': 'sup-2',
  'cat-kids': 'sup-5',
  'cat-shoes': 'sup-3',
  'cat-acc': 'sup-4',
};
