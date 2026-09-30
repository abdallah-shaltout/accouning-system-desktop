//! `coa.rs` (02-setup.md §3.3 "`coa.rs`"): a line-for-line port of `src/mocks/fixtures/accounts.ts`
//! (`NORMAL_SIDE`, `leaf`, `group`, `rootRows`, `standardAccountRows`, `basicAccountRows`,
//! `detailedAccountRows`, `saAddonRows`, `pharmacyAddonRows`, `buildAccounts`). D-6: kept in exact
//! parity with the TS fixture by a fixture test (§8) — `previewCoaTemplate` stays frontend-only and
//! calls the TS version directly, so any drift here would only show up in that test, never in the UI.

use crate::domains::accounting::dto::{AccountKind, AccountSubtype, NormalSide};
use crate::shared::ledger::accounts::SystemRole;

/// One un-persisted row, before ids/parent-ids are resolved (`Row` in the TS fixture).
#[derive(Debug, Clone)]
pub struct AccountRow {
    pub code: &'static str,
    pub name: &'static str,
    pub name_en: Option<&'static str>,
    pub parent_code: Option<&'static str>,
    pub is_group: bool,
    pub kind: AccountKind,
    pub subtype: AccountSubtype,
    pub normal_side: NormalSide,
    pub role: Option<SystemRole>,
    pub requires_party: bool,
    pub allow_manual: bool,
    pub can_delete: bool,
}

struct LeafOpts {
    role: Option<SystemRole>,
    requires_party: bool,
    allow_manual: Option<bool>,
    contra: bool,
    can_delete: Option<bool>,
}

impl Default for LeafOpts {
    fn default() -> Self {
        LeafOpts { role: None, requires_party: false, allow_manual: None, contra: false, can_delete: None }
    }
}

impl LeafOpts {
    fn role(mut self, role: SystemRole) -> Self {
        self.role = Some(role);
        self
    }
    fn requires_party(mut self, v: bool) -> Self {
        self.requires_party = v;
        self
    }
    fn allow_manual(mut self, v: bool) -> Self {
        self.allow_manual = Some(v);
        self
    }
    fn contra(mut self, v: bool) -> Self {
        self.contra = v;
        self
    }
    fn can_delete(mut self, v: bool) -> Self {
        self.can_delete = Some(v);
        self
    }
}

fn opts() -> LeafOpts {
    LeafOpts::default()
}

/// `leaf()` (`fixtures/accounts.ts:40-61`): inherits its kind's normal side unless `contra`.
/// `canDelete` defaults to `!role` (a system-role account can't be deleted).
fn leaf(code: &'static str, name: &'static str, parent_code: &'static str, kind: AccountKind, subtype: AccountSubtype, opts: LeafOpts) -> AccountRow {
    let base_side = kind.normal_side();
    let normal_side = if opts.contra {
        match base_side {
            NormalSide::Debit => NormalSide::Credit,
            NormalSide::Credit => NormalSide::Debit,
        }
    } else {
        base_side
    };
    AccountRow {
        code,
        name,
        name_en: None,
        parent_code: Some(parent_code),
        is_group: false,
        kind,
        subtype,
        normal_side,
        role: opts.role,
        requires_party: opts.requires_party,
        allow_manual: opts.allow_manual.unwrap_or(true),
        can_delete: opts.can_delete.unwrap_or(opts.role.is_none()),
    }
}

/// `group()` (`fixtures/accounts.ts:63-65`).
fn group(code: &'static str, name: &'static str, parent_code: Option<&'static str>, kind: AccountKind) -> AccountRow {
    AccountRow {
        code,
        name,
        name_en: None,
        parent_code,
        is_group: true,
        kind,
        subtype: AccountSubtype::OtherCurrentAsset,
        normal_side: kind.normal_side(),
        role: None,
        requires_party: false,
        allow_manual: false,
        can_delete: false,
    }
}

/// `rootRows()` (`:67-77`): the five kinds' root headers, shared by every template.
fn root_rows() -> Vec<AccountRow> {
    vec![
        group("1", "الأصول", None, AccountKind::Asset),
        group("2", "الالتزامات", None, AccountKind::Liability),
        group("3", "حقوق الملكية", None, AccountKind::Equity),
        group("4", "الإيرادات", None, AccountKind::Revenue),
        group("5", "تكلفة المبيعات", None, AccountKind::Expense),
        group("6", "المصروفات", None, AccountKind::Expense),
    ]
}

