//! `templates` service logic (03-domains/15-templates.md §3) — behaviour-exact port of
//! `src/modules/templates/services/templateService.ts`. Every step below cites its mock line
//! (`ts:<line>`) and the spec's own section (`§3.n`).

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::{map_unique_violation, AppError};
use crate::core::lock;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::platform::print_templates::{
    ActiveModel as TemplateActiveModel, Column, Entity as TemplateEntity, SyncStatus,
};
use crate::utils::dates::format_iso_ms;
use crate::utils::id::Id;

use super::dto::{BaseTemplateId, DocumentKind, PdfTemplate};

const DEFAULT_KEY_CONSTRAINT: &str = "uq_print_templates_default_key";

// --- 3.0 Helpers ---------------------------------------------------------------------------------

/// `settings.default_branch_id` (T-1) — every query in this domain is scoped to this branch.
async fn branch<C: ConnectionTrait>(conn: &C) -> TxResult<Id> {
    let settings = crate::core::settings::load(conn).await?;
    Ok(settings.default_branch_id)
}

/// A literal port of `defaultTemplateOptions()` (`tt:90-126`) — every key, in the same order and
/// with the same default values, built directly as a `serde_json::Value` so it matches the JSON the
/// mock would have produced byte-for-byte in shape (key order is irrelevant to JSON equality, only
/// values matter — the fixture test in `tests/domain_templates.rs` compares parsed values).
pub fn default_options() -> serde_json::Value {
    serde_json::json!({
        "accentColor": "#4f46e5",
        "logoPosition": "start",
        "logoSize": "m",
        "fontFamily": "Cairo",
        "fontSize": 10,
        "header": {
            "showCompanyName": true,
            "showAddress": true,
            "showVatNumber": true,
            "showCommercialRegister": true,
            "showPhone": true,
            "showEmail": false,
            "showWebsite": false,
            "title": "فاتورة ضريبية",
            "titleEn": "TAX INVOICE",
        },
        "columns": [
            { "key": "index", "label": "#", "visible": true },
            { "key": "name", "label": "الصنف", "visible": true },
            { "key": "qty", "label": "الكمية", "visible": true },
            { "key": "price", "label": "السعر", "visible": true },
            { "key": "discount", "label": "الخصم", "visible": false },
            { "key": "net", "label": "صافي", "visible": false },
            { "key": "vatRate", "label": "الضريبة %", "visible": true },
            { "key": "vat", "label": "الضريبة", "visible": true },
            { "key": "total", "label": "الإجمالي", "visible": true },
        ],
        "totals": {
            "showAmountInWords": true,
            "showBalance": false,
        },
        "footer": {
            "terms": "",
            "bankDetails": "",
            "showSignatureLines": false,
            "thankYouLine": "شكراً لتعاملكم معنا",
            "showPageNumbers": true,
        },
        "qr": {
            "position": "center",
            "size": "3cm",
        },
        "paper": "a4",
    })
}

fn options_as_map(value: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
    match value {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    }
}

/// Port of `load()`/`seedDefaults()` (`ts:21-67`, T-2): if the branch has no live template, seed
/// the two defaults. Every command calls this first, exactly as every mock function calls `load()`.
/// Concurrency (§4): locks the branch row, re-counts under that lock (READ COMMITTED sees a
/// concurrent seeder's commit), and only inserts if still empty.
pub async fn ensure_seeded<C: ConnectionTrait>(conn: &C, cx: &TxCtx, branch_id: Id) -> TxResult<()> {
    lock::for_update_by_id(conn, "branches", &branch_id.to_string()).await?;

    let count = TemplateEntity::find()
        .filter(Column::BranchId.eq(branch_id))
        .filter(Column::DeletedAt.is_null())
        .count(conn)
        .await?;
    if count > 0 {
        return Ok(());
    }

    let now = cx.clock.now;

    let standard_options = options_as_map(default_options());
    let standard = TemplateActiveModel {
        id: Set(Id::new()),
        name: Set("الفاتورة الضريبية القياسية".to_string()),
        kind: Set(DocumentKind::Invoice.into()),
        base_template_id: Set(BaseTemplateId::InvoiceStandard.as_str().to_string()),
        options: Set(serde_json::Value::Object(standard_options)),
        custom_source: Set(None),
        is_default: Set(true),
        branch_id: Set(branch_id),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set(SyncStatus::Local),
        default_key: sea_orm::ActiveValue::NotSet,
    };
    standard.insert(conn).await?;

    let mut simplified_options = options_as_map(default_options());
    if let Some(serde_json::Value::Object(header)) = simplified_options.get_mut("header") {
        header.insert("title".to_string(), serde_json::Value::String("فاتورة ضريبية مبسطة".to_string()));
        header.insert("titleEn".to_string(), serde_json::Value::String("SIMPLIFIED TAX INVOICE".to_string()));
    }
    let simplified = TemplateActiveModel {
        id: Set(Id::new()),
        name: Set("الفاتورة الضريبية المبسطة".to_string()),
        kind: Set(DocumentKind::Invoice.into()),
        base_template_id: Set(BaseTemplateId::InvoiceSimplified.as_str().to_string()),
        options: Set(serde_json::Value::Object(simplified_options)),
        custom_source: Set(None),
        is_default: Set(false),
        branch_id: Set(branch_id),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set(SyncStatus::Local),
        default_key: sea_orm::ActiveValue::NotSet,
    };
    simplified.insert(conn).await?;

    cx.touch(crate::core::events::ChangeCategory::Catalog);
    Ok(())
}

