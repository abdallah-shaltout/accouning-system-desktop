export interface Tax {
  id: string;
  name: string;
  rate: number;
  type: 'OUTPUT' | 'INPUT';
  isDefault: boolean;
  active: boolean;
}

export type PrinterMode = 'a4' | 'thermal';
export type ThermalWidth = 58 | 80;

export interface StoreSettings {
  storeName: string;
  logo?: string;
  currency: string;
  vatNumber?: string;
  defaultTaxId?: string;
  invoiceNumberPrefix: string;
  printer: {
    mode: PrinterMode;
    thermalWidthMm: ThermalWidth;
  };
  theme: 'light' | 'dark';
  /** Extensions shown on printed documents. */
  address?: string;
  phone?: string;
  commercialRegister?: string;
  receiptFooter?: string;
}
