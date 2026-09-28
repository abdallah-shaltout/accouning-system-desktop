//! Manager-PIN approval grants (plan 21 Part 03, G-P3). When `users_verify_manager_pin` succeeds,
//! the approving manager's id is granted for a short window; a command that needs a manager's
//! approval (e.g. an inventory adjustment above the approval threshold, 06b S-4) re-checks the grant
//! server-side instead of trusting the `approvedBy` the frontend sends. Per terminal process, in
//! memory only: a grant never outlives the app or crosses to another terminal.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::utils::id::Id;

/// How long a verified manager PIN authorises approvals on this terminal.
pub const GRANT_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Default)]
pub struct ApprovalGrants {
    granted_at: Mutex<HashMap<Id, Instant>>,
}

impl ApprovalGrants {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records (or refreshes) a grant for `approver` — called only after the PIN verified.
    pub fn grant(&self, approver: Id) {
        let mut map = self.granted_at.lock().unwrap();
        map.retain(|_, at| at.elapsed() < GRANT_TTL);
        map.insert(approver, Instant::now());
    }

    /// True while `approver`'s last verified PIN is younger than `GRANT_TTL`.
    pub fn is_granted(&self, approver: Id) -> bool {
        self.granted_at.lock().unwrap().get(&approver).is_some_and(|at| at.elapsed() < GRANT_TTL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grant_then_is_granted_and_unknown_is_not() {
        let grants = ApprovalGrants::new();
        let a = Id::new();
        assert!(!grants.is_granted(a));
        grants.grant(a);
        assert!(grants.is_granted(a));
        assert!(!grants.is_granted(Id::new()));
    }
}
