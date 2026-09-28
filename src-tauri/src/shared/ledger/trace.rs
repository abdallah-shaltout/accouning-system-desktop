//! `shared::ledger::trace` (C-7, P2-35): the posting-trace ring, ported from
//! `src/mocks/backend/posting-trace.ts`. Only OBSERVES what `resolve_posting`/`post` already
//! computed — it never decides an account, amount or rounding. Kept in memory only
//! (`AppState.traces`, a 500-entry ring): never written to the DB, never part of a backup.

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Serialize;

use crate::utils::id::Id;

use super::post::{party_kind_str, ResolvedLine};

pub const TRACE_RING_CAPACITY: usize = 500;

/// Per-line annotations a caller can attach to its `PostingLine` before calling
/// `resolve_posting`/`post` (mirrors `core.ts`'s `PostingLine.trace`). Every field is optional and
/// purely observational — a caller that never sets `trace` posts exactly as it always has.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineTrace {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_discount: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_discount: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vat: Option<VatTrace>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostTrace>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fx: Option<FxTrace>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VatTrace {
    pub base: Decimal,
    pub rate: Decimal,
    pub tax: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rounding_delta: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostTrace {
    pub qty_before: Decimal,
    pub avg_before: Decimal,
    pub qty_after: Decimal,
    pub avg_after: Decimal,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxTrace {
    pub rate: Decimal,
    pub amount_fc: Decimal,
    pub base: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rounding_delta: Option<Decimal>,
}

/// `PostingTraceStep` (`posting-trace.ts:21-25`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostingTraceStep {
    pub kind: &'static str,
    pub label: String,
    pub detail: serde_json::Value,
}

/// A resolved journal line, exactly as recorded into a trace's `lines` (mirrors what the mock
/// stores as `JournalLine` on `PostingTrace.lines`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceLine {
    pub account_id: Id,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub debit: Decimal,
    pub credit: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party_kind: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party_id: Option<Id>,
    pub branch_id: Id,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_center_id: Option<Id>,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_fc: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<Decimal>,
}

/// `PostingTrace` (`posting-trace.ts:27-36`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostingTrace {
    pub doc_type: String,
    pub doc_id: Id,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<Id>,
    pub steps: Vec<PostingTraceStep>,
    pub lines: Vec<TraceLine>,
    pub totals: Totals,
    pub balanced: bool,
    #[serde(with = "crate::utils::dates::iso_ms")]
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Totals {
    pub dr: Decimal,
    pub cr: Decimal,
}

impl PostingTrace {
    /// Builds a trace from a posting's resolved lines, right after `resolve_posting` runs (mirrors
    /// `recordPostingTrace`, `posting-trace.ts:98-140`) — reads what was already computed, changes
    /// nothing. One `accountResolution` step per line, plus whatever the caller annotated via each
    /// line's `PostingLine.trace`.
    pub fn build(doc_type: String, doc_id: Id, lines: &[ResolvedLine], total_debit: Decimal, total_credit: Decimal, at: DateTime<Utc>) -> Self {
        let mut steps = Vec::new();
        let mut trace_lines = Vec::with_capacity(lines.len());

        for line in lines {
            if let Some(t) = &line.input.trace {
                if let Some(d) = &t.line_discount {
                    steps.push(PostingTraceStep {
                        kind: "lineDiscount",
                        label: format!("خصم على السطر{}", describe_line(line)),
                        detail: d.clone(),
                    });
                }
                if let Some(d) = &t.invoice_discount {
                    steps.push(PostingTraceStep { kind: "invoiceDiscount", label: "توزيع خصم الفاتورة".to_string(), detail: d.clone() });
                }
                if let Some(vat) = &t.vat {
                    steps.push(PostingTraceStep {
                        kind: "vat",
                        label: "ضريبة القيمة المضافة".to_string(),
                        detail: serde_json::to_value(vat).unwrap_or(serde_json::Value::Null),
                    });
                }
                if let Some(cost) = &t.cost {
                    steps.push(PostingTraceStep {
                        kind: "cost",
                        label: "تكلفة (متوسط مرجح)".to_string(),
                        detail: serde_json::to_value(cost).unwrap_or(serde_json::Value::Null),
                    });
                }
                if let Some(fx) = &t.fx {
                    steps.push(PostingTraceStep {
                        kind: "fx",
                        label: "تحويل عملة".to_string(),
                        detail: serde_json::to_value(fx).unwrap_or(serde_json::Value::Null),
                    });
                }
                if let Some(note) = &t.note {
                    steps.push(PostingTraceStep { kind: "note", label: note.clone(), detail: serde_json::Value::Object(Default::default()) });
                }
            }

            // Account resolution: always knowable at this choke point (`posting-trace.ts:82-92`).
            let (why, role_field) = match &line.input.account {
                super::post::AccountRef::Role(role) => {
                    let mut why = format!("role=\"{}\"", role.as_str());
                    if line.input.branch_id.is_some() {
                        why.push_str(&format!(" branchId=\"{}\"", line.branch_id));
                    }
                    if let Some(cur) = &line.input.currency {
                        why.push_str(&format!(" currency=\"{cur}\""));
                    }
                    (why, Some(role.as_str()))
                }
                super::post::AccountRef::Id(_) => ("accountId مباشر من المستدعي (قيد يدوي أو حساب مختار)".to_string(), None),
            };
            let mut detail = serde_json::Map::new();
            if let Some(role) = role_field {
                detail.insert("role".to_string(), serde_json::Value::String(role.to_string()));
            }
            detail.insert("chosenAccountId".to_string(), serde_json::Value::String(line.account_id.to_string()));
            detail.insert("why".to_string(), serde_json::Value::String(why));
            steps.push(PostingTraceStep { kind: "accountResolution", label: "اختيار الحساب".to_string(), detail: serde_json::Value::Object(detail) });

            trace_lines.push(TraceLine {
                account_id: line.account_id,
                description: line.description.clone(),
                debit: line.debit,
                credit: line.credit,
                party_kind: line.party.map(|p| party_kind_str(p.kind)),
                party_id: line.party.map(|p| p.id),
                branch_id: line.branch_id,
                cost_center_id: line.cost_center_id,
                currency: line.currency.clone(),
                amount_fc: line.amount_fc,
                rate: line.rate,
            });
        }

        let balanced = (total_debit - total_credit).abs() <= rust_decimal_macros::dec!(0.001);

        PostingTrace {
            doc_type,
            doc_id,
            correlation_id: None,
            steps,
            lines: trace_lines,
            totals: Totals { dr: total_debit, cr: total_credit },
            balanced,
            at,
        }
    }