/// `find(id)` (§3.0): parse `id` as `Id`; unparsable or no live row in the branch -> `None` — the
/// mock's `find` -> `undefined` (T-4). Never `NOT_FOUND` for a lookup.
async fn find<C: ConnectionTrait>(conn: &C, branch_id: Id, id: &str) -> TxResult<Option<crate::entities::platform::print_templates::Model>> {
    let Ok(parsed) = id.parse::<Id>() else { return Ok(None) };
    let row = TemplateEntity::find_by_id(parsed)
        .filter(Column::BranchId.eq(branch_id))
        .filter(Column::DeletedAt.is_null())
        .one(conn)
        .await?;
    Ok(row)
}

fn to_dto(model: &crate::entities::platform::print_templates::Model) -> PdfTemplate {
    let base_template_id = BaseTemplateId::parse(&model.base_template_id).unwrap_or(BaseTemplateId::InvoiceStandard);
    PdfTemplate {
        id: model.id,
        name: model.name.clone(),
        kind: model.kind.clone().into(),
        base_template_id,
        options: options_as_map(model.options.clone()),
        custom_source: model.custom_source.clone(),
        is_default: model.is_default,
        created_at: format_iso_ms(model.created_at),
        updated_at: format_iso_ms(model.updated_at),
    }
}

// --- 3.1 list_templates --------------------------------------------------------------------------

pub async fn list_templates<C: ConnectionTrait>(conn: &C, cx: &TxCtx, kind: Option<DocumentKind>) -> TxResult<Vec<PdfTemplate>> {
    let branch_id = branch(conn).await?;
    ensure_seeded(conn, cx, branch_id).await?;

    let mut query = TemplateEntity::find()
        .filter(Column::BranchId.eq(branch_id))
        .filter(Column::DeletedAt.is_null());
    if let Some(kind) = kind {
        query = query.filter(Column::Kind.eq(crate::entities::platform::print_templates::DocumentKind::from(kind)));
    }
    let rows = query.order_by_asc(Column::CreatedAt).order_by_asc(Column::Id).all(conn).await?;
    Ok(rows.iter().map(to_dto).collect())
}

// --- 3.2 get_template ------------------------------------------------------------------------------

pub async fn get_template<C: ConnectionTrait>(conn: &C, cx: &TxCtx, id: &str) -> TxResult<Option<PdfTemplate>> {
    let branch_id = branch(conn).await?;
    ensure_seeded(conn, cx, branch_id).await?;
    let row = find(conn, branch_id, id).await?;
    Ok(row.as_ref().map(to_dto))
}

// --- 3.3 get_default_template ----------------------------------------------------------------------

pub async fn get_default_template<C: ConnectionTrait>(conn: &C, cx: &TxCtx, kind: DocumentKind) -> TxResult<Option<PdfTemplate>> {
    let branch_id = branch(conn).await?;
    ensure_seeded(conn, cx, branch_id).await?;

    let rows = TemplateEntity::find()
        .filter(Column::BranchId.eq(branch_id))
        .filter(Column::DeletedAt.is_null())
        .filter(Column::Kind.eq(crate::entities::platform::print_templates::DocumentKind::from(kind)))
        .order_by_asc(Column::CreatedAt)
        .order_by_asc(Column::Id)
        .all(conn)
        .await?;

    let chosen = rows.iter().find(|r| r.is_default).or_else(|| rows.first());
    Ok(chosen.map(to_dto))
}

// --- 3.4 save_template -----------------------------------------------------------------------------