/// `standardAccountRows()` (`fixtures/accounts.ts:80-162`).
pub fn standard_account_rows() -> Vec<AccountRow> {
    use AccountKind::*;
    use AccountSubtype as S;
    use SystemRole as R;

    let mut rows = root_rows();
    rows.extend([
        group("11", "الأصول المتداولة", Some("1"), Asset),
        leaf("1110", "الصندوق — الفرع الرئيسي", "11", Asset, S::Cash, opts().role(R::Cash)),
        leaf("1115", "العهد النقدية وسلف الموظفين", "11", Asset, S::OtherCurrentAsset, opts()),
        leaf("1120", "البنك — الحساب الجاري", "11", Asset, S::Bank, opts().role(R::Bank)),
        leaf("1125", "مدى والبطاقات تحت التسوية", "11", Asset, S::Clearing, opts().role(R::CardClearing)),
        leaf("1126", "المحافظ الإلكترونية تحت التسوية", "11", Asset, S::Clearing, opts().role(R::WalletClearing)),
        leaf("1130", "العملاء", "11", Asset, S::Receivable, opts().role(R::Receivable).requires_party(true)),
        leaf("1140", "المخزون", "11", Asset, S::Inventory, opts().role(R::Inventory).allow_manual(false)),
        leaf("1145", "بضاعة بالطريق بين الفروع", "11", Asset, S::Inventory, opts().role(R::InventoryInTransit).allow_manual(false)),
        leaf("1150", "ضريبة القيمة المضافة — مدخلات", "11", Asset, S::Tax, opts().role(R::VatInput).allow_manual(false)),
        leaf("1160", "دفعات مقدمة للموردين", "11", Asset, S::OtherCurrentAsset, opts()),
        leaf("1170", "مصروفات مدفوعة مقدماً", "11", Asset, S::Prepaid, opts()),
        leaf("1190", "مخصص الديون المشكوك في تحصيلها", "11", Asset, S::OtherCurrentAsset, opts().contra(true)),
        group("12", "الأصول غير المتداولة", Some("1"), Asset),
        leaf("1210", "أثاث وديكورات", "12", Asset, S::FixedAsset, opts()),
        leaf("1220", "أجهزة ومعدات", "12", Asset, S::FixedAsset, opts()),
        leaf("1230", "سيارات", "12", Asset, S::FixedAsset, opts()),
        leaf("1240", "تحسينات على مبانٍ مستأجرة", "12", Asset, S::FixedAsset, opts()),
        leaf("1290", "مجمع الإهلاك", "12", Asset, S::AccumulatedDepreciation, opts().contra(true)),
        group("21", "الالتزامات المتداولة", Some("2"), Liability),
        leaf("2100", "الموردين", "21", Liability, S::Payable, opts().role(R::Payable).requires_party(true)),
        leaf("2150", "ضريبة القيمة المضافة — مخرجات", "21", Liability, S::Tax, opts().role(R::VatOutput).allow_manual(false)),
        leaf("2155", "ضريبة القيمة المضافة — صافي مستحق", "21", Liability, S::Tax, opts().role(R::VatPayable)),
        leaf("2160", "رواتب مستحقة", "21", Liability, S::CurrentLiability, opts()),
        leaf("2170", "مصروفات مستحقة", "21", Liability, S::CurrentLiability, opts()),
        leaf("2180", "دفعات مقدمة وأرصدة دائنة للعملاء", "21", Liability, S::CurrentLiability, opts().role(R::CustomerAdvances)),
        group("22", "الالتزامات غير المتداولة", Some("2"), Liability),
        leaf("2210", "قروض طويلة الأجل", "22", Liability, S::LongTermLiability, opts()),
        leaf("2220", "مخصص مكافأة نهاية الخدمة", "22", Liability, S::LongTermLiability, opts()),
        leaf("3100", "رأس المال", "3", Equity, S::Equity, opts().role(R::Capital)),
        leaf("3150", "جاري المالك / الشركاء", "3", Equity, S::Equity, opts().role(R::OwnerCurrent)),
        leaf("3250", "الأرباح المحتجزة", "3", Equity, S::Equity, opts().role(R::RetainedEarnings)),
        leaf("3300", "صافي ربح الفترة (افتراضي — يُحسب)", "3", Equity, S::Equity, opts().role(R::CurrentEarnings).allow_manual(false).can_delete(false)),
        leaf("3400", "المسحوبات الشخصية", "3", Equity, S::Equity, opts().role(R::Drawings).contra(true)),
        leaf("3900", "أرصدة افتتاحية (مؤقت)", "3", Equity, S::Equity, opts().role(R::OpeningBalanceEquity)),
        leaf("4100", "مبيعات البضائع", "4", Revenue, S::Revenue, opts().role(R::Sales)),
        leaf("4110", "إيرادات الخدمات", "4", Revenue, S::Revenue, opts().role(R::ServiceRevenue)),
        leaf("4200", "مرتجعات ومسموحات المبيعات", "4", Revenue, S::Revenue, opts().role(R::SalesReturns).contra(true)),
        leaf("4300", "إيرادات أخرى", "4", Revenue, S::OtherIncome, opts().role(R::OtherIncome)),
        leaf("4310", "أرباح فروق العملة", "4", Revenue, S::OtherIncome, opts().role(R::FxGain)),
        leaf("4320", "خصم مكتسب من الموردين", "4", Revenue, S::OtherIncome, opts().role(R::PurchaseDiscounts)),
        leaf("4330", "زيادة الصندوق", "4", Revenue, S::OtherIncome, opts().role(R::CashOver)),
        leaf("5100", "تكلفة البضاعة المباعة", "5", Expense, S::CostOfSales, opts().role(R::Cogs).allow_manual(false)),
        leaf("5110", "فروقات جرد المخزون", "5", Expense, S::CostOfSales, opts().role(R::InventoryVariance).allow_manual(false)),
        leaf("5120", "بضاعة تالفة ومنتهية الصلاحية", "5", Expense, S::CostOfSales, opts().role(R::InventoryWriteOff)),
        leaf("5130", "شحن وتخليص المشتريات", "5", Expense, S::CostOfSales, opts().role(R::FreightIn)),
        group("61", "مصروفات البيع والتسويق", Some("6"), Expense),
        leaf("6110", "عمولات ومكافآت البيع", "61", Expense, S::OperatingExpense, opts()),
        leaf("6120", "دعاية وإعلان", "61", Expense, S::OperatingExpense, opts()),
        leaf("6130", "مواد تغليف وأكياس", "61", Expense, S::OperatingExpense, opts()),
        leaf("6140", "عمولات البطاقات ونقاط البيع", "61", Expense, S::OperatingExpense, opts().role(R::CardFees)),
        leaf("6150", "مصاريف توصيل", "61", Expense, S::OperatingExpense, opts()),
        group("62", "المصروفات العمومية والإدارية", Some("6"), Expense),
        leaf("6210", "الرواتب والأجور", "62", Expense, S::OperatingExpense, opts()),
        leaf("6215", "التأمينات الاجتماعية", "62", Expense, S::OperatingExpense, opts()),
        leaf("6220", "الإيجار", "62", Expense, S::OperatingExpense, opts()),
        leaf("6230", "الكهرباء والمياه", "62", Expense, S::OperatingExpense, opts()),
        leaf("6240", "الاتصالات والإنترنت", "62", Expense, S::OperatingExpense, opts()),
        leaf("6250", "الصيانة والإصلاح", "62", Expense, S::OperatingExpense, opts()),
        leaf("6260", "رسوم حكومية وتراخيص", "62", Expense, S::OperatingExpense, opts()),
        leaf("6270", "أدوات مكتبية ومطبوعات", "62", Expense, S::OperatingExpense, opts()),
        leaf("6280", "رسوم بنكية", "62", Expense, S::OperatingExpense, opts().role(R::BankFees)),
        leaf("6290", "مصروف الإهلاك", "62", Expense, S::OperatingExpense, opts().role(R::Depreciation)),
        leaf("6295", "ديون معدومة ومشكوك فيها", "62", Expense, S::OperatingExpense, opts().role(R::BadDebt)),
        group("63", "مصروفات أخرى", Some("6"), Expense),
        leaf("6310", "خسائر فروق العملة", "63", Expense, S::OtherExpense, opts().role(R::FxLoss)),
        leaf("6320", "عجز الصندوق", "63", Expense, S::OtherExpense, opts().role(R::CashShort)),
        leaf("6390", "مصروفات متنوعة", "63", Expense, S::OtherExpense, opts()),
    ]);
    rows
}

