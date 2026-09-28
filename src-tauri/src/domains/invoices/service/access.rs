//! `domains::invoices::service::access` — `require_any` re-export point (same pattern as
//! 11-expenses §1 / 03-users D-3): every command in this domain authorizes against one or more
//! areas via `TxCtx::require_any`/`ReadCtx::require_any` (G-6, `core/settings.rs`). This module has
//! no logic of its own — it exists so `commands.rs` reads `access::AREAS_*` constants naming which
//! areas each command accepts, matching 08 §1 / 08b §1's tables exactly.

use crate::core::auth::Area;

/// Sales reads reachable from POS, the desk invoice list, and party detail pages.
pub const SALES_POS_PARTIES_READ: &[(Area, crate::core::auth::Access)] =
    &[(Area::Sales, crate::core::auth::Access::Read), (Area::Pos, crate::core::auth::Access::Read), (Area::Parties, crate::core::auth::Access::Read)];

/// Sales reads reachable from POS only (no parties screen calls these).
pub const SALES_POS_READ: &[(Area, crate::core::auth::Access)] = &[(Area::Sales, crate::core::auth::Access::Read), (Area::Pos, crate::core::auth::Access::Read)];

/// Sales writes reachable from POS.
pub const SALES_POS_WRITE: &[(Area, crate::core::auth::Access)] = &[(Area::Sales, crate::core::auth::Access::Write), (Area::Pos, crate::core::auth::Access::Write)];

/// Print data: sales, POS and settings (the printing-settings test-print screen).
pub const SALES_POS_SETTINGS_READ: &[(Area, crate::core::auth::Access)] = &[
    (Area::Sales, crate::core::auth::Access::Read),
    (Area::Pos, crate::core::auth::Access::Read),
    (Area::Settings, crate::core::auth::Access::Read),
];

/// Desk-only sales reads/writes (quotations).
pub const SALES_READ: &[(Area, crate::core::auth::Access)] = &[(Area::Sales, crate::core::auth::Access::Read)];
pub const SALES_WRITE: &[(Area, crate::core::auth::Access)] = &[(Area::Sales, crate::core::auth::Access::Write)];

/// POS-only (shifts, held sales).
pub const POS_READ: &[(Area, crate::core::auth::Access)] = &[(Area::Pos, crate::core::auth::Access::Read)];
pub const POS_WRITE: &[(Area, crate::core::auth::Access)] = &[(Area::Pos, crate::core::auth::Access::Write)];

/// `getCurrentShift` is also read from the dashboard (`CashierHome`).
pub const POS_DASHBOARD_READ: &[(Area, crate::core::auth::Access)] =
    &[(Area::Pos, crate::core::auth::Access::Read), (Area::Dashboard, crate::core::auth::Access::Read)];