    /// Attaches a correlation id after construction (the caller may not have one yet at `build`
    /// time — mirrors the mock's optional `opts.correlationId`).
    pub fn with_correlation_id(mut self, id: Id) -> Self {
        self.correlation_id = Some(id);
        self
    }
}

fn describe_line(line: &ResolvedLine) -> String {
    match &line.description {
        Some(d) if !d.is_empty() => format!(" ({d})"),
        _ => String::new(),
    }
}

/// The 500-entry in-memory ring (`AppState.traces`), plus a debug-mode index by entry id (always
/// on server-side — there is no `localStorage` debug-flag gate here the way the mock has one;
/// callers that want the "only if accounting debug is on" behavior gate their own read, since
/// keeping every entry indexed costs nothing extra here — the ring itself already bounds memory).
/// Interior-mutability, `Send + Sync`, so it can be shared as `Arc<TraceRing>` across async tasks.
pub struct TraceRing {
    capacity: usize,
    inner: Mutex<TraceRingInner>,
}

#[derive(Default)]
struct TraceRingInner {
    ring: std::collections::VecDeque<PostingTrace>,
    by_entry_id: std::collections::HashMap<Id, PostingTrace>,
}

impl TraceRing {
    pub fn new(capacity: usize) -> Self {
        Self { capacity, inner: Mutex::new(TraceRingInner::default()) }
    }

    /// Pushes every trace from one committed transaction. Evicts the oldest entries once over
    /// capacity (mirrors `posting-trace.ts`'s `ring.splice(0, ring.length - RING_SIZE)`).
    pub fn push_all(&self, traces: Vec<PostingTrace>) {
        let mut inner = self.inner.lock().unwrap();
        for trace in traces {
            inner.by_entry_id.insert(trace.doc_id, trace.clone());
            inner.ring.push_back(trace);
        }
        // Evict from the id index together with the ring, so both stay bounded by `capacity`
        // (an index that only grows would leak for the lifetime of the app).
        while inner.ring.len() > self.capacity {
            if let Some(evicted) = inner.ring.pop_front() {
                let still_in_ring = inner.ring.iter().any(|t| t.doc_id == evicted.doc_id);
                if !still_in_ring {
                    inner.by_entry_id.remove(&evicted.doc_id);
                }
            }
        }
    }

    /// The most recent `n` traces, newest first.
    pub fn recent(&self, n: usize) -> Vec<PostingTrace> {
        let inner = self.inner.lock().unwrap();
        inner.ring.iter().rev().take(n).cloned().collect()
    }

    /// The trace recorded for a specific journal entry id, if it's still in the ring/index.
    pub fn for_entry(&self, entry_id: Id) -> Option<PostingTrace> {
        self.inner.lock().unwrap().by_entry_id.get(&entry_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sample_trace(id: Id) -> PostingTrace {
        PostingTrace {
            doc_type: "journal".to_string(),
            doc_id: id,
            correlation_id: None,
            steps: vec![],
            lines: vec![],
            totals: Totals { dr: dec!(10), cr: dec!(10) },
            balanced: true,
            at: Utc::now(),
        }
    }

    #[test]
    fn ring_evicts_oldest_past_capacity() {
        let ring = TraceRing::new(2);
        let a = Id::new();
        let b = Id::new();
        let c = Id::new();
        ring.push_all(vec![sample_trace(a)]);
        ring.push_all(vec![sample_trace(b)]);
        ring.push_all(vec![sample_trace(c)]);
        let recent = ring.recent(10);
        assert_eq!(recent.len(), 2);
        assert!(ring.for_entry(a).is_none());
        assert!(ring.for_entry(b).is_some());
        assert!(ring.for_entry(c).is_some());
    }

    #[test]
    fn for_entry_finds_a_specific_trace() {
        let ring = TraceRing::new(TRACE_RING_CAPACITY);
        let id = Id::new();
        ring.push_all(vec![sample_trace(id)]);
        assert!(ring.for_entry(id).is_some());
        assert!(ring.for_entry(Id::new()).is_none());
    }
}
