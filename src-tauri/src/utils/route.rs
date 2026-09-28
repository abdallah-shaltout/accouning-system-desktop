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
    /// G-13/PG-5: query-string params (e.g. `{ name: 'payments', query: { highlight } }`,
    /// `payments.ts:303,365,385`) — kept serde/TS-compatible with the hand-written `AppRoute`
    /// (`core/types/route.ts`), which is itself `RouteLocationAsRelative`-shaped and so already
    /// allows an optional `query` map.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<BTreeMap<String, String>>,
}

impl RouteRef {
    /// A list route with no params (e.g. `{ name: 'approvals' }`).
    pub fn list(name: impl Into<String>) -> Self {
        Self { name: name.into(), params: None, query: None }
    }

    /// A detail route with a single `id` param (the overwhelmingly common shape).
    pub fn detail(name: impl Into<String>, id: impl Into<String>) -> Self {
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), id.into());
        Self { name: name.into(), params: Some(params), query: None }
    }

    /// G-13/PG-5: attaches a query-string map to an existing route ref (e.g. a list route plus a
    /// `highlight` param: `RouteRef::list("payments").with_query("highlight", id)`).
    pub fn with_query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.get_or_insert_with(BTreeMap::new).insert(key.into(), value.into());
        self
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

    #[test]
    fn with_query_adds_query_map_and_omits_when_absent() {
        let r = RouteRef::list("payments").with_query("highlight", "pay-9");
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, r#"{"name":"payments","query":{"highlight":"pay-9"}}"#);

        let no_query = RouteRef::list("approvals");
        let json = serde_json::to_string(&no_query).unwrap();
        assert!(!json.contains("query"), "query must be omitted entirely when absent, not null");
    }
}
