//! `business_type.rs` (02-setup.md §3.5, `setup.ts:22-39`): seeds default units — only if the
//! catalog is still empty (a fresh company), so revisiting the step never clobbers owner edits.

use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, PaginatorTrait, Set};

use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::catalog::units::{ActiveModel as UnitActiveModel, Entity as UnitEntity};
use crate::utils::id::Id;

struct UnitSeed {
    name: &'static str,
    symbol: Option<&'static str>,
}

fn extra_units(business_type: &str) -> Vec<UnitSeed> {
    match business_type {
        "pharmacy" => vec![UnitSeed { name: "علبة", symbol: Some("box") }, UnitSeed { name: "شريط", symbol: Some("strip") }],
        "supermarket" => vec![UnitSeed { name: "كرتونة", symbol: Some("ctn") }],
        "services" => vec![UnitSeed { name: "خدمة", symbol: None }],
        "clothing" => vec![UnitSeed { name: "طقم", symbol: Some("set") }],
        _ => Vec::new(),
    }
}

/// `applyBusinessTypeDefaults` (`setup.ts:22-39`): no-op once `units` is non-empty.
pub async fn apply<C: ConnectionTrait>(conn: &C, cx: &TxCtx, business_type: &str) -> TxResult<()> {
    let existing = UnitEntity::find().count(conn).await.map_err(TxError::from)?;
    if existing > 0 {
        return Ok(());
    }

    let mut seeds = vec![UnitSeed { name: "قطعة", symbol: Some("pc") }];
    seeds.extend(extra_units(business_type));

    for (i, seed) in seeds.into_iter().enumerate() {
        let model = UnitActiveModel {
            id: Set(Id::new()),
            name: Set(seed.name.to_string()),
            symbol: Set(seed.symbol.map(|s| s.to_string())),
            allows_decimals: Set(None),
            created_at: Set(cx.clock.now + chrono::Duration::milliseconds(i as i64)),
            updated_at: Set(cx.clock.now),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
            name_live: sea_orm::ActiveValue::NotSet,
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}
