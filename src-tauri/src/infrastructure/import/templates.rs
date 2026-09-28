//! Step 3.2.9 (`03-domains/00-import.md`): imports the `pdf_templates_v1` `localStorage` array
//! (D9 — moved from browser storage to `print_templates`, shared per branch).

use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait};
use serde::Deserialize;

use crate::core::error::AppError;
use crate::core::tx::{TxError, TxResult};
use crate::entities::platform::print_templates::{ActiveModel as PrintTemplateActiveModel, DocumentKind as EntityDocumentKind};
use crate::utils::id::Id;

/// The subset of `PdfTemplate` (`src/modules/templates/types/index.ts`) this importer reads —
/// `options` is copied through as opaque JSON (the entity column is `serde_json::Value` too), so a
/// future `TemplateOptions` field never needs a reader-model change here.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfTemplateV1 {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    pub kind: String,
    pub base_template_id: String,
    #[serde(default)]
    pub options: serde_json::Value,
    #[serde(default)]
    pub custom_source: Option<String>,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

fn parse_kind(kind: &str) -> Option<EntityDocumentKind> {
    Some(match kind {
        "invoice" => EntityDocumentKind::Invoice,
        "quotation" => EntityDocumentKind::Quotation,
        "creditNote" => EntityDocumentKind::CreditNote,
        "debitNote" => EntityDocumentKind::DebitNote,
        "purchaseOrder" => EntityDocumentKind::PurchaseOrder,
        "voucher" => EntityDocumentKind::Voucher,
        "statement" => EntityDocumentKind::Statement,
        "zReport" => EntityDocumentKind::ZReport,
        "transferNote" => EntityDocumentKind::TransferNote,
        "report" => EntityDocumentKind::Report,
        _ => return None,
    })
}

/// Parses `templates_json` (the raw `pdf_templates_v1` localStorage string) into the list of
/// templates to import — `None`/empty/unparseable → no templates (§3.1's `has_templates` /
/// §3.2.9's "absent/empty → none").
pub fn parse_templates(templates_json: Option<&str>) -> Vec<PdfTemplateV1> {
    let Some(json) = templates_json else { return Vec::new() };
    if json.trim().is_empty() {
        return Vec::new();
    }
    serde_json::from_str::<Vec<PdfTemplateV1>>(json).unwrap_or_default()
}

/// Inserts every parsed template as a `print_templates` row on `branch_id`. If two templates of the
/// same `kind` both carry `is_default = true`, only the first (array order) keeps it — the rest are
/// cleared and counted in `cleared_defaults` (the generated `default_key` unique, C-15; §3.2.9's own
/// rule). Returns the count of templates whose duplicate-default flag was cleared, for the caller's
/// log line.
pub async fn insert_templates<C: ConnectionTrait>(conn: &C, templates: &[PdfTemplateV1], branch_id: Id) -> TxResult<i64> {
    let mut seen_default_for_kind: std::collections::HashSet<EntityDocumentKind> = std::collections::HashSet::new();
    let mut cleared_defaults = 0i64;

    for tpl in templates {
        let Some(kind) = parse_kind(&tpl.kind) else {
            // An unrecognized kind string can't be inserted into the DB enum — skip rather than
            // fail the whole import over one bad row (the mock's own designer only ever writes the
            // 10 known kinds, so this only guards against a hand-edited/corrupt localStorage value).
            continue;
        };

        let mut is_default = tpl.is_default;
        if is_default {
            if seen_default_for_kind.contains(&kind) {
                is_default = false;
                cleared_defaults += 1;
            } else {
                seen_default_for_kind.insert(kind);
            }
        }

        let now = chrono::Utc::now();
        let created_at = tpl
            .created_at
            .as_deref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&chrono::Utc))
            .unwrap_or(now);

        let model = PrintTemplateActiveModel {
            id: Set(Id::new()),
            name: Set(tpl.name.clone()),
            kind: Set(kind),
            base_template_id: Set(tpl.base_template_id.clone()),
            options: Set(tpl.options.clone()),
            custom_source: Set(tpl.custom_source.clone()),
            is_default: Set(is_default),
            branch_id: Set(branch_id),
            created_at: Set(created_at),
            updated_at: Set(now),
            deleted_at: Set(None),
            sync_status: Set(crate::entities::platform::print_templates::SyncStatus::Local),
            default_key: sea_orm::ActiveValue::NotSet,
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }

    Ok(cleared_defaults)
}

/// The branch-picker validation rule (§3.2.9): more than one branch and no `template_branch_id`
/// given is a hard error — the UI must ask, never guess.
pub fn resolve_template_branch(
    branch_count: usize,
    template_branch_id: Option<Id>,
    only_branch: Option<Id>,
) -> Result<Option<Id>, AppError> {
    if template_branch_id.is_some() {
        return Ok(template_branch_id);
    }
    if branch_count <= 1 {
        return Ok(only_branch);
    }
    Err(AppError::validation("اختر الفرع الذي تنتمي إليه قوالب الطباعة"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_templates_handles_absent_and_empty() {
        assert!(parse_templates(None).is_empty());
        assert!(parse_templates(Some("")).is_empty());
        assert!(parse_templates(Some("   ")).is_empty());
        assert!(parse_templates(Some("not json")).is_empty());
        assert!(parse_templates(Some("[]")).is_empty());
    }

    #[test]
    fn parse_templates_reads_a_real_array() {
        let json = r#"[{"id":"tpl_1","name":"A","kind":"invoice","baseTemplateId":"invoice_standard","options":{},"isDefault":true}]"#;
        let parsed = parse_templates(Some(json));
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "A");
        assert!(parsed[0].is_default);
    }

    #[test]
    fn resolve_template_branch_requires_explicit_pick_for_multi_branch() {
        let a = Id::new();
        assert_eq!(resolve_template_branch(1, None, Some(a)).unwrap(), Some(a));
        assert!(resolve_template_branch(2, None, Some(a)).is_err());
        let b = Id::new();
        assert_eq!(resolve_template_branch(2, Some(b), Some(a)).unwrap(), Some(b));
    }
}