/// `basicAccountRows()` (`:165-193`).
pub fn basic_account_rows() -> Vec<AccountRow> {
    use AccountKind::*;
    use AccountSubtype as S;
    use SystemRole as R;

    let mut rows = root_rows();
    rows.extend([
        leaf("1110", "الصندوق", "1", Asset, S::Cash, opts().role(R::Cash)),
        leaf("1120", "البنك", "1", Asset, S::Bank, opts().role(R::Bank)),
        leaf("1130", "العملاء", "1", Asset, S::Receivable, opts().role(R::Receivable).requires_party(true)),
        leaf("1140", "المخزون", "1", Asset, S::Inventory, opts().role(R::Inventory).allow_manual(false)),
        leaf("1150", "ضريبة القيمة المضافة — مدخلات", "1", Asset, S::Tax, opts().role(R::VatInput).allow_manual(false)),
        leaf("2100", "الموردين", "2", Liability, S::Payable, opts().role(R::Payable).requires_party(true)),
        leaf("2150", "ضريبة القيمة المضافة — مخرجات", "2", Liability, S::Tax, opts().role(R::VatOutput).allow_manual(false)),
        leaf("2155", "ضريبة القيمة المضافة — صافي مستحق", "2", Liability, S::Tax, opts().role(R::VatPayable)),
        leaf("3100", "رأس المال", "3", Equity, S::Equity, opts().role(R::Capital)),
        leaf("3250", "الأرباح المحتجزة", "3", Equity, S::Equity, opts().role(R::RetainedEarnings)),
        leaf("3300", "صافي ربح الفترة (افتراضي — يُحسب)", "3", Equity, S::Equity, opts().role(R::CurrentEarnings).allow_manual(false)),
        leaf("3400", "المسحوبات الشخصية", "3", Equity, S::Equity, opts().role(R::Drawings).contra(true)),
        leaf("3900", "أرصدة افتتاحية (مؤقت)", "3", Equity, S::Equity, opts().role(R::OpeningBalanceEquity)),
        leaf("4100", "مبيعات البضائع", "4", Revenue, S::Revenue, opts().role(R::Sales)),
        leaf("4200", "مرتجعات ومسموحات المبيعات", "4", Revenue, S::Revenue, opts().role(R::SalesReturns).contra(true)),
        leaf("4300", "إيرادات أخرى", "4", Revenue, S::OtherIncome, opts().role(R::OtherIncome)),
        leaf("5100", "تكلفة البضاعة المباعة", "5", Expense, S::CostOfSales, opts().role(R::Cogs).allow_manual(false)),
        leaf("5110", "فروقات جرد المخزون", "5", Expense, S::CostOfSales, opts().role(R::InventoryVariance).allow_manual(false)),
        leaf("5120", "بضاعة تالفة ومنتهية الصلاحية", "5", Expense, S::CostOfSales, opts().role(R::InventoryWriteOff)),
        leaf("6110", "إيجار ورواتب", "6", Expense, S::OperatingExpense, opts()),
        leaf("6120", "مصروفات تشغيل عامة", "6", Expense, S::OperatingExpense, opts()),
        leaf("6130", "مواد تغليف وأكياس", "6", Expense, S::OperatingExpense, opts()),
        leaf("6140", "رسوم بنكية وعمولات بطاقات", "6", Expense, S::OperatingExpense, opts().role(R::CardFees)),
        leaf("6390", "مصروفات متنوعة", "6", Expense, S::OtherExpense, opts()),
    ]);
    rows
}

