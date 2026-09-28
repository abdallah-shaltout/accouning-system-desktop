//! `platform` table entities (21.02-B, owner B2). Declare each table as `pub mod <table>;`.
//! `print_templates` also lives here (see its own file's top-of-file note: no dedicated
//! `templates` group folder was pre-created).

pub mod activity;
pub mod approval_requests;
pub mod audit;
pub mod print_templates;

pub use activity::Entity as Activity;
pub use approval_requests::Entity as ApprovalRequests;
pub use audit::Entity as Audit;
pub use print_templates::Entity as PrintTemplates;
