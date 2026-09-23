import { db } from '../db';
import { categoriesFixture, priceListsFixture, productsFixture, unitsFixture } from '../fixtures/catalog';
import { recordStockAdjustment } from '../backend/inventory';
import { clone } from '../utils';

/** Catalog area: categories, units, price lists and products (stock starts at 0). */
export function seedCatalog(): void {
  db.categories = clone(categoriesFixture);
  db.units = clone(unitsFixture);
  db.priceLists = clone(priceListsFixture);
  db.products = productsFixture.map(({ openingQty: _ignored, ...p }) => clone(p));
}

/** Posts the initial stock-in for every product with an opening quantity. Needs accounts + catalog seeded first. */
export function postOpeningStock(date: string, createdBy: string): void {
  recordStockAdjustment(
    {
      type: 'STOCK_IN',
      date,
      note: 'رصيد افتتاحي للمخزون',
      reason: 'opening',
      lines: productsFixture.filter((p) => p.openingQty > 0).map((p) => ({ productId: p.id, qtyChange: p.openingQty })),
    },
    createdBy,
  );
}