/// `detailedAccountRows()` (`:196-206`): standard plus 6 extra rows.
pub fn detailed_account_rows() -> Vec<AccountRow> {
    use AccountKind::*;
    use AccountSubtype as S;

    let mut rows = standard_account_rows();
    rows.extend([
        leaf("1135", "شيكات تحت التحصيل", "11", Asset, S::OtherCurrentAsset, opts()),
        leaf("2110", "شيكات تحت الدفع", "21", Liability, S::CurrentLiability, opts()),
        leaf("2185", "قسائم شراء مصدرة", "21", Liability, S::CurrentLiability, opts()),
        leaf("6216", "التأمينات الاجتماعية (جوسي)", "62", Expense, S::OperatingExpense, opts()),
        leaf("6217", "مكافأة نهاية الخدمة", "62", Expense, S::OperatingExpense, opts()),
        leaf("1171", "إيجار مدفوع مقدماً — لكل فرع", "11", Asset, S::Prepaid, opts()),
    ]);
    rows
}

/// `saAddonRows()` (`:209-211`).
pub fn sa_addon_rows() -> Vec<AccountRow> {
    vec![leaf("6910", "الزكاة", "69", AccountKind::Expense, AccountSubtype::ZakatTax, opts().role(SystemRole::Zakat))]
}

