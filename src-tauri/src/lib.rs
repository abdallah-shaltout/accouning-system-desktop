// 21.02-A: module layout after the moves. Always refer to the first as `crate::core::…` and
// never write `use core::…` in this file (P2-37) — that would collide with the builtin `core`
// crate. `domains` doesn't exist yet (Part 03 creates it).
pub mod core;
pub mod domains;
pub mod entities;
pub mod infrastructure;
pub mod shared;
pub mod utils;

/// The installer ships a pinned WebView2 runtime next to the exe (`webviewInstallMode: fixedRuntime`,
/// see scripts/fetch-webview2.js), so the UI renders the same on every PC. When that folder isn't
/// there — `tauri dev` (target/debug) or a damaged install — fall back to the system WebView2
/// instead of failing to open the window.
fn context() -> tauri::Context<tauri::Wry> {
    #[allow(unused_mut)]
    let mut context = tauri::generate_context!();
    #[cfg(windows)]
    {
        use tauri::utils::config::WebviewInstallMode;
        let install_mode = &mut context.config_mut().bundle.windows.webview_install_mode;
        if let WebviewInstallMode::FixedRuntime { path } = install_mode {
            let present = std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|dir| dir.join(&*path).join("msedgewebview2.exe")))
                .is_some_and(|exe| exe.exists());
            if !present {
                *install_mode = WebviewInstallMode::default();
            }
        }
    }
    context
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::Manager as _;

    // Guards against a second `RunEvent::ExitRequested` re-entering the shutdown-and-exit branch
    // below while the first one's async shutdown is still in flight.
    let exit_in_progress = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    tauri::Builder::default()
        // Guarantees only one app process — and thus only one MariaDB supervisor — ever runs
        // against the machine's data directory at a time (P2-58). Must be the *first* plugin
        // registered so a second launch is caught before anything else (window creation, DB boot)
        // starts.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        // A3-3: registered unconditionally (cheap no-op plugin when never enabled) — the Rust
        // `ManagerExt::autolaunch()` handle is toggled only by
        // `infrastructure::database::lan::enable_lan_sharing`/`disable_lan_sharing`, never anywhere
        // else. No JS permission is added for this (`--background` is a plain process arg, not an
        // IPC-visible capability).
        .plugin({
            #[cfg(windows)]
            {
                infrastructure::database::hosting::autostart_plugin()
            }
            #[cfg(not(windows))]
            {
                tauri_plugin_autostart::Builder::new().build()
            }
        })
        .setup(|app| {
            let handle = app.handle().clone();
            core::state::boot(handle.clone());

            // A3-3: show the main window immediately unless this launch is the OS starting the app
            // at logon in the background (`--background`, passed only by the autostart entry this
            // phase registers) — no flash of the window before it hides itself again on a
            // LAN-sharing PC.
            #[cfg(windows)]
            infrastructure::database::hosting::show_main_window_unless_background(&handle);
            #[cfg(not(windows))]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                }
            }

            // A3-3: if LAN sharing is already on at launch (the common case — the owner enabled it
            // on a previous run), install the tray icon + hide-on-close behavior once the managed
            // server reaches `Running`. If LAN sharing gets turned on *during* this run instead,
            // `infrastructure::database::lan::enable_lan_sharing` installs it itself right after a
            // successful enable (`install_tray_and_continuity` is idempotent — see its doc comment
            // — so this boot-time watcher and that call can never install it twice).
            #[cfg(windows)]
            {
                let handle_for_tray = handle.clone();
                tauri::async_runtime::spawn(async move {
                    use tauri::Manager as _;
                    for _ in 0..240 {
                        // up to ~60s: provisioning + first start can legitimately take a while
                        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                        let state = handle_for_tray.state::<core::state::AppState>();
                        let is_running = state
                            .server
                            .state
                            .try_read()
                            .map(|s| s.phase == infrastructure::database::supervisor::ServerPhase::Running)
                            .unwrap_or(false);
                        if !is_running {
                            continue;
                        }
                        // A2-7: block Windows shutdown/sign-out until the managed server has
                        // stopped cleanly — started once, only after the server is really running.
                        infrastructure::database::session_end::start(std::sync::Arc::clone(&state.server));
                        let lan_sharing = infrastructure::database::state_file::load(&state.server.paths.server_json())
                            .ok()
                            .flatten()
                            .map(|s| s.lan_sharing)
                            .unwrap_or(false);
                        if lan_sharing {
                            let server = std::sync::Arc::clone(&state.server);
                            let _ = infrastructure::database::hosting::install_tray_and_continuity(&handle_for_tray, server);
                        }
                        break;
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(invoke_handler())
        .build(context())
        .expect("error while building tauri application")
        .run(move |app_handle, event| {
            let tauri::RunEvent::ExitRequested { api, .. } = event else { return };

            #[cfg(windows)]
            let managed_and_running = {
                use infrastructure::database::supervisor::ServerPhase;
                let state = app_handle.state::<core::state::AppState>();
                state.server.state.try_read().map(|s| s.phase == ServerPhase::Running).unwrap_or(false)
            };
            #[cfg(not(windows))]
            let managed_and_running = false;

            if !managed_and_running {
                return;
            }
            if exit_in_progress.swap(true, std::sync::atomic::Ordering::SeqCst) {
                return; // a shutdown is already in flight from a previous exit request
            }

            api.prevent_exit();
            let app_handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                #[cfg(windows)]
                {
                    let state = app_handle.state::<core::state::AppState>();
                    let server = std::sync::Arc::clone(&state.server);
                    server.shutdown(std::time::Duration::from_secs(60)).await;
                }
                app_handle.exit(0);
            });
        });
}

/// The app's one command list (plan 21 Part 04, P4-3/B-1): `run()` registers it on the real `Wry`
/// runtime and `src/bin/parity_host.rs` registers the same list on Tauri's `MockRuntime`, so parity
/// runs exactly the code path the app runs. Generic over the runtime, so every listed command must
/// be too (the few that need an `AppHandle` take `AppHandle<R>`). `core::ipc`'s
/// `ipc_manifest_matches_handler` still reads this list from this file's text.
pub fn invoke_handler<R: tauri::Runtime>() -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        infrastructure::pdf::render::render_pdf,
        infrastructure::pdf::render::render_preview,
        infrastructure::print::commands::list_printers,
        infrastructure::print::commands::print_thermal_receipt,
        infrastructure::print::commands::print_test_receipt,
        core::diag::diag_append,
        core::diag::diag_read,
        core::diag::diag_clear,
        core::diag::diag_open_folder,
        core::diag::diag_rotate,
        core::status::core_backend_status,
        infrastructure::import::commands::setup_inspect_legacy_snapshot,
        infrastructure::import::commands::setup_import_snapshot,
        infrastructure::import::commands::setup_wipe_business_data,
        domains::settings::commands::settings_get_settings,
        domains::settings::commands::settings_update_settings,
        domains::settings::commands::settings_get_taxes,
        domains::settings::commands::settings_save_tax,
        domains::settings::commands::settings_delete_tax,
        domains::settings::commands::settings_get_payment_methods,
        domains::settings::commands::settings_save_payment_method,
        domains::settings::commands::settings_reorder_payment_methods,
        domains::settings::commands::settings_delete_payment_method,
        domains::settings::commands::settings_get_branches,
        domains::settings::commands::settings_create_branch,
        domains::settings::commands::settings_update_branch,
        domains::settings::commands::settings_deactivate_branch,
        domains::settings::commands::settings_reactivate_branch,
        domains::settings::commands::settings_get_cost_centers,
        domains::settings::commands::settings_create_cost_center,
        domains::settings::commands::settings_update_cost_center,
        domains::settings::commands::settings_delete_cost_center,
        domains::settings::commands::settings_get_currencies,
        domains::settings::commands::settings_get_exchange_rates,
        domains::settings::commands::settings_create_currency,
        domains::settings::commands::settings_update_currency,
        domains::settings::commands::settings_save_exchange_rate,
        domains::settings::commands::settings_is_base_currency_locked,
        domains::settings::commands::settings_set_base_currency,
        domains::settings::commands::settings_get_revaluation_preview,
        domains::settings::commands::settings_get_default_revaluation_rates,
        domains::settings::commands::settings_post_revaluation,
        domains::settings::commands::settings_get_lan_sharing_status,
        domains::settings::commands::settings_enable_lan_sharing,
        domains::settings::commands::settings_disable_lan_sharing,
        domains::settings::commands::settings_rotate_pairing_code,
        domains::settings::commands::settings_reconnect_backend,
        domains::users::commands::users_get_users,
        domains::users::commands::users_get_user,
        domains::users::commands::users_create_user,
        domains::users::commands::users_update_user,
        domains::users::commands::users_login,
        domains::users::commands::users_logout,
        domains::users::commands::users_restore_session,
        domains::users::commands::users_verify_manager_pin,
        domains::templates::commands::templates_list_templates,
        domains::templates::commands::templates_get_template,
        domains::templates::commands::templates_get_default_template,
        domains::templates::commands::templates_save_template,
        domains::templates::commands::templates_set_as_default,
        domains::templates::commands::templates_duplicate_template,
        domains::templates::commands::templates_delete_template,
        domains::templates::commands::templates_reset_template_to_defaults,
        domains::templates::commands::templates_import_template,
        domains::templates::commands::templates_create_template,
        domains::diagnostics::commands::diagnostics_get_audit_entries,
        domains::diagnostics::commands::diagnostics_get_audit_entities,
        domains::diagnostics::commands::diagnostics_export_support_bundle,
        domains::approvals::commands::approvals_submit_approval_request,
        domains::approvals::commands::approvals_get_approval_requests,
        domains::approvals::commands::approvals_get_pending_approval_count,
        domains::approvals::commands::approvals_approve_request,
        domains::approvals::commands::approvals_reject_request,
        domains::expenses::commands::expenses_get_expense_categories,
        domains::expenses::commands::expenses_save_expense_category,
        domains::expenses::commands::expenses_delete_expense_category,
        domains::expenses::commands::expenses_get_expenses,
        domains::expenses::commands::expenses_get_expense,
        domains::expenses::commands::expenses_create_expense,
        domains::expenses::commands::expenses_get_recurring_expenses,
        domains::expenses::commands::expenses_save_recurring_expense,
        domains::expenses::commands::expenses_delete_recurring_expense,
        domains::expenses::commands::expenses_get_due_recurring_expenses,
        domains::expenses::commands::expenses_post_due_recurring_expense,
        domains::parties::commands::parties_check_duplicates,
        domains::parties::commands::parties_get_party_groups,
        domains::parties::commands::parties_get_customers,
        domains::parties::commands::parties_get_customer,
        domains::parties::commands::parties_save_customer,
        domains::parties::commands::parties_get_customer_statement,
        domains::parties::commands::parties_get_suppliers,
        domains::parties::commands::parties_get_supplier,
        domains::parties::commands::parties_save_supplier,
        domains::parties::commands::parties_get_supplier_statement,
        domains::parties::commands::parties_link_party_records,
        domains::parties::commands::parties_unlink_party_record,
        domains::parties::commands::parties_get_linked_net_balance,
        domains::parties::commands::parties_get_party_history,
        domains::parties::commands::parties_get_party_aging,
        domains::setup::commands::setup_get_device_setup_state,
        domains::setup::commands::setup_provision_main,
        domains::setup::commands::setup_pair_terminal,
        domains::setup::commands::setup_get_onboarding_progress,
        domains::setup::commands::setup_save_onboarding_progress,
        domains::setup::commands::setup_mark_step_done,
        domains::setup::commands::setup_mark_step_skipped,
        domains::setup::commands::setup_apply_business_type_defaults,
        domains::setup::commands::setup_is_base_currency_locked,
        domains::setup::commands::setup_apply_country_tax,
        domains::setup::commands::setup_apply_fiscal_year,
        domains::setup::commands::setup_apply_branches,
        domains::setup::commands::setup_apply_coa_template,
        domains::setup::commands::setup_apply_payment_methods,
        domains::setup::commands::setup_get_opening_balance_equity_net,
        domains::setup::commands::setup_is_first_use_posted,
        domains::setup::commands::setup_post_opening_balances,
        domains::setup::commands::setup_post_opening_stock,
        domains::setup::commands::setup_reclose_opening_balance_equity,
        domains::setup::commands::setup_post_party_opening,
        domains::setup::commands::setup_reverse_party_opening,
        domains::setup::commands::setup_finish_onboarding,
        infrastructure::backup::commands::settings_backup_settings,
        infrastructure::backup::commands::settings_save_backup_settings,
        infrastructure::backup::commands::settings_preview_backup_counts,
        infrastructure::backup::commands::settings_build_backup_archive,
        infrastructure::backup::commands::settings_record_backup_saved,
        infrastructure::backup::commands::settings_run_auto_backup_if_due,
        infrastructure::backup::commands::settings_preview_restore,
        infrastructure::backup::commands::settings_restore_from_archive,
        domains::products::commands::catalog::products_get_products,
        domains::products::commands::catalog::products_get_product,
        domains::products::commands::catalog::products_find_by_code,
        domains::products::commands::catalog::products_generate_ean13,
        domains::products::commands::catalog::products_create_product,
        domains::products::commands::catalog::products_update_product,
        domains::products::commands::catalog::products_suggest_sku,
        domains::products::commands::catalog::products_get_categories,
        domains::products::commands::catalog::products_save_category,
        domains::products::commands::catalog::products_delete_category,
        domains::products::commands::catalog::products_get_units,
        domains::products::commands::catalog::products_save_unit,
        domains::products::commands::catalog::products_apply_unit_preset,
        domains::products::commands::catalog::products_delete_unit,
        domains::products::commands::catalog::products_get_price_lists,
        domains::products::commands::catalog::products_save_price_list,
        domains::products::commands::catalog::products_delete_price_list,
        domains::products::commands::catalog::products_set_price_list_values,
        domains::products::commands::catalog::products_get_custom_field_defs,
        domains::products::commands::catalog::products_save_custom_field_def,
        domains::products::commands::catalog::products_delete_custom_field_def,
        domains::products::commands::catalog::products_reorder_custom_field_defs,
        domains::products::commands::inventory::products_get_stock_adjustments,
        domains::products::commands::inventory::products_get_stock_adjustment,
        domains::products::commands::inventory::products_create_stock_adjustment,
        domains::products::commands::inventory::products_complete_adjustment,
        domains::products::commands::inventory::products_delete_draft_adjustment,
        domains::products::commands::inventory::products_get_stock_movements,
        domains::products::commands::inventory::products_get_stock_movements_paged,
        domains::products::commands::inventory::products_get_batches,
        domains::products::commands::inventory::products_get_expiry_report,
        domains::products::commands::inventory::products_write_off_expired_batches,
        domains::products::commands::inventory::products_return_batches_to_supplier,
        domains::products::commands::inventory::products_get_debit_note_drafts,
        domains::products::commands::inventory::products_get_stock_counts,
        domains::products::commands::inventory::products_get_stock_count,
        domains::products::commands::inventory::products_create_stock_count,
        domains::products::commands::inventory::products_update_stock_count_line,
        domains::products::commands::inventory::products_submit_count_for_review,
        domains::products::commands::inventory::products_resume_counting,
        domains::products::commands::inventory::products_complete_stock_count,
        domains::products::commands::transfers::products_get_transfers,
        domains::products::commands::transfers::products_get_transfer,
        domains::products::commands::transfers::products_create_transfer,
        domains::products::commands::transfers::products_send_transfer,
        domains::products::commands::transfers::products_receive_transfer,
        domains::products::commands::transfers::products_reject_transfer,
        domains::vouchers::commands::vouchers_create_receipt_voucher,
        domains::vouchers::commands::vouchers_create_payment_voucher,
        domains::vouchers::commands::vouchers_create_transfer_voucher,
        domains::vouchers::commands::vouchers_create_owner_voucher,
        domains::vouchers::commands::vouchers_get_vouchers,
        domains::vouchers::commands::vouchers_get_voucher,
        domains::vouchers::commands::vouchers_get_unsettled_tender_groups,
        domains::vouchers::commands::vouchers_estimate_settlement_fee,
        domains::vouchers::commands::vouchers_create_card_settlement,
        domains::vouchers::commands::vouchers_get_card_settlements,
        domains::vouchers::commands::vouchers_get_card_settlement,
        domains::purchases::commands::purchases_get_purchase_orders,
        domains::purchases::commands::purchases_get_purchase_order,
        domains::purchases::commands::purchases_get_purchase_return,
        domains::purchases::commands::purchases_get_active_batches,
        domains::purchases::commands::purchases_get_debit_note_drafts,
        domains::purchases::commands::purchases_save_purchase_order,
        domains::purchases::commands::purchases_send_purchase_order_to_supplier,
        domains::purchases::commands::purchases_receive_purchase_order,
        domains::purchases::commands::purchases_confirm_purchase_order,
        domains::purchases::commands::purchases_cancel_purchase_order,
        domains::purchases::commands::purchases_create_purchase_return,
        domains::purchases::commands::purchases_post_debit_note_draft,
        domains::payments::commands::payments_get_payments,
        domains::payments::commands::payments_get_payments_paged,
        domains::payments::commands::payments_get_payment,
        domains::payments::commands::payments_create_payment,
        domains::payments::commands::payments_allocate_existing_payment,
        domains::payments::commands::payments_remove_allocation,
        domains::payments::commands::payments_get_open_documents,
        domains::invoices::commands::invoices_get_invoices,
        domains::invoices::commands::invoices_get_invoices_paged,
        domains::invoices::commands::invoices_get_invoice,
        domains::invoices::commands::invoices_preview_sale,
        domains::invoices::commands::invoices_create_sale,
        domains::invoices::commands::invoices_create_refund,
        domains::invoices::commands::invoices_get_refund,
        domains::invoices::commands::invoices_get_invoice_print_data,
        domains::invoices::commands::invoices_get_quotations,
        domains::invoices::commands::invoices_get_quotation,
        domains::invoices::commands::invoices_save_quotation,
        domains::invoices::commands::invoices_set_quotation_status,
        domains::invoices::commands::invoices_convert_quotation_to_invoice,
        domains::invoices::commands::invoices_get_current_shift,
        domains::invoices::commands::invoices_get_shifts,
        domains::invoices::commands::invoices_get_shift,
        domains::invoices::commands::invoices_open_pos_shift,
        domains::invoices::commands::invoices_get_x_report,
        domains::invoices::commands::invoices_close_pos_shift,
        domains::invoices::commands::invoices_force_close_pos_shift,
        domains::invoices::commands::invoices_record_cash_in_out,
        domains::invoices::commands::invoices_get_held_sales,
        domains::invoices::commands::invoices_hold_sale,
        domains::invoices::commands::invoices_resume_held_sale,
        domains::invoices::commands::invoices_discard_held_sale,
        domains::accounting::commands::accounts::accounting_get_accounts,
        domains::accounting::commands::accounts::accounting_save_account,
        domains::accounting::commands::accounts::accounting_delete_account,
        domains::accounting::commands::accounts::accounting_reparent_account,
        domains::accounting::commands::journal::accounting_get_journal_entries,
        domains::accounting::commands::journal::accounting_get_journal_entries_for_source,
        domains::accounting::commands::journal::accounting_get_journal_entries_paged,
        domains::accounting::commands::journal::accounting_get_journal_entry,
        domains::accounting::commands::journal::accounting_create_journal_entry,
        domains::accounting::commands::journal::accounting_update_journal_draft,
        domains::accounting::commands::journal::accounting_post_journal_draft,
        domains::accounting::commands::journal::accounting_delete_journal_draft,
        domains::accounting::commands::journal::accounting_reverse_journal_entry,
        domains::accounting::commands::period::accounting_get_fiscal_years,
        domains::accounting::commands::period::accounting_get_current_fiscal_year,
        domains::accounting::commands::period::accounting_save_fiscal_year,
        domains::accounting::commands::period::accounting_get_lock_date,
        domains::accounting::commands::period::accounting_save_lock_date,
        domains::accounting::commands::period::accounting_get_close_year_pre_checks,
        domains::accounting::commands::period::accounting_close_year,
        domains::accounting::commands::period::accounting_reopen_year,
        domains::accounting::commands::period::accounting_get_vat_period_totals,
        domains::accounting::commands::period::accounting_submit_vat_settlement,
        domains::accounting::commands::period::accounting_pay_vat_settlement_now,
        domains::accounting::commands::templates::accounting_get_journal_templates,
        domains::accounting::commands::templates::accounting_get_journal_template,
        domains::accounting::commands::templates::accounting_create_or_update_journal_template,
        domains::accounting::commands::templates::accounting_remove_journal_template,
        domains::accounting::commands::templates::accounting_load_template_into_entry,
        domains::accounting::commands::templates::accounting_post_recurring_template,
        domains::diagnostics::commands::diagnostics_list_recent_documents,
        domains::diagnostics::commands::diagnostics_get_posting_trace,
        domains::diagnostics::commands::diagnostics_get_journal_entry_raw,
        domains::diagnostics::commands::diagnostics_get_balances_around,
        domains::diagnostics::commands::diagnostics_get_invariant_results,
        domains::diagnostics::commands::diagnostics_get_drift_report,
        domains::diagnostics::commands::diagnostics_explain_account_balance,
        domains::reports::commands::reports_get_trial_balance,
        domains::reports::commands::reports_get_profit_and_loss,
        domains::reports::commands::reports_get_profit_and_loss_comparison,
        domains::reports::commands::reports_get_cost_center_profit_and_loss,
        domains::reports::commands::reports_get_cost_center_budget_vs_actual,
        domains::reports::commands::reports_get_balance_sheet,
        domains::reports::commands::reports_get_account_ledger,
        domains::reports::commands::reports_get_party_ledger,
        domains::reports::commands::reports_get_vat_report,
        domains::reports::commands::reports_get_vat_detail,
        domains::reports::commands::reports_get_cash_flow_statement,
        domains::reports::commands::reports_get_day_book,
        domains::reports::commands::reports_get_period_comparison,
        domains::reports::commands::reports_get_business_health_report,
        domains::reports::commands::reports_get_ledger_targets,
        domains::reports::commands::reports_get_dimension_options,
        domains::reports::commands::reports_get_sales_report,
        domains::reports::commands::reports_get_inventory_report,
        domains::reports::commands::reports_get_discounts_report,
        domains::reports::commands::reports_get_gross_profit_report,
        domains::reports::commands::reports_get_returns_report,
        domains::reports::commands::reports_get_expenses_report,
        domains::reports::commands::reports_get_shifts_report,
        domains::reports::commands::reports_get_low_stock_report,
        domains::reports::commands::reports_get_dead_stock_report,
        domains::reports::commands::reports_get_stocktake_variances,
        domains::reports::commands::reports_get_transfers_report,
        domains::reports::commands::reports_get_purchases_report,
        domains::reports::commands::reports_get_branch_comparison,
        domains::reports::commands::reports_get_profit_leakage_report,
        domains::reports::commands::reports_get_aging_report,
        domains::reports::commands::reports_get_overdue_report,
        domains::analytics::commands::analytics_get_sales_analytics,
        domains::analytics::commands::analytics_get_product_analytics,
        domains::analytics::commands::analytics_get_customer_analytics,
        domains::dashboard::commands::dashboard_get_dashboard_summary,
        domains::dashboard::commands::dashboard_get_home_kpis,
        domains::dashboard::commands::dashboard_get_low_stock_products,
        domains::dashboard::commands::dashboard_get_recent_invoices,
        domains::dashboard::commands::dashboard_get_recent_activity,
        domains::dashboard::commands::dashboard_get_top_products,
        domains::dashboard::commands::dashboard_get_top_customers,
        domains::dashboard::commands::dashboard_get_in_transit_transfers,
        domains::dashboard::commands::dashboard_get_pending_approval_requests,
        domains::dashboard::commands::dashboard_get_last_backup_failed_at,
        domains::dashboard::commands::dashboard_get_journal_draft_count,
        domains::dashboard::commands::dashboard_get_stock_value_snapshot,
        domains::dashboard::commands::dashboard_has_any_products,
        domains::dashboard::commands::dashboard_compute_insights,
        domains::dashboard::commands::dashboard_get_product_inline_hints,
        domains::attachments::commands::attachments_fetch_attachments,
        domains::attachments::commands::attachments_fetch_attachment,
        domains::attachments::commands::attachments_fetch_attachments_by_ids,
        domains::attachments::commands::attachments_save_attachment,
        domains::attachments::commands::attachments_remove_attachment
    ]
}

/// The device/LAN commands (`settings_*_lan_sharing`, `settings_reconnect_backend`, `setup_*` device
/// steps) drive the bundled server, the tray and autostart, which exist only on the real `Wry`
/// runtime. They take `AppHandle<R>` so the shared list above compiles for any runtime, and get
/// the `Wry` handle back here; any other runtime (the parity host) gets `FORBIDDEN`.
pub(crate) fn wry_handle<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<&tauri::AppHandle<tauri::Wry>, crate::core::error::AppError> {
    (app as &dyn std::any::Any)
        .downcast_ref::<tauri::AppHandle<tauri::Wry>>()
        .ok_or_else(|| crate::core::error::AppError::forbidden("هذه الميزة غير متاحة في هذا الوضع"))
}
