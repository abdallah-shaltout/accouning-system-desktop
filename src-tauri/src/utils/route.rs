//! `RouteRef`: the wire shape of `AppRoute` (`core/types/route.ts`) — CLAUDE.md rule 25, "every
//! navigation target is a named route object." Every service link field (`actionTo`, `sourceLink`,
//! `refLink`, `link`) is typed as this on the Rust side.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteRef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<BTreeMap<String, String>>,
}

impl RouteRef {
    /// A list route with no params (e.g. `{ name: 'approvals' }`).
    pub fn list(name: impl Into<String>) -> Self {
        Self { name: name.into(), params: None }
    }

    /// A detail route with a single `id` param (the overwhelmingly common shape).
    pub fn detail(name: impl Into<String>, id: impl Into<String>) -> Self {
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), id.into());
        Self { name: name.into(), params: Some(params) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_omits_params() {
        let r = RouteRef::list("approvals");
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, r#"{"name":"approvals"}"#);
    }

    #[test]
    fn detail_includes_id_param() {
        let r = RouteRef::detail("invoice", "inv-12");
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, r#"{"name":"invoice","params":{"id":"inv-12"}}"#);
    }
}