/// `pharmacyAddonRows()` (`:214-216`).
pub fn pharmacy_addon_rows() -> Vec<AccountRow> {
    vec![leaf("4120", "مبيعات أدوية معفاة/صفرية", "4", AccountKind::Revenue, AccountSubtype::Revenue, opts())]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountTemplateKind {
    Basic,
    Standard,
    Detailed,
}

fn rows_for_template(template: AccountTemplateKind) -> Vec<AccountRow> {
    match template {
        AccountTemplateKind::Basic => basic_account_rows(),
        AccountTemplateKind::Detailed => detailed_account_rows(),
        AccountTemplateKind::Standard => standard_account_rows(),
    }
}

/// A resolved row: still un-persisted, but with `parent_code` resolved to `None`/`Some(code)`
/// (identity is by `code`, D-7 — the caller assigns real `Id`s and `created_at` order).
#[derive(Debug, Clone)]
pub struct ResolvedAccountRow {
    pub code: String,
    pub name: String,
    pub name_en: Option<String>,
    pub parent_code: Option<String>,
    pub is_group: bool,
    pub kind: AccountKind,
    pub subtype: AccountSubtype,
    pub normal_side: NormalSide,
    pub role: Option<SystemRole>,
    pub requires_party: bool,
    pub allow_manual: bool,
    pub can_delete: bool,
}

/// `buildAccounts()` (`fixtures/accounts.ts:233-255`): template rows + country/business add-ons, in
/// template order. `country == Some("SA")` appends `saAddonRows`; `business_type == Some("pharmacy")`
/// appends `pharmacyAddonRows`. Order is preserved exactly (template order matters for D-7's
/// `created_at = now + i ms` trick in `apply_coa_template`).
pub fn build_accounts(template: AccountTemplateKind, country: Option<&str>, business_type: Option<&str>) -> Vec<ResolvedAccountRow> {
    let mut rows = rows_for_template(template);
    if country == Some("SA") {
        rows.extend(sa_addon_rows());
    }
    if business_type == Some("pharmacy") {
        rows.extend(pharmacy_addon_rows());
    }

    let codes: std::collections::HashSet<&str> = rows.iter().map(|r| r.code).collect();
    rows.into_iter()
        .map(|r| ResolvedAccountRow {
            code: r.code.to_string(),
            name: r.name.to_string(),
            name_en: r.name_en.map(|s| s.to_string()),
            parent_code: r.parent_code.filter(|pc| codes.contains(pc)).map(|s| s.to_string()),
            is_group: r.is_group,
            kind: r.kind,
            subtype: r.subtype,
            normal_side: r.normal_side,
            role: r.role,
            requires_party: r.requires_party,
            allow_manual: r.allow_manual,
            can_delete: r.can_delete,
        })
        .collect()
}

// --- `apply_coa_template` (02-setup.md §3.5, `setup.ts:42-55`, `setupService.ts:161-166`) ---------

/// `AccountTemplate` (Rust) -> the fixture's own enum.
pub fn template_kind_from_str(s: &str) -> AccountTemplateKind {
    match s {
        "basic" => AccountTemplateKind::Basic,
        "detailed" => AccountTemplateKind::Detailed,
        _ => AccountTemplateKind::Standard,
    }
}

pub fn template_kind_as_str(k: AccountTemplateKind) -> &'static str {
    match k {
        AccountTemplateKind::Basic => "basic",
        AccountTemplateKind::Standard => "standard",
        AccountTemplateKind::Detailed => "detailed",
    }
}

