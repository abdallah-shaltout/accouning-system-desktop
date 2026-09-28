//! `templates` domain (03-domains/15-templates.md, D9) — print-template designer store, moved
//! from per-device `localStorage` to the branch DB. No undo compensator (design metadata, §5).

pub mod commands;
pub mod dto;
pub mod service;

use crate::core::ipc::IpcSig;
use crate::ipc_sig;

pub fn ipc_signatures() -> Vec<IpcSig> {
    vec![
        ipc_sig!(templates_list_templates, dto::TemplatesListTemplatesArgs, Vec<dto::PdfTemplate>),
        ipc_sig!(templates_get_template, dto::TemplatesGetTemplateArgs, Option<dto::PdfTemplate>),
        ipc_sig!(templates_get_default_template, dto::TemplatesGetDefaultTemplateArgs, Option<dto::PdfTemplate>),
        ipc_sig!(templates_save_template, dto::TemplatesSaveTemplateArgs, dto::PdfTemplate),
        ipc_sig!(templates_set_as_default, dto::TemplatesSetAsDefaultArgs, ()),
        ipc_sig!(templates_duplicate_template, dto::TemplatesDuplicateTemplateArgs, Option<dto::PdfTemplate>),
        ipc_sig!(templates_delete_template, dto::TemplatesDeleteTemplateArgs, ()),
        ipc_sig!(templates_reset_template_to_defaults, dto::TemplatesResetTemplateToDefaultsArgs, Option<dto::PdfTemplate>),
        ipc_sig!(templates_import_template, dto::TemplatesImportTemplateArgs, dto::PdfTemplate),
        ipc_sig!(templates_create_template, dto::TemplatesCreateTemplateArgs, dto::PdfTemplate),
    ]
}
