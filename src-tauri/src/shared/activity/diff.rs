//! `diff_fields` — a behaviour-exact port of `diffFields` (`core.ts:443-455`): a shallow
//! field-by-field diff of two "plain objects" for `AuditEntry.before`/`after`.
//!
//! The mock takes two `Record<string, unknown>` and iterates `new Set([...Object.keys(before),
//! ...Object.keys(after)])` — insertion order: `before`'s keys first, then any of `after`'s keys
//! not already seen. This port takes **ordered slices** instead of maps so no global serde_json
//! `preserve_order` feature is needed; the caller supplies the fields in the same "before's keys,
//! then after's new keys" order the mock would have produced from its own object literals.
//!
//! `None` means the JS `undefined` a field simply not being present in the object — distinct from
//! `Some(Value::Null)`, which is an explicit JSON `null`. Equal JSON (by value, including the
//! `None`/`None` case) is skipped; the output entry omits `before`/`after` when that side is `None`.

use serde_json::Value;

/// One changed field, matching `AuditFieldDiff` (`{ field, before?, after? }`).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct FieldDiff {
    pub field: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<Value>,
}

/// Compares `before`/`after` field-by-field, in the order the caller lists them (before's keys,
/// then after's new keys — the mock's `Set` insertion order). Returns the two parallel diff lists
/// `logAudit`'s `before`/`after` expect.
pub fn diff_fields(before: &[(&str, Option<Value>)], after: &[(&str, Option<Value>)]) -> (Vec<FieldDiff>, Vec<FieldDiff>) {
    let mut order: Vec<&str> = Vec::with_capacity(before.len() + after.len());
    for (field, _) in before {
        if !order.contains(field) {
            order.push(field);
        }
    }
    for (field, _) in after {
        if !order.contains(field) {
            order.push(field);
        }
    }

    let mut before_out = Vec::new();
    let mut after_out = Vec::new();

    for field in order {
        let b = before.iter().find(|(f, _)| *f == field).map(|(_, v)| v.clone()).unwrap_or(None);
        let a = after.iter().find(|(f, _)| *f == field).map(|(_, v)| v.clone()).unwrap_or(None);
        if b == a {
            continue;
        }
        before_out.push(FieldDiff { field: field.to_string(), before: b, after: None });
        after_out.push(FieldDiff { field: field.to_string(), before: None, after: a });
    }

    (before_out, after_out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unchanged_fields_are_skipped() {
        let before = [("name", Some(json!("a"))), ("qty", Some(json!(1)))];
        let after = [("name", Some(json!("a"))), ("qty", Some(json!(2)))];
        let (b, a) = diff_fields(&before, &after);
        assert_eq!(b, vec![FieldDiff { field: "qty".into(), before: Some(json!(1)), after: None }]);
        assert_eq!(a, vec![FieldDiff { field: "qty".into(), before: None, after: Some(json!(2)) }]);
    }

    #[test]
    fn key_order_is_before_keys_then_afters_new_keys() {
        let before = [("b", Some(json!(1))), ("a", Some(json!(1)))];
        let after = [("a", Some(json!(2))), ("c", Some(json!(3)))];
        let (b, _a) = diff_fields(&before, &after);
        let fields: Vec<&str> = b.iter().map(|d| d.field.as_str()).collect();
        // "b" changes (1 -> undefined), "a" changes (1 -> 2); "c" is after-only (undefined -> 3).
        assert_eq!(fields, vec!["b", "a", "c"]);
    }

    #[test]
    fn undefined_differs_from_explicit_null() {
        let before = [("x", None)];
        let after = [("x", Some(Value::Null))];
        let (b, a) = diff_fields(&before, &after);
        assert_eq!(b.len(), 1, "undefined -> null must be reported as a change");
        assert_eq!(b[0].before, None);
        assert_eq!(a[0].after, Some(Value::Null));
    }

    #[test]
    fn both_undefined_is_not_a_change() {
        let before = [("x", None)];
        let after: [(&str, Option<Value>); 0] = [];
        let (b, a) = diff_fields(&before, &after);
        assert!(b.is_empty());
        assert!(a.is_empty());
    }

    #[test]
    fn equal_json_values_are_skipped_even_when_object_shaped() {
        let before = [("meta", Some(json!({"a": 1, "b": [1,2,3]})))];
        let after = [("meta", Some(json!({"a": 1, "b": [1,2,3]})))];
        let (b, a) = diff_fields(&before, &after);
        assert!(b.is_empty());
        assert!(a.is_empty());
    }

    #[test]
    fn output_omits_the_missing_side() {
        let before: [(&str, Option<Value>); 0] = [];
        let after = [("new_field", Some(json!("x")))];
        let (b, a) = diff_fields(&before, &after);
        assert_eq!(b, vec![FieldDiff { field: "new_field".into(), before: None, after: None }]);
        assert_eq!(a, vec![FieldDiff { field: "new_field".into(), before: None, after: Some(json!("x")) }]);
        // Serialized shape omits the None side entirely.
        let json_b = serde_json::to_value(&b[0]).unwrap();
        assert_eq!(json_b, json!({"field": "new_field"}));
    }
}