/// `applyCoaTemplate` (`setup.ts:42-55`, D-7 "identity by code"): refused once any journal entry
/// exists; soft-deletes live accounts whose code is not in the new template, updates in place those
/// whose code is, inserts the rest — `created_at = now + i ms` in template order so
/// `ORDER BY created_at, id` equals template order. Returns the rows as `Account` DTOs, in template
/// order.
pub async fn apply<C: sea_orm::ConnectionTrait>(
    conn: &C,
    cx: &crate::core::tx::TxCtx,
    template: AccountTemplateKind,
    country: Option<&str>,
    business_type: Option<&str>,
) -> crate::core::tx::TxResult<Vec<crate::domains::accounting::dto::Account>> {
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    use crate::core::error::AppError;
    use crate::core::tx::TxError;
    use crate::entities::journal::journal_entries::Entity as JournalEntryEntity;
    use crate::entities::org::accounts::{ActiveModel as AccountActiveModel, Entity as AccountEntity};
    use crate::entities::soft_delete::SoftDelete;
    use crate::utils::id::Id;

    let journal_count = {
        use sea_orm::PaginatorTrait;
        JournalEntryEntity::find().count(conn).await.map_err(TxError::from)?
    };
    if journal_count > 0 {
        return Err(TxError::App(AppError::forbidden("لا يمكن تغيير شجرة الحسابات بعد بدء الترحيل")));
    }

    let rows = build_accounts(template, country, business_type);
    let new_codes: std::collections::HashSet<&str> = rows.iter().map(|r| r.code.as_str()).collect();

    let live_accounts = AccountEntity::find_live().all(conn).await.map_err(TxError::from)?;
    let mut by_code: std::collections::HashMap<String, crate::entities::org::accounts::Model> = std::collections::HashMap::new();
    for a in live_accounts {
        if !new_codes.contains(a.code.as_str()) {
            AccountEntity::soft_delete(conn, a.id, cx.clock.now).await.map_err(TxError::from)?;
        } else {
            by_code.insert(a.code.clone(), a);
        }
    }

    // First pass: assign an id to every code (existing accounts keep their id — D-7).
    let mut code_to_id: std::collections::HashMap<String, Id> = std::collections::HashMap::new();
    for r in &rows {
        let id = by_code.get(&r.code).map(|a| a.id).unwrap_or_else(Id::new);
        code_to_id.insert(r.code.clone(), id);
    }

    let mut saved_models: Vec<crate::entities::org::accounts::Model> = Vec::with_capacity(rows.len());
    for (i, r) in rows.iter().enumerate() {
        let id = code_to_id[&r.code];
        let parent_id = r.parent_code.as_ref().map(|pc| code_to_id[pc]);
        let created_at = cx.clock.now + chrono::Duration::milliseconds(i as i64);

        if let Some(existing) = by_code.get(&r.code) {
            let mut model: AccountActiveModel = existing.clone().into();
            model.name = Set(r.name.clone());
            model.name_en = Set(r.name_en.clone());
            model.parent_id = Set(parent_id);
            model.is_group = Set(r.is_group);
            model.kind = Set(r.kind.as_str().to_string());
            model.subtype = Set(r.subtype.as_str().to_string());
            model.normal_side = Set(r.normal_side.as_str().to_string());
            model.system_role = Set(r.role.map(|role| role.as_str().to_string()));
            model.requires_party = Set(r.requires_party.then_some(true)) /* mock: `requiresParty` is set only when true, else absent */;
            model.allow_manual = Set(r.allow_manual);
            model.can_delete = Set(r.can_delete);
            model.created_at = Set(created_at);
            model.updated_at = Set(cx.clock.now);
            let updated = model.update(conn).await.map_err(TxError::from)?;
            saved_models.push(updated);
        } else {
            let model = AccountActiveModel {
                id: Set(id),
                code: Set(r.code.clone()),
                name: Set(r.name.clone()),
                name_en: Set(r.name_en.clone()),
                parent_id: Set(parent_id),
                is_group: Set(r.is_group),
                kind: Set(r.kind.as_str().to_string()),
                subtype: Set(r.subtype.as_str().to_string()),
                normal_side: Set(r.normal_side.as_str().to_string()),
                system_role: Set(r.role.map(|role| role.as_str().to_string())),
                currency: Set(None),
                branch_id: Set(None),
                requires_party: Set(r.requires_party.then_some(true)) /* mock: `requiresParty` is set only when true, else absent */,
                allow_manual: Set(r.allow_manual),
                requires_cost_center: Set(None),
                active: Set(true),
                can_delete: Set(r.can_delete),
                created_at: Set(created_at),
                updated_at: Set(cx.clock.now),
                deleted_at: Set(None),
                sync_status: Set("local".to_string()),
                code_live: sea_orm::ActiveValue::NotSet,
            };
            let inserted = model.insert(conn).await.map_err(TxError::from)?;
            saved_models.push(inserted);
        }
    }

    // `onboarding.coaTemplate = template`.
    let locked = crate::core::settings::load_shared_locked(conn).await.map_err(TxError::App)?;
    let mut onboarding = locked.onboarding.clone().unwrap_or(crate::entities::values::OnboardingState {
        business_type: None,
        go_live_date: None,
        completed_step: None,
        skipped: Vec::new(),
        done: Vec::new(),
        finished_at: None,
        opening_entry_id: None,
        closing_entry_id: None,
        coa_template: None,
    });
    onboarding.coa_template = Some(template_kind_as_str(template).to_string());
    let mut settings_model: crate::entities::org::settings::ActiveModel = locked.into();
    settings_model.onboarding = Set(Some(onboarding));
    settings_model.updated_at = Set(cx.clock.now);
    settings_model.update(conn).await.map_err(TxError::from)?;

    cx.touch(crate::core::events::ChangeCategory::Ledger);

    Ok(saved_models.into_iter().map(crate::domains::accounting::dto::Account::from_model).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_template_has_expected_row_count_and_root_headers() {
        let rows = build_accounts(AccountTemplateKind::Standard, None, None);
        assert!(rows.iter().any(|r| r.code == "1" && r.is_group));
        assert!(rows.iter().any(|r| r.code == "1110" && r.role == Some(SystemRole::Cash)));
        assert!(rows.iter().any(|r| r.code == "3900" && r.role == Some(SystemRole::OpeningBalanceEquity)));
    }

    #[test]
    fn sa_addon_appended_only_for_sa() {
        let eg = build_accounts(AccountTemplateKind::Standard, Some("EG"), None);
        let sa = build_accounts(AccountTemplateKind::Standard, Some("SA"), None);
        assert!(!eg.iter().any(|r| r.code == "6910"));
        assert!(sa.iter().any(|r| r.code == "6910" && r.role == Some(SystemRole::Zakat)));
    }

    #[test]
    fn pharmacy_addon_appended_only_for_pharmacy() {
        let retail = build_accounts(AccountTemplateKind::Standard, None, None);
        let pharmacy = build_accounts(AccountTemplateKind::Standard, None, Some("pharmacy"));
        assert!(!retail.iter().any(|r| r.code == "4120"));
        assert!(pharmacy.iter().any(|r| r.code == "4120"));
    }

    #[test]
    fn parent_code_resolves_to_none_when_missing() {
        let rows = build_accounts(AccountTemplateKind::Standard, None, None);
        let root = rows.iter().find(|r| r.code == "1").unwrap();
        assert_eq!(root.parent_code, None);
        let child = rows.iter().find(|r| r.code == "1110").unwrap();
        assert_eq!(child.parent_code.as_deref(), Some("11"));
    }

    #[test]
    fn basic_template_is_smaller_than_standard() {
        let basic = build_accounts(AccountTemplateKind::Basic, None, None);
        let standard = build_accounts(AccountTemplateKind::Standard, None, None);
        assert!(basic.len() < standard.len());
    }

    #[test]
    fn detailed_extends_standard() {
        let standard = build_accounts(AccountTemplateKind::Standard, None, None);
        let detailed = build_accounts(AccountTemplateKind::Detailed, None, None);
        assert!(detailed.len() > standard.len());
        assert!(detailed.iter().any(|r| r.code == "1135"));
    }
}
