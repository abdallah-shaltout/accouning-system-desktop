//! `progress.rs` (02-setup.md §3.4, `setupService.ts:67-88,231-234`): the `settings.onboarding`
//! read/patch/step-tracking commands. All four wizard writers here take
//! `core::settings::load_shared_locked` first (§3 "Common") — serialises wizard steps.

use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};

use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::org::settings::{ActiveModel as SettingsActiveModel, Entity as SettingsEntity};
use crate::entities::values::OnboardingState;

use super::super::dto::{OnboardingProgress, OnboardingProgressPatch};
use super::session::clear_bootstrap_session;

/// `OnboardingState` has no `Default` impl (it's a foreign entity-value type, `entities::values`) —
/// this is the empty state `?? []`-style defaults (`setupService.ts:64`) resolve to.
fn empty_onboarding_state() -> OnboardingState {
    OnboardingState {
        business_type: None,
        go_live_date: None,
        completed_step: None,
        skipped: Vec::new(),
        done: Vec::new(),
        finished_at: None,
        opening_entry_id: None,
        closing_entry_id: None,
        coa_template: None,
    }
}

fn to_dto(state: Option<&OnboardingState>) -> OnboardingProgress {
    let state = state.cloned().unwrap_or_else(empty_onboarding_state);
    OnboardingProgress {
        business_type: state.business_type,
        go_live_date: state.go_live_date.map(|d| d.format("%Y-%m-%d").to_string()),
        completed_step: state.completed_step,
        skipped: state.skipped,
        done: state.done,
        finished_at: state.finished_at.map(crate::utils::dates::format_iso_ms),
        opening_entry_id: state.opening_entry_id,
        closing_entry_id: state.closing_entry_id,
        coa_template: state.coa_template,
    }
}

/// `getOnboardingProgress`'s DTO half (`setupService.ts:61-65`) — the shell-seeding + bootstrap
/// session steps live in `commands.rs` (they need `AppState`, not just a connection).
pub async fn get<C: ConnectionTrait>(conn: &C) -> TxResult<OnboardingProgress> {
    let settings = SettingsEntity::find().one(conn).await.map_err(TxError::from)?;
    // The mock projects exactly six keys (`setupService.ts:62-67`); the opening/closing entry ids
    // and the COA template stay in `settings.onboarding`, not in this read.
    let mut dto = to_dto(settings.as_ref().and_then(|s| s.onboarding.as_ref()));
    dto.opening_entry_id = None;
    dto.closing_entry_id = None;
    dto.coa_template = None;
    Ok(dto)
}

/// `saveOnboardingProgress` (`:67-72`): shallow-merges present keys into `onboarding`.
pub async fn save<C: ConnectionTrait>(conn: &C, cx: &TxCtx, patch: OnboardingProgressPatch) -> TxResult<()> {
    let locked = crate::core::settings::load_shared_locked(conn).await.map_err(TxError::App)?;
    let mut state = locked.onboarding.clone().unwrap_or_else(empty_onboarding_state);

    if let Some(v) = patch.business_type {
        state.business_type = Some(v);
    }
    if let Some(v) = patch.go_live_date {
        state.go_live_date = chrono::NaiveDate::parse_from_str(&v, "%Y-%m-%d").ok();
    }
    if let Some(v) = patch.completed_step {
        state.completed_step = Some(v);
    }
    if let Some(v) = patch.skipped {
        state.skipped = v;
    }
    if let Some(v) = patch.done {
        state.done = v;
    }
    if let Some(v) = patch.finished_at {
        state.finished_at = chrono::DateTime::parse_from_rfc3339(&v).ok().map(|d| d.with_timezone(&chrono::Utc));
    }
    if let Some(v) = patch.opening_entry_id {
        state.opening_entry_id = Some(v);
    }
    if let Some(v) = patch.closing_entry_id {
        state.closing_entry_id = Some(v);
    }
    if let Some(v) = patch.coa_template {
        state.coa_template = Some(v);
    }

    let mut model: SettingsActiveModel = locked.into();
    model.onboarding = Set(Some(state));
    model.updated_at = Set(cx.clock.now);
    model.update(conn).await.map_err(TxError::from)?;
    Ok(())
}

async fn mutate_onboarding<C: ConnectionTrait>(conn: &C, cx: &TxCtx, f: impl FnOnce(&mut OnboardingState)) -> TxResult<()> {
    let locked = crate::core::settings::load_shared_locked(conn).await.map_err(TxError::App)?;
    let mut state = locked.onboarding.clone().unwrap_or_else(empty_onboarding_state);
    f(&mut state);
    let mut model: SettingsActiveModel = locked.into();
    model.onboarding = Set(Some(state));
    model.updated_at = Set(cx.clock.now);
    model.update(conn).await.map_err(TxError::from)?;
    Ok(())
}

/// `markStepDone` (`:74-80`): appends `key` to `done` if absent (Set order); optionally bumps
/// `completedStep`.
pub async fn mark_step_done<C: ConnectionTrait>(conn: &C, cx: &TxCtx, key: String, step_index: Option<i32>) -> TxResult<()> {
    mutate_onboarding(conn, cx, |state| {
        if !state.done.contains(&key) {
            state.done.push(key);
        }
        if let Some(idx) = step_index {
            state.completed_step = Some(idx);
        }
    })
    .await
}

/// `markStepSkipped` (`:82-88`): same de-dup-set pattern for `skipped`.
pub async fn mark_step_skipped<C: ConnectionTrait>(conn: &C, cx: &TxCtx, key: String) -> TxResult<()> {
    mutate_onboarding(conn, cx, |state| {
        if !state.skipped.contains(&key) {
            state.skipped.push(key);
        }
    })
    .await
}

/// `finishOnboarding` (`:231-234`): stamps `finished_at`; the caller (`commands.rs`) clears the
/// bootstrap session after this commits.
pub async fn finish<C: ConnectionTrait>(conn: &C, cx: &TxCtx) -> TxResult<()> {
    mutate_onboarding(conn, cx, |state| {
        state.finished_at = Some(cx.clock.now);
    })
    .await
}

/// Called by the command layer after `finish`'s transaction commits (D-1) — takes `&AppState`
/// directly since clearing the session is a process-local side effect, not a DB write.
pub fn clear_bootstrap_session_after_finish(state: &crate::core::state::AppState) {
    clear_bootstrap_session(state);
}