pub async fn save_template<C: ConnectionTrait>(conn: &C, cx: &TxCtx, template: PdfTemplate) -> TxResult<PdfTemplate> {
    let branch_id = branch(conn).await?;
    ensure_seeded(conn, cx, branch_id).await?;

    let id_str = template.id.to_string();
    lock::for_update_by_id(conn, "print_templates", &id_str).await?;

    let existing = find(conn, branch_id, &id_str).await?.ok_or_else(|| AppError::not_found("القالب غير موجود"))?;

    // T-7: only design fields are updated; kind/isDefault/createdAt keep their stored values —
    // the default flag changes only through set_as_default, so a stale client copy can't break the
    // one-default rule.
    let mut active: TemplateActiveModel = existing.into();
    active.name = Set(template.name);
    active.base_template_id = Set(template.base_template_id.as_str().to_string());
    active.options = Set(serde_json::Value::Object(template.options));
    active.custom_source = Set(template.custom_source);
    active.updated_at = Set(cx.clock.now);

    let saved = active.update(conn).await?;
    cx.touch(crate::core::events::ChangeCategory::Catalog);
    Ok(to_dto(&saved))
}

// --- 3.5 set_as_default ----------------------------------------------------------------------------

pub async fn set_as_default<C: ConnectionTrait>(conn: &C, cx: &TxCtx, id: &str) -> TxResult<()> {
    let branch_id = branch(conn).await?;
    ensure_seeded(conn, cx, branch_id).await?;

    let Some(target) = find(conn, branch_id, id).await? else { return Ok(()) };

    // Lock every live row of (branch, target.kind), sorted by id (AT§5: the race between two
    // terminals setting two defaults of the same kind).
    let sibling_ids: Vec<String> = TemplateEntity::find()
        .filter(Column::BranchId.eq(branch_id))
        .filter(Column::Kind.eq(target.kind.clone()))
        .filter(Column::DeletedAt.is_null())
        .all(conn)
        .await?
        .into_iter()
        .map(|r| r.id.to_string())
        .collect();
    lock::for_update_many_sorted(conn, "print_templates", &sibling_ids).await?;

    let now = cx.clock.now;

    // Two statements, in this order (a single `SET is_default = (id = ?)` could briefly hold two
    // defaults and trip the generated-column unique mid-statement).
    let clear_model = TemplateActiveModel { is_default: Set(false), updated_at: Set(now), ..Default::default() };
    TemplateEntity::update_many()
        .set(clear_model)
        .filter(Column::BranchId.eq(branch_id))
        .filter(Column::Kind.eq(target.kind.clone()))
        .filter(Column::IsDefault.eq(true))
        .filter(Column::DeletedAt.is_null())
        .filter(Column::Id.ne(target.id))
        .exec(conn)
        .await
        .map_err(|e| {
            map_unique_violation(e, DEFAULT_KEY_CONSTRAINT, || "قيد التعديل من جهاز آخر".to_string())
        })?;

    let set_model = TemplateActiveModel { is_default: Set(true), updated_at: Set(now), ..Default::default() };
    TemplateEntity::update_many()
        .set(set_model)
        .filter(Column::Id.eq(target.id))
        .exec(conn)
        .await
        .map_err(|e| {
            map_unique_violation(e, DEFAULT_KEY_CONSTRAINT, || "قيد التعديل من جهاز آخر".to_string())
        })?;

    cx.touch(crate::core::events::ChangeCategory::Catalog);
    Ok(())
}

// --- 3.6 duplicate_template ------------------------------------------------------------------------

pub async fn duplicate_template<C: ConnectionTrait>(conn: &C, cx: &TxCtx, id: &str) -> TxResult<Option<PdfTemplate>> {
    let branch_id = branch(conn).await?;
    ensure_seeded(conn, cx, branch_id).await?;

    let Some(source) = find(conn, branch_id, id).await? else { return Ok(None) };
    let now = cx.clock.now;

    let copy = TemplateActiveModel {
        id: Set(Id::new()),
        name: Set(format!("{} (نسخة)", source.name)),
        kind: Set(source.kind.clone()),
        base_template_id: Set(source.base_template_id.clone()),
        options: Set(source.options.clone()),
        custom_source: Set(source.custom_source.clone()),
        is_default: Set(false),
        branch_id: Set(branch_id),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set(SyncStatus::Local),
        default_key: sea_orm::ActiveValue::NotSet,
    };
    let saved = copy.insert(conn).await?;
    cx.touch(crate::core::events::ChangeCategory::Catalog);
    Ok(Some(to_dto(&saved)))
}

// --- 3.7 delete_template ---------------------------------------------------------------------------

pub async fn delete_template<C: ConnectionTrait + Sync>(conn: &C, cx: &TxCtx, id: &str) -> TxResult<()> {
    let branch_id = branch(conn).await?;
    ensure_seeded(conn, cx, branch_id).await?;

    let Some(target) = find(conn, branch_id, id).await? else { return Ok(()) };

    use crate::entities::soft_delete::SoftDelete;
    TemplateEntity::soft_delete(conn, target.id, cx.clock.now).await?;

    cx.touch(crate::core::events::ChangeCategory::Catalog);
    Ok(())
}

