//! `templates` domain DTOs (21.03.15, D9) — mirrors `src/modules/templates/types/index.ts`
//! exactly. See `15-templates.md` §2 for the field-by-field notes (T-4, T-5).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::utils::id::Id;

/// Separate from `entities::platform::print_templates::DocumentKind` (the DB enum) — this is the
/// wire/TS shape (`tt:13-23`). `From` both ways below.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "templates/types/gen/")]
pub enum DocumentKind {
    Invoice,
    Quotation,
    CreditNote,
    DebitNote,
    PurchaseOrder,
    Voucher,
    Statement,
    ZReport,
    TransferNote,
    Report,
}

impl From<DocumentKind> for crate::entities::platform::print_templates::DocumentKind {
    fn from(k: DocumentKind) -> Self {
        use crate::entities::platform::print_templates::DocumentKind as E;
        match k {
            DocumentKind::Invoice => E::Invoice,
            DocumentKind::Quotation => E::Quotation,
            DocumentKind::CreditNote => E::CreditNote,
            DocumentKind::DebitNote => E::DebitNote,
            DocumentKind::PurchaseOrder => E::PurchaseOrder,
            DocumentKind::Voucher => E::Voucher,
            DocumentKind::Statement => E::Statement,
            DocumentKind::ZReport => E::ZReport,
            DocumentKind::TransferNote => E::TransferNote,
            DocumentKind::Report => E::Report,
        }
    }
}

impl From<crate::entities::platform::print_templates::DocumentKind> for DocumentKind {
    fn from(k: crate::entities::platform::print_templates::DocumentKind) -> Self {
        use crate::entities::platform::print_templates::DocumentKind as E;
        match k {
            E::Invoice => DocumentKind::Invoice,
            E::Quotation => DocumentKind::Quotation,
            E::CreditNote => DocumentKind::CreditNote,
            E::DebitNote => DocumentKind::DebitNote,
            E::PurchaseOrder => DocumentKind::PurchaseOrder,
            E::Voucher => DocumentKind::Voucher,
            E::Statement => DocumentKind::Statement,
            E::ZReport => DocumentKind::ZReport,
            E::TransferNote => DocumentKind::TransferNote,
            E::Report => DocumentKind::Report,
        }
    }
}

/// Which built-in `.typ` file a template starts from (`tt:134-147`). Stored as the plain text in
/// `base_template_id VARCHAR(32)` — snake_case on the wire (T-4/§2), unlike `DocumentKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "templates/types/gen/")]
pub enum BaseTemplateId {
    InvoiceStandard,
    InvoiceSimplified,
    Quotation,
    CreditNote,
    DebitNote,
    PurchaseOrder,
    Voucher,
    Statement,
    ZReport,
    TransferNote,
    GenericReport,
    LabelSheet,
    LabelThermal,
}

impl BaseTemplateId {
    /// The exact `snake_case` text stored in `base_template_id` — matches `serde`'s own
    /// `rename_all = "snake_case"` rendering of each variant, kept as an explicit table so the
    /// stored text can be constructed without a serde round-trip in `service.rs`.
    pub fn as_str(self) -> &'static str {
        match self {
            BaseTemplateId::InvoiceStandard => "invoice_standard",
            BaseTemplateId::InvoiceSimplified => "invoice_simplified",
            BaseTemplateId::Quotation => "quotation",
            BaseTemplateId::CreditNote => "credit_note",
            BaseTemplateId::DebitNote => "debit_note",
            BaseTemplateId::PurchaseOrder => "purchase_order",
            BaseTemplateId::Voucher => "voucher",
            BaseTemplateId::Statement => "statement",
            BaseTemplateId::ZReport => "z_report",
            BaseTemplateId::TransferNote => "transfer_note",
            BaseTemplateId::GenericReport => "generic_report",
            BaseTemplateId::LabelSheet => "label_sheet",
            BaseTemplateId::LabelThermal => "label_thermal",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "invoice_standard" => BaseTemplateId::InvoiceStandard,
            "invoice_simplified" => BaseTemplateId::InvoiceSimplified,
            "quotation" => BaseTemplateId::Quotation,
            "credit_note" => BaseTemplateId::CreditNote,
            "debit_note" => BaseTemplateId::DebitNote,
            "purchase_order" => BaseTemplateId::PurchaseOrder,
            "voucher" => BaseTemplateId::Voucher,
            "statement" => BaseTemplateId::Statement,
            "z_report" => BaseTemplateId::ZReport,
            "transfer_note" => BaseTemplateId::TransferNote,
            "generic_report" => BaseTemplateId::GenericReport,
            "label_sheet" => BaseTemplateId::LabelSheet,
            "label_thermal" => BaseTemplateId::LabelThermal,
            _ => return None,
        })
    }
}

/// `PdfTemplate` (`tt:149-160`). `options` is opaque JSON (T-5) — a typed struct would silently
/// drop unknown/future keys on save. `customSource` is `string | null`, never omitted (no
/// `skip_serializing_none` on this struct, per §2).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "templates/types/gen/")]
pub struct PdfTemplate {
    #[ts(type = "string")]
    pub id: Id,
    pub name: String,
    pub kind: DocumentKind,
    pub base_template_id: BaseTemplateId,
    #[ts(type = "import('../index').TemplateOptions")]
    pub options: serde_json::Map<String, serde_json::Value>,
    #[ts(type = "string | null")]
    pub custom_source: Option<String>,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Args of `templates_import_template` — `json` is validated by hand (§3.9) against
/// `TemplateExport`'s shape so a bad file produces the mock's Arabic message instead of a serde
/// error.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "templates/types/gen/")]
pub struct TemplatesImportTemplateArgs {
    #[ts(type = "import('../index').TemplateExport")]
    pub json: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "templates/types/gen/")]
pub struct TemplatesListTemplatesArgs {
    #[serde(default)]
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<DocumentKind>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "templates/types/gen/")]
pub struct TemplatesGetTemplateArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "templates/types/gen/")]
pub struct TemplatesGetDefaultTemplateArgs {
    pub kind: DocumentKind,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "templates/types/gen/")]
pub struct TemplatesSaveTemplateArgs {
    pub template: PdfTemplate,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "templates/types/gen/")]
pub struct TemplatesSetAsDefaultArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "templates/types/gen/")]
pub struct TemplatesDuplicateTemplateArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "templates/types/gen/")]
pub struct TemplatesDeleteTemplateArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "templates/types/gen/")]
pub struct TemplatesResetTemplateToDefaultsArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "templates/types/gen/")]
pub struct TemplatesCreateTemplateArgs {
    pub kind: DocumentKind,
    pub base_template_id: BaseTemplateId,
    pub name: String,
}
