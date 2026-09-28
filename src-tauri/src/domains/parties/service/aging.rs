//! `domains::parties::service::aging` — §3 `get_party_aging` (`partyService.ts:263-289`).

use sea_orm::ConnectionTrait;

use crate::core::tx::TxResult;
use crate::shared::balances::{self, OpenDocument};
use crate::utils::dates::BusinessClock;
use crate::utils::id::Id;
use crate::utils::money::round2;

use super::super::dto::{AgingBucket, AgingBucketKey, AgingDocument, PartyKind};

/// The 4 buckets in order, with their exact Arabic labels (U+2013 en-dash) — `AGING_BUCKETS`
/// (`partyService.ts:263-268`).
const BUCKET_DEFS: [(AgingBucketKey, &str); 4] =
    [(AgingBucketKey::Current, "حتى تاريخ الاستحقاق"), (AgingBucketKey::D30, "1–30 يوم"), (AgingBucketKey::D60, "31–60 يوم"), (AgingBucketKey::D90Plus, "90+ يوم")];

/// **`get_party_aging`** (`:276-289`): `today` = business clock's "today" (G-4); documents = the
/// party's open invoices/purchase orders (`shared::balances::open_invoices_for`/
/// `open_purchase_orders_for`, already ordered by date); each doc buckets by `days = today −
/// (due_date ?? date.day)`: `≤0 → current`, `≤30 → 1-30`, `≤60 → 31-60`, else `90+` (Q-4: the
/// "90+" bucket actually starts at 61 days, matching the mock's `daysOverdue <= 60 ? 2 : 3`); bucket
/// totals accumulate with `round2` applied **per step** (`partyService.ts:285`), not just once at
/// the end.
pub async fn get_party_aging<C: ConnectionTrait>(conn: &C, clock: &BusinessClock, kind: PartyKind, party_id: Id) -> TxResult<Vec<AgingBucket>> {
    let today = clock.today();

    let docs: Vec<OpenDocument> = match kind {
        PartyKind::Customer => balances::open_invoices_for(conn, party_id).await?,
        PartyKind::Supplier => balances::open_purchase_orders_for(conn, party_id).await?,
    };

    let mut buckets: Vec<AgingBucket> =
        BUCKET_DEFS.iter().map(|(key, label)| AgingBucket { key: *key, label: label.to_string(), total: rust_decimal::Decimal::ZERO, documents: Vec::new() }).collect();

    for doc in docs {
        let ref_day = doc.due_date.unwrap_or(doc.date.day);
        let days_overdue = (today - ref_day).num_days();
        let bucket_index = if days_overdue <= 0 {
            0
        } else if days_overdue <= 30 {
            1
        } else if days_overdue <= 60 {
            2
        } else {
            3
        };

        buckets[bucket_index].total = round2(buckets[bucket_index].total + doc.outstanding);
        buckets[bucket_index].documents.push(AgingDocument {
            id: doc.id,
            number: doc.number,
            date: doc.date.key(),
            due_date: doc.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
            outstanding: doc.outstanding,
        });
    }

    Ok(buckets)
}