// --- 3.8 reset_template_to_defaults ------------------------------------------------------------------

pub async fn reset_template_to_defaults<C: ConnectionTrait>(conn: &C, cx: &TxCtx, id: &str) -> TxResult<Option<PdfTemplate>> {
    let branch_id = branch(conn).await?;
    ensure_seeded(conn, cx, branch_id).await?;

    let Some(target) = find(conn, branch_id, id).await? else { return Ok(None) };

    let mut active: TemplateActiveModel = target.into();
    active.options = Set(default_options());
    active.custom_source = Set(None);
    active.updated_at = Set(cx.clock.now);

    let saved = active.update(conn).await?;
    cx.touch(crate::core::events::ChangeCategory::Catalog);
    Ok(Some(to_dto(&saved)))
}

// --- 3.9 import_template ---------------------------------------------------------------------------

/// Validates `json` by hand against `TemplateExport`'s shape (T-8): any failure ->
/// `VALIDATION "ملف القالب غير صالح"`, the mock's message and code (AT§3).
fn validate_import(json: &serde_json::Value) -> Option<(String, DocumentKind, BaseTemplateId, serde_json::Map<String, serde_json::Value>, Option<String>)> {
    let obj = json.as_object()?;
    if obj.get("schema")?.as_str()? != "pdf-template-v1" {
        return None;
    }
    let template = obj.get("template")?.as_object()?;

    let name = template.get("name")?.as_str()?.to_string();

    let kind_str = template.get("kind")?.as_str()?;
    let kind = parse_document_kind_camel(kind_str)?;

    let base_str = template.get("baseTemplateId")?.as_str()?;
    let base_template_id = BaseTemplateId::parse(base_str)?;

    let options = template.get("options")?.as_object()?.clone();

    let custom_source = match template.get("customSource") {
        None => None,
        Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        Some(_) => return None,
    };

    Some((name, kind, base_template_id, options, custom_source))
}

fn parse_document_kind_camel(s: &str) -> Option<DocumentKind> {
    Some(match s {
        "invoice" => DocumentKind::Invoice,
        "quotation" => DocumentKind::Quotation,
        "creditNote" => DocumentKind::CreditNote,
        "debitNote" => DocumentKind::DebitNote,
        "purchaseOrder" => DocumentKind::PurchaseOrder,
        "voucher" => DocumentKind::Voucher,
        "statement" => DocumentKind::Statement,
        "zReport" => DocumentKind::ZReport,
        "transferNote" => DocumentKind::TransferNote,
        "report" => DocumentKind::Report,
        _ => return None,
    })
}

pub async fn import_template<C: ConnectionTrait>(conn: &C, cx: &TxCtx, json: serde_json::Value) -> TxResult<PdfTemplate> {
    let branch_id = branch(conn).await?;
    ensure_seeded(conn, cx, branch_id).await?;

    let (name, kind, base_template_id, options, custom_source) =
        validate_import(&json).ok_or_else(|| AppError::validation("ملف القالب غير صالح"))?;

    let now = cx.clock.now;
    let row = TemplateActiveModel {
        id: Set(Id::new()),
        name: Set(name),
        kind: Set(kind.into()),
        base_template_id: Set(base_template_id.as_str().to_string()),
        options: Set(serde_json::Value::Object(options)),
        custom_source: Set(custom_source),
        is_default: Set(false),
        branch_id: Set(branch_id),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set(SyncStatus::Local),
        default_key: sea_orm::ActiveValue::NotSet,
    };
    let saved = row.insert(conn).await?;
    cx.touch(crate::core::events::ChangeCategory::Catalog);
    Ok(to_dto(&saved))
}

// --- 3.10 create_template --------------------------------------------------------------------------

pub async fn create_template<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    kind: DocumentKind,
    base_template_id: BaseTemplateId,
    name: String,
) -> TxResult<PdfTemplate> {
    let branch_id = branch(conn).await?;
    ensure_seeded(conn, cx, branch_id).await?;

    let now = cx.clock.now;
    let row = TemplateActiveModel {
        id: Set(Id::new()),
        name: Set(name),
        kind: Set(kind.into()),
        base_template_id: Set(base_template_id.as_str().to_string()),
        options: Set(default_options()),
        custom_source: Set(None),
        is_default: Set(false),
        branch_id: Set(branch_id),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set(SyncStatus::Local),
        default_key: sea_orm::ActiveValue::NotSet,
    };
    let saved = row.insert(conn).await?;
    cx.touch(crate::core::events::ChangeCategory::Catalog);
    Ok(to_dto(&saved))
}
