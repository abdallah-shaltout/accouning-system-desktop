import { db } from '../db';
import { categoriesFixture, pharmaBatchesFixture, priceListsFixture, productsFixture, unitsFixture } from '../fixtures/catalog';
import { recordStockAdjustment } from '../backend/inventory';
import { clone, localDateKey } from '../utils';

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

  // v2 §3 demo: seed the pharmacy product's opening batches (one near-expiry, one further out) so
  // the expiry report and the product's "التشغيلات" tab have something to show out of the box.
  const panadol = productsFixture.find((p) => p.id === 'prd-panadol');
  if (panadol) {
    const now = new Date(date);
    recordStockAdjustment(
      {
        type: 'STOCK_IN',
        date,
        note: 'رصيد افتتاحي — بانادول 500 (تشغيلات)',
        reason: 'opening',
        lines: pharmaBatchesFixture.map((b) => {
          const expiry = new Date(now);
          expiry.setDate(expiry.getDate() + b.daysUntilExpiry);
          return { productId: panadol.id, qtyChange: b.qty, batchNo: b.batchNo, expiryDate: localDateKey(expiry) };
        }),
      },
      createdBy,
    );
  }
}
